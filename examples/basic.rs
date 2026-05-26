use anyhow::Result;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use forge::api::agents::{AgentBuilder, prompt_stream, prompt_with_tools};
use forge::api::dtos::Role::USER;
use forge::api::dtos::{ImageUrl, Message, MultiContent};
use forge::api::request::console_log;
use forge::api::tools_registry::{Tool, ToolRegistry};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let agent = AgentBuilder::new()
        .url("http://127.0.0.1:1234/v1")
        .model("google/gemma-4-e4b")
        .tool_registry(Arc::new({
            let mut registry = ToolRegistry::new();
            registry.register(MultiplyTool);
            registry
        }))
        .build()?;

    let a = 1234;
    let b = 5678;
    let prompt = format!("Whats {a} * {b}?, Use your tools!!!");

    // create message
    let message = vec![Message {
        role: USER,
        content: Some(prompt.clone()),
        multi_content: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }];

    let response = prompt_with_tools(&agent, message, 8).await?;
    println!("Response: {}", response);

    let prompt = "Who is this pokemon? Answer correctly".to_string();
    let message = vec![Message {
        role: USER,
        content: None,
        multi_content: Some(vec![
            MultiContent {
                r#type: "text".to_string(),
                text: Some(prompt),
                image_url: None,
            },
            MultiContent {
                r#type: "image".to_string(),
                text: None,
                image_url: Some(ImageUrl {
                    url: format!(
                        "data:image/jpg;base64,{}",
                        BASE64_STANDARD
                            .encode(&tokio::fs::read(Path::new("examples/palkia.jpg")).await?)
                    ),
                }),
            },
        ]),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }];

    let response = prompt_stream(&agent, message).await?;

    // nicely handles streaming to terminal
    console_log(22, response).await?;

    Ok(())
}

struct MultiplyTool;

#[async_trait::async_trait]
impl Tool for MultiplyTool {
    fn name(&self) -> &str {
        "multiply_tool"
    }

    // must same exact pattern for tooling
    fn description(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": "Multiplies two numbers together",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "a": {
                            "type": "integer",
                            "description": "First number",
                        },
                        "b": {
                            "type": "integer",
                            "description": "Second number",
                        }
                    },
                    "required": ["a", "b"]
                }
            }
        })
    }

    fn tool_callback(&self) -> bool {
        true
    }

    async fn execute_tool(&self, args: Value) -> Result<String> {
        println!("Received args for multiply_tool: {}", args);
        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'a'"))?;
        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'b'"))?;
        Ok((a * b).to_string())
    }
}
