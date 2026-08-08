use anyhow::Result;
use forge::Value;
use forge::agents::Agent;
use forge::tools_registry::{Tool, ToolRegistry};

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

struct NumGrinder;

#[async_trait::async_trait]
impl Tool for NumGrinder {
    fn name(&self) -> &str {
        "num_grinder"
    }

    fn description(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": "Grind two numbers together",
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
        // `grind` two number
        let a_bytes = a.to_le_bytes();
        let b_bytes = b.to_le_bytes();

        let mut grind = [0u8; 8];
        for i in 0..a_bytes.len() {
            grind[i] = a_bytes[i] ^ b_bytes[i];
        }
        Ok(f64::from_le_bytes(grind).to_string())
    }
}
