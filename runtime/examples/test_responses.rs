use dotenvy::dotenv;
use std::env;

use anyhow::{Context, Result};
use runtime::ai::responses::ResponsesClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string_pretty};

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub name: String,
    pub date: String,
    pub participants: Vec<String>,
}

pub fn calendar_event_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": { "type": "string" },
            "date": { "type": "string" },
            "participants": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["name", "date", "participants"]
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().expect(".env file not found");

    let api_key = env::var("OPENAI_API_KEY")
        .context("OPENAI_API_KEY environment variable must be set to run this binary")?;
    let base = env::var("OPENAI_BASE_URL").ok();
    let model = env::var("OPENAI_RESPONSES_MODEL").unwrap_or_else(|_| "gpt-5-mini".to_string());

    let system = env::var("RESPONSES_SYSTEM_PROMPT").unwrap_or_else(|_| {
        "You are an assistant that extracts calendar events from user messages.".to_string()
    });

    let user_prompt = match env::args().nth(1) {
        Some(arg) => arg,
        None => {
            "Lunch with Casey next Wednesday at 12:30pm, include Taylor and send them the invite."
                .to_string()
        }
    };

    let client = ResponsesClient::new(api_key, base);
    let schema = calendar_event_schema();

    let event: CalendarEvent = client
        .responses_structured(
            &model,
            &system,
            &user_prompt,
            "calendar_event",
            schema,
            true,
        )
        .await
        .context("Responses API request failed")?;

    let pretty = to_string_pretty(&event)?;
    println!("{}", pretty);

    Ok(())
}
