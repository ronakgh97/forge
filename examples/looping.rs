use anyhow::Result;
use forge::agents::Agent;
use forge::tools_registry::{Tool, ToolRegistry};
use serde_json::Value;
use std::io;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

static BALANCE: LazyLock<Arc<Mutex<f64>>> = LazyLock::new(|| Arc::new(Mutex::new(5_000.0)));

#[tokio::main]
async fn main() -> Result<()> {
    let mut tool_registry = ToolRegistry::init();
    tool_registry.register(GamblingBox);

    let mut agent = Agent::init(
        "google/gemma-4-e4b".to_string(),
        "http://localhost:1234/v1".to_string(),
        "local".to_string(),
        "You are a relentless gambler.".to_string(),
        0.68,
        Some(tool_registry),
    );

    let initial_balance = *BALANCE.lock().await;

    print!("Enter target amount: ");
    io::Write::flush(&mut io::stdout())?;
    let mut target_to_hit = String::new();
    io::stdin().read_line(&mut target_to_hit)?;
    if target_to_hit.trim().is_empty() {
        eprintln!("Target amount cannot be empty");
        std::process::exit(1);
    }
    target_to_hit.trim().to_string();

    let user_message = format!(
        "Your goal is to reach a balance of {target_to_hit} starting from {initial_balance}. \
    You can bet any amount of your current balance. \
    You are NOT ALLOWED to get bankrupted, you NEED to make some amount of profit before quiting. \
    If, its seems absolutely impossible to reach the target, then only return back to user. Good luck!"
    );

    let (response, _) = agent.prompt_with_tools_loop(&user_message).await?;
    println!("Response text: {}", response);

    Ok(())
}

struct GamblingBox;

#[async_trait::async_trait]
impl Tool for GamblingBox {
    fn name(&self) -> &str {
        "gambling_box"
    }

    fn description(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": "A mysterious toolbox that allows you to gamble with your money.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "bet": {
                            "type": "float",
                            "description": "Money amount to bet in",
                        }
                    },
                    "required": ["bet"]
                }
            }
        })
    }

    async fn execute_tool(&self, args: Value) -> Result<String> {
        let bet_amount = args
            .get("bet")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'bet'"))?;

        if bet_amount < 0.0 {
            return Err(anyhow::anyhow!("Bet must be greater than 0"));
        }

        let balance_available = *BALANCE.lock().await;
        if bet_amount > balance_available {
            return Err(anyhow::anyhow!("Insufficient balance"));
        }

        let multiple = [-1.0, 1.5, -2.0, 2.5, -3.0, 3.5];
        let rand_choice = rand::random_range(0..multiple.len());
        let result = bet_amount * multiple[rand_choice];
        *BALANCE.lock().await += result;

        println!(
            "Available balance: {:.2}, Result: {:.2}, Bet: {:.2}",
            *BALANCE.lock().await,
            result,
            bet_amount
        );

        Ok(format!(
            "Available balance: {:.2}, Result: {:.2}",
            *BALANCE.lock().await,
            result
        ))
    }
}
