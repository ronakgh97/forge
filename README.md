# forge

A tiny async library for building AI agent loops (OpenAI/OpenRouter compatible).

```
AI -> tools -> memory -> loop
```

> NOTE: Experimental, use for local only.

## Usage

```rust
use forge::api::agents::AgentBuilder;
use forge::api::tools_registry::{Tool, ToolRegistry};
use serde_json::Value;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut agent = AgentBuilder::new()
        .url("http://127.0.0.1:1234/v1")
        .model("google/gemma-4-e4b")
        .tool_registry(Arc::new({
            let mut reg = ToolRegistry::new();
            reg.register(MyTool);
            reg
        }))
        .build()?;

    // Simple prompt
    let response = agent.prompt("Hello!").await?;

    // Tool loop — agent calls tools, feeds results back, repeats until done
    let response = agent.prompt_with_tools("What is 2+2? Use your tools.", 8).await?;

    // Conversation history is automatic
    println!("{} messages in history", agent.history().len());
    agent.clear_history();

    Ok(())
}
```

## Implementing a Tool

```rust
struct MyTool;

#[async_trait::async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }

    fn description(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": "Does something useful",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "input": { "type": "string", "description": "The input" }
                    },
                    "required": ["input"]
                }
            }
        })
    }

    fn tool_callback(&self) -> bool { true } // true = continue loop, false = return result immediately

    async fn execute_tool(&self, args: serde_json::Value) -> anyhow::Result<String> {
        let input = args["input"].as_str().unwrap();
        Ok(format!("Processed: {input}"))
    }
}
```
