use anyhow::Result;
use forge::Value;
use forge::agents::Agent;
use forge::tools_registry::{Tool, ToolRegistry};

#[tokio::main]
async fn main() -> Result<()> {
    let mut tool_registry = ToolRegistry::init();
    tool_registry.register(NumBlender);

    let mut agent = Agent::init(
        "google/gemma-4-e4b".to_string(),
        "http://localhost:1234/v1".to_string(),
        "local".to_string(),
        "You are a helpful assistant.".to_string(),
        0.68,
        Some(tool_registry),
    );

    let a = 1234;
    let b = 5678;
    let prompt = format!("Blend them {a}, {b}");

    let response = agent.prompt_with_tools(&prompt).await?;
    println!("Response: {}", response);
    println!("Message count: {}", agent.get_history().len());
    println!("History: {:?}", agent.get_history());
    Ok(())
}

struct NumBlender;

#[async_trait::async_trait]
impl Tool for NumBlender {
    fn name(&self) -> &str {
        "blend_tool"
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
        true // will loop
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
