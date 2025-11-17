use anyhow::Context;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::time::{Duration, sleep, timeout};
use tracing::{debug, info, warn};

pub struct ResponsesClient {
    http: Client,
    api_key: String,
    base: String,
}

impl ResponsesClient {
    pub fn new(api_key: String, base: Option<String>) -> Self {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(600))
            .build()
            .expect("client");
        Self {
            http,
            api_key,
            base: base.unwrap_or_else(|| "https://api.openai.com".into()),
        }
    }

    fn extract_structured_output<T: DeserializeOwned>(root: &Value) -> Option<T> {
        if let Some(candidate) = root.get("output_parsed") {
            if let Some(parsed) = Self::parse_candidate::<T>(candidate) {
                return Some(parsed);
            }
        }

        if let Some(candidate) = root.get("output_text") {
            if let Some(parsed) = Self::parse_candidate::<T>(candidate) {
                return Some(parsed);
            }
        }

        if let Some(output) = root.get("output") {
            if let Value::Array(items) = output {
                for item in items {
                    if let Some(parsed) = item
                        .get("parsed")
                        .and_then(|v| Self::parse_candidate::<T>(v))
                    {
                        return Some(parsed);
                    }

                    if let Some(parsed) =
                        item.get("text").and_then(|v| Self::parse_candidate::<T>(v))
                    {
                        return Some(parsed);
                    }

                    if let Some(content) = item.get("content") {
                        if let Value::Array(blocks) = content {
                            for block in blocks {
                                if let Some(parsed) = block
                                    .get("parsed")
                                    .and_then(|v| Self::parse_candidate::<T>(v))
                                {
                                    return Some(parsed);
                                }

                                if let Some(parsed) = block
                                    .get("text")
                                    .and_then(|v| Self::parse_candidate::<T>(v))
                                {
                                    return Some(parsed);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn parse_candidate<T: DeserializeOwned>(value: &Value) -> Option<T> {
        match value {
            Value::String(s) => serde_json::from_str::<T>(s)
                .or_else(|_| serde_json::from_value(Value::String(s.clone())))
                .ok(),
            Value::Array(items) => items
                .iter()
                .find_map(|item| Self::parse_candidate::<T>(item)),
            _ => serde_json::from_value(value.clone()).ok(),
        }
    }

    async fn poll_oai_response(&self, raw_response: Value, path: &str) -> anyhow::Result<Value> {
        let id = raw_response
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("missing response id"))?;
        let overall_timeout = Duration::from_secs(300000);
        let req_timeout = Duration::from_secs(150000);
        let mut delay = Duration::from_secs(2);

        timeout(overall_timeout, async {
            loop {
                let url = format!("{}/v1{}/{id}", self.base, path);
                match timeout(
                    req_timeout,
                    self.http.get(url).bearer_auth(&self.api_key).send(),
                )
                .await
                {
                    Ok(Ok(res)) if res.status().is_success() => {
                        let payload: Value = res
                            .json()
                            .await
                            .with_context(|| format!("error parsing OpenAI response {id}"))?;
                        match payload.get("status").and_then(Value::as_str) {
                            Some("completed") => return Ok(payload),
                            Some(status @ ("failed" | "cancelled")) => {
                                let detail = payload
                                    .pointer("/error/message")
                                    .or_else(|| payload.pointer("/last_error/message"))
                                    .and_then(Value::as_str)
                                    .unwrap_or("no detail provided");
                                return Err(anyhow::anyhow!("OpenAI background responses | status={status} | detail={detail}, response_id={id}"));
                            }
                            _ => debug!(response_id = id, "background job still running"),
                        }
                    }
                    Ok(Ok(res)) => {
                        let status = res.status();
                        let body = res.text().await.unwrap_or_default();
                        if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                            warn!(response_id=id, %status, "transient poll failure; retrying");
                        } else {
                            return Err(anyhow::anyhow!("OpenAI poll returned {}: {}", status, body));
                        }
                    }
                    Ok(Err(err)) => {
                        warn!(
                            response_id = id,
                            error = %err,
                            "network error polling; retrying"
                        );
                    }
                    Err(_) => {
                        warn!(response_id = id, "per-request timeout; retrying");
                    }
                }

                sleep(delay + Duration::from_millis(fastrand::u64(0..500))).await;
                delay = (delay * 2).min(Duration::from_secs(20));
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("polling OpenAI response {id} timed out"))?
    }

    async fn post_json(&self, path: &str, body: &Value) -> reqwest::Result<reqwest::Response> {
        self.http
            .post(format!("{}/v1{}", self.base, path))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
    }

    pub async fn responses_structured<T: DeserializeOwned + Default>(
        &self,
        model: &str,
        system: &str,
        user: &str,
        chunk_id: Option<&str>,
        schema_name: &str,
        schema: Value,
        strict: bool,
    ) -> anyhow::Result<T> {
        let response_format = json!({
            "type": "json_schema",
            "name": schema_name,
            "strict": strict,
            "schema": schema
        });

        let body = json!({
            "model": model,
            "input": [
                { "role": "system", "content": [{ "type": "input_text", "text": system }] },
                { "role": "user",   "content": [{ "type": "input_text", "text": user }] }
            ],
            "text": {"format": response_format},
            "reasoning": {"effort":"high"},
            "service_tier": "flex",
            "background": true,
        });

        let mut delay = Duration::from_millis(300);
        for attempt in 0..5 {
            let resp = self.post_json("/responses", &body).await?;
            if resp.status().is_success() {
                let v: Value = resp
                    .json()
                    .await
                    .with_context(|| "Error from OpenAI responses api")?;
                let v = self
                    .poll_oai_response(v, "/responses")
                    .await
                    .with_context(|| "Error polling OpenAI responses api")?;
                if let Some(parsed) = Self::extract_structured_output(&v) {
                    if let Some(id) = chunk_id {
                        info!(chunk_id = %id, "Extracted entity relations for");
                    }
                    return Ok(parsed);
                }
                let id = v
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing response id"))?;
                warn!(response_id=%id, "Structured output not found in response");
                return Ok(T::default());
            }

            if matches!(resp.status(), StatusCode::TOO_MANY_REQUESTS)
                || resp.status().is_server_error()
            {
                if attempt < 4 {
                    tokio::time::sleep(delay).await;
                    delay = Duration::from_millis((delay.as_millis() as f64 * 1.8) as u64)
                        + Duration::from_millis(fastrand::u64(0..250));
                    continue;
                }
            }

            let status = resp.status();
            let err_txt = resp
                .text()
                .await
                .with_context(|| "Error getting error text from OpenAI")
                .unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI error {}: {}", status, err_txt));
        }
        Err(anyhow::anyhow!("Retries exhausted"))
    }
}
