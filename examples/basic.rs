use anyhow::Result;
use forge::api::agents::{AgentBuilder, prompt_with_tools};
use forge::api::dtos::Message;
use forge::api::dtos::Role::USER;
use forge::api::tools_registry::{Tool, ToolRegistry};
use serde_json::Value;
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

    let message = vec![Message {
        role: USER,
        content: Some(prompt.clone()),
        multi_content: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }];

    let response = prompt_with_tools(&agent, message, 10).await?;
    println!("Response: {}", response);

    Ok(())
}

struct MultiplyTool;

#[async_trait::async_trait]
impl Tool for MultiplyTool {
    fn name(&self) -> &str {
        "multiply_tool"
    }

    fn description(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": "Multiplies two numbers together",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "first": {
                            "type": "integer",
                            "description": "First number",
                        },
                        "second": {
                            "type": "integer",
                            "description": "Second number",
                        }
                    },
                    "required": ["first", "second"]
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
            .get("first")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'a'"))?;
        let b = args
            .get("second")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'b'"))?;
        Ok((a * b).to_string())
    }
}
