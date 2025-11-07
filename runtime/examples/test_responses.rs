use std::env;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use dotenvy::dotenv;
use runtime::ai::agent::{AgentConfig, ReActAgent, Tool};
use runtime::ai::responses::ResponsesClient;
use runtime::ai::schemas::{CalendarEvent, calendar_event_schema};
use serde_json::to_string_pretty;

struct CalendarExtractionTool {
    client: Arc<ResponsesClient>,
    model: String,
    system_prompt: String,
}

impl CalendarExtractionTool {
    fn new(client: Arc<ResponsesClient>, model: String, system_prompt: String) -> Self {
        Self {
            client,
            model,
            system_prompt,
        }
    }
}

#[async_trait]
impl Tool for CalendarExtractionTool {
    fn name(&self) -> &str {
        "calendar_extractor"
    }

    fn description(&self) -> &str {
        "Extracts structured calendar events from natural language."
    }

    async fn invoke(&self, input: &str) -> Result<String> {
        let schema = calendar_event_schema();
        let event: CalendarEvent = self
            .client
            .responses_structured(
                &self.model,
                &self.system_prompt,
                input,
                None,
                "calendar_event",
                schema,
                true,
            )
            .await?;

        Ok(to_string_pretty(&event)?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().expect(".env file not found");

    let api_key = env::var("OPENAI_API_KEY")
        .context("OPENAI_API_KEY environment variable must be set to run this binary")?;
    let base = env::var("OPENAI_BASE_URL").ok();
    let model = env::var("OPENAI_RESPONSES_MODEL").unwrap_or_else(|_| "gpt-5-mini".to_string());

    let extraction_system = env::var("RESPONSES_SYSTEM_PROMPT").unwrap_or_else(|_| {
        "You are an assistant that extracts calendar events from user messages.".to_string()
    });

    let user_prompt = match env::args().nth(1) {
        Some(arg) => arg,
        None => {
            "Lunch with Casey next Wednesday at 12:30pm, include Taylor and send them the invite."
                .to_string()
        }
    };

    let client = Arc::new(ResponsesClient::new(api_key, base));

    let agent_config = AgentConfig {
        model: model.clone(),
        ..AgentConfig::default()
    };

    let agent = ReActAgent::builder(client.clone())
        .config(agent_config)
        .with_tool(CalendarExtractionTool::new(
            client.clone(),
            model.clone(),
            extraction_system,
        ))
        .build();

    let outcome = agent
        .run(&user_prompt)
        .await
        .context("ReAct agent failed to produce an answer")?;

    println!("Final answer: {}", outcome.final_answer);
    for (idx, step) in outcome.steps.iter().enumerate() {
        println!("Step {} thought: {}", idx + 1, step.thought);
        if let Some(action) = &step.action {
            let input = step.action_input.as_deref().unwrap_or("");
            println!("Step {} action: {} -> {}", idx + 1, action, input);
        }
        if let Some(observation) = &step.observation {
            println!("Step {} observation: {}", idx + 1, observation);
        }
        if let Some(answer) = &step.final_answer {
            println!("Step {} final answer: {}", idx + 1, answer);
        }
    }

    Ok(())
}
