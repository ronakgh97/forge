**forge** - A tiny async library for building AI agent loops (OpenAI/OpenRouter compatible).

> NOTE: Experimental, use for local only.

[Usages](./examples)

### Example

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let mut tool_registry = ToolRegistry::init();
    tool_registry.register(NumBlender);
    tool_registry.register(NumGrinder);

    let mut agent = Agent::init(
        "google/gemma-4-e4b".to_string(),
        "http://localhost:1234/v1".to_string(),
        "local".to_string(),
        "You are a helpful AI assistant.".to_string(),
        0.68,
        Some(tool_registry),
    );

    let a = 1234.19;
    let b = 5678.34;
    let prompt = format!("Blend and grind them {a}, {b}");

    let response = agent.prompt_with_tools_no_loop(&prompt).await?;
    println!("Response text: {}", response.0);
    println!(
        "Reasoning text: {}",
        response.1.unwrap_or("None".to_string())
    );
    println!("Tool call: {:?}", response.2.unwrap_or_else(Vec::new));
    println!("Message count: {}", agent.get_history().len());
    println!("History: {:?}", agent.get_history());
    Ok(())
}
```

### Tool Impl

```rust
struct NumBlender;

#[async_trait::async_trait]
impl Tool for NumBlender {
    fn name(&self) -> &str {
        "num_blender"
    }

    fn description(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": "Blend two numbers together",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "a": {
                            "type": "float",
                            "description": "First number",
                        },
                        "b": {
                            "type": "float",
                            "description": "Second number",
                        }
                    },
                    "required": ["a", "b"]
                }
            }
        })
    }

    async fn execute_tool(&self, args: Value) -> Result<String> {
        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'a'"))?;
        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'b'"))?;
        Ok(rand::random_range(a..b).to_string()) // 'blend' two numbers
    }
}
```