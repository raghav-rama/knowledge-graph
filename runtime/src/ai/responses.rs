use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::time::Duration;

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

    async fn post_json(&self, path: &str, body: &Value) -> reqwest::Result<reqwest::Response> {
        self.http
            .post(format!("{}/v1{}", self.base, path))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
    }

    pub async fn responses_structured<T: DeserializeOwned>(
        &self,
        model: &str,
        system: &str,
        user: &str,
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
            "reasoning": {"effort":"high"}
        });

        let mut delay = Duration::from_millis(300);
        for attempt in 0..5 {
            let resp = self.post_json("/responses", &body).await?;
            if resp.status().is_success() {
                let v: Value = resp.json().await?;
                if let Some(parsed) = Self::extract_structured_output(&v) {
                    return Ok(parsed);
                }
                anyhow::bail!("Structured output not found in response");
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
            let err_txt = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI error {}: {}", status, err_txt);
        }
        anyhow::bail!("Retries exhausted")
    }
}
