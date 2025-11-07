use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use super::responses::ResponsesClient;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn invoke(&self, input: &str) -> Result<String>;
}

#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub model: String,
    pub max_steps: usize,
    pub system_prompt: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "gpt-5-mini".to_string(),
            max_steps: 10,
            system_prompt: "You are a ReAct agent. Think carefully about when to use a tool versus when to answer directly. Always return JSON that matches the provided schema.".to_string(),
        }
    }
}

pub struct ReActAgent {
    client: Arc<ResponsesClient>,
    config: AgentConfig,
    tools: Vec<Arc<dyn Tool>>,
}

impl ReActAgent {
    pub fn new(
        client: Arc<ResponsesClient>,
        config: AgentConfig,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        Self {
            client,
            config,
            tools,
        }
    }

    pub fn builder(client: Arc<ResponsesClient>) -> ReActAgentBuilder {
        ReActAgentBuilder::new(client)
    }

    pub async fn run(&self, question: &str) -> Result<AgentOutcome> {
        let mut steps = Vec::new();
        let mut final_answer: Option<String> = None;

        for _ in 0..self.config.max_steps {
            let decision = self.plan_step(question, &steps).await?;
            match decision.decision_type {
                DecisionKind::Act => {
                    let tool_name = decision
                        .tool
                        .ok_or_else(|| anyhow!("Agent did not specify tool name"))?;
                    let tool_input = decision
                        .tool_input
                        .ok_or_else(|| anyhow!("Agent did not provide tool input"))?;
                    let observation = self.invoke_tool(&tool_name, &tool_input).await;

                    let step = AgentStep {
                        thought: decision.thought,
                        action: Some(tool_name),
                        action_input: Some(tool_input),
                        observation: Some(observation),
                        final_answer: None,
                    };
                    steps.push(step);
                }
                DecisionKind::Finish => {
                    let answer = decision
                        .final_answer
                        .ok_or_else(|| anyhow!("Agent did not provide a final answer"))?;
                    let step = AgentStep {
                        thought: decision.thought,
                        action: None,
                        action_input: None,
                        observation: None,
                        final_answer: Some(answer.clone()),
                    };
                    steps.push(step);
                    final_answer = Some(answer);
                    break;
                }
            }
        }

        if let Some(answer) = final_answer {
            Ok(AgentOutcome {
                final_answer: answer,
                steps,
            })
        } else {
            Err(anyhow!(
                "Max steps ({}) reached without a final answer",
                self.config.max_steps
            ))
        }
    }

    async fn invoke_tool(&self, tool_name: &str, tool_input: &str) -> String {
        if let Some(tool) = self.tools.iter().find(|tool| tool.name() == tool_name) {
            match tool.invoke(tool_input).await {
                Ok(output) => output,
                Err(err) => format!("Tool `{}` failed: {}", tool_name, err),
            }
        } else {
            format!("Tool `{}` is not available", tool_name)
        }
    }

    async fn plan_step(&self, question: &str, steps: &[AgentStep]) -> Result<AgentDecision> {
        let tool_names: Vec<String> = self
            .tools
            .iter()
            .map(|tool| tool.name().to_string())
            .collect();
        let schema = decision_schema(&tool_names);
        let prompt = build_user_prompt(question, steps, &self.tools);

        self.client
            .responses_structured(
                &self.config.model,
                &self.config.system_prompt,
                &prompt,
                None,
                "react_agent",
                schema,
                true,
            )
            .await
    }
}

pub struct ReActAgentBuilder {
    client: Arc<ResponsesClient>,
    config: AgentConfig,
    tools: Vec<Arc<dyn Tool>>,
}

impl ReActAgentBuilder {
    fn new(client: Arc<ResponsesClient>) -> Self {
        Self {
            client,
            config: AgentConfig::default(),
            tools: Vec::new(),
        }
    }

    pub fn config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_tool<T>(mut self, tool: T) -> Self
    where
        T: Tool + 'static,
    {
        self.tools.push(Arc::new(tool));
        self
    }

    pub fn with_tool_arc(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn build(self) -> ReActAgent {
        ReActAgent::new(self.client, self.config, self.tools)
    }
}

fn build_user_prompt(question: &str, steps: &[AgentStep], tools: &[Arc<dyn Tool>]) -> String {
    let mut sections = Vec::new();

    if tools.is_empty() {
        sections.push(
            "No external tools are available. You must answer directly once you are ready."
                .to_string(),
        );
    } else {
        let mut tool_lines = vec!["You can invoke the following tools: ".to_string()];
        for tool in tools {
            tool_lines.push(format!("- {}: {}", tool.name(), tool.description()));
        }
        sections.push(tool_lines.join("\n"));
    }

    if steps.is_empty() {
        sections.push("No previous steps have been taken.".to_string());
    } else {
        let mut history = vec!["Previous steps:".to_string()];
        for (idx, step) in steps.iter().enumerate() {
            history.push(format!("Step {} thought: {}", idx + 1, step.thought));
            if let Some(action) = &step.action {
                let input = step.action_input.as_deref().unwrap_or("");
                history.push(format!("Step {} action: {} -> {}", idx + 1, action, input));
            }
            if let Some(observation) = &step.observation {
                history.push(format!("Step {} observation: {}", idx + 1, observation));
            }
            if let Some(answer) = &step.final_answer {
                history.push(format!("Step {} final answer: {}", idx + 1, answer));
            }
        }
        sections.push(history.join("\n"));
    }

    sections.push(format!("Question: {}", question));
    sections.push("Respond with JSON that matches the schema. Use type=`act` to call a tool or type=`finish` to deliver the final_answer.".to_string());

    sections.join("\n\n")
}

#[derive(Debug, Clone)]
pub struct AgentStep {
    pub thought: String,
    pub action: Option<String>,
    pub action_input: Option<String>,
    pub observation: Option<String>,
    pub final_answer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentOutcome {
    pub final_answer: String,
    pub steps: Vec<AgentStep>,
}

#[derive(Debug, Deserialize, Default)]
struct AgentDecision {
    #[serde(rename = "type")]
    decision_type: DecisionKind,
    thought: String,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    tool_input: Option<String>,
    #[serde(default)]
    final_answer: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum DecisionKind {
    #[default]
    Act,
    Finish,
}

fn decision_schema(tool_names: &[String]) -> Value {
    if tool_names.is_empty() {
        return json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["type", "thought", "final_answer"],
            "properties": {
                "type": {"const": "finish"},
                "thought": {"type": "string"},
                "final_answer": {"type": "string"}
            }
        });
    }

    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "type": { "type": "string", "enum": ["act", "finish"] },
            "thought": { "type": "string" },
            "tool": { "type": "string", "enum": tool_names },
            "tool_input": { "type": "string" },
            "final_answer": { "type": "string" }
        },
        "required": ["type", "thought", "tool", "tool_input", "final_answer"],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_without_tools_allows_only_finish() {
        let schema = decision_schema(&[]);
        assert_eq!(schema["properties"]["type"]["const"], "finish");
    }

    #[test]
    fn schema_with_tools_includes_act() {
        let schema = decision_schema(&["search".to_string()]);
        let enum_values = schema["properties"]["type"]["enum"].as_array().unwrap();
        assert!(enum_values.iter().any(|v| v == "act"));
        assert!(enum_values.iter().any(|v| v == "finish"));
    }
}
