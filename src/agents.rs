use crate::dtos::Role::{Assistant, System, Tool, User};
use crate::dtos::{CompletionRequest, Message, ToolCall};
use crate::request::send_network_request;
use crate::tools_registry::ToolRegistry;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Represents an AI agent that can interact with a language model,
/// optionally using external tools during the conversation (internally manages conversation history).
#[derive(Serialize, Deserialize, Clone)]
pub struct Agent {
    pub model: String,
    pub url: String,
    pub api_key: String,
    pub system_prompt: String,
    pub temperature: f32,
    #[serde(skip)]
    pub(crate) tool_registry: Option<Arc<ToolRegistry>>,
    #[serde(skip)]
    pub(crate) client: Client,
    #[serde(skip)]
    pub(crate) history: Vec<Message>,
}

impl Default for Agent {
    /// Default agent configuration, with no tools registered.
    /// * Model: "google/gemma-4-e4b"
    /// * URL: "http://127.0.0.1:1234"
    /// * API Key: "local-key"
    /// * System Prompt: "You are AI assistant, try helping the user using your all capabilities."
    /// * Temperature: 0.0
    fn default() -> Self {
        Agent {
            model: "google/gemma-4-e4b".to_string(),
            url: "http://127.0.0.1:1234".to_string(),
            api_key: "local-key".to_string(),
            system_prompt:
                "You are AI assistant, try helping the user using your all capabilities."
                    .to_string(),
            temperature: 1.0,
            tool_registry: None,
            client: Default::default(),
            history: vec![],
        }
    }
}

impl Agent {
    /// Fresh init an agent, with all required parameters, and with no tools registered.
    pub fn init(
        model: String,
        url: String,
        api_key: String,
        system_prompt: String,
        temperature: f32,
    ) -> Self {
        Agent {
            model,
            url,
            api_key,
            system_prompt,
            temperature: temperature.clamp(0.0, 1.0),
            tool_registry: None,
            client: Default::default(),
            history: vec![],
        }
    }

    /// Load an agent from a JSON string, which contains the agent's configuration (model, url, api_key, system_prompt, temperature).
    pub fn from_json_str(json_str: &str) -> Result<Self> {
        let agent_builder = serde_json::from_str::<Agent>(json_str)?;
        Ok(agent_builder)
    }

    /// Serialize the agent's configuration (model, url, api_key, system_prompt, temperature) to a JSON string.
    pub fn to_json_str(&self) -> Result<String> {
        let json_str = serde_json::to_string_pretty(self)?;
        Ok(json_str)
    }

    /// Add a tool registry to the agent, which allows the agent to call external tools during the conversation.
    pub fn add_tools(&mut self, registry: ToolRegistry) {
        self.tool_registry = Some(Arc::new(registry));
    }

    /// Send a simple prompt with no tools, returns the model's text response.
    pub async fn prompt(&mut self, message: &str) -> Result<String> {
        self.history.push(user_message(message));

        let (response, _) = self.send_prompt().await?;

        self.history.push(Message {
            role: Assistant,
            content: Some(response.clone()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });

        Ok(response)
    }

    /// Run the infamous internal **Loop** `Context -> AI -> tools -> history -> loop (if callback)`,
    /// returns the final text model response.
    pub async fn prompt_with_tools(&mut self, message: &str, max_loop: usize) -> Result<String> {
        if self.tool_registry.is_none() {
            return Err(anyhow!("No tool registry"));
        }

        self.history.push(user_message(message));

        for i in 0..max_loop {
            let (response, calls) = self.send_prompt().await?;

            let calls = match calls {
                Some(c) if !c.is_empty() => c,
                _ => {
                    self.history.push(assistant_message(response.clone(), None));
                    return Ok(response);
                }
            };

            // Last iteration — can't send tool results back
            if i == max_loop - 1 {
                self.history.push(assistant_message(response, Some(calls)));
                return Err(anyhow!("Max iterations ({}) reached", max_loop));
            }

            self.history
                .push(assistant_message(response, Some(calls.clone())));

            for call in calls {
                let tool_name = &call.function.name;
                let should_callback = self
                    .tool_registry
                    .as_ref()
                    .unwrap() // Safe: checked above
                    .check_tool_callback(tool_name)?;

                let args: serde_json::Value = serde_json::from_str(&call.function.arguments)?;
                let result = self
                    .tool_registry
                    .as_ref()
                    .unwrap()
                    .execute(tool_name, args)
                    .await?;

                self.history.push(Message {
                    role: Tool,
                    content: Some(result.clone()),
                    tool_calls: None,
                    tool_call_id: Some(call.id),
                    name: Some(tool_name.clone()),
                });

                if !should_callback {
                    return Ok(result);
                }
            }
        }

        Err(anyhow!("Max iterations ({}) reached", max_loop))
    }

    /// Access conversation history.
    pub fn history(&self) -> &[Message] {
        &self.history
    }

    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Build the full message list with system prompt and send to the model.
    async fn send_prompt(&self) -> Result<(String, Option<Vec<ToolCall>>)> {
        let mut messages = Vec::with_capacity(1 + self.history.len());
        messages.push(Message {
            role: System,
            content: Some(self.system_prompt.clone()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });
        messages.extend_from_slice(&self.history);

        let request = CompletionRequest {
            model: self.model.clone(),
            messages,
            tools: self
                .tool_registry
                .as_ref()
                .map(|reg| reg.get_tool_definitions()),
            temperature: self.temperature,
            stream: Some(false),
        };

        let response = send_network_request(self, request).await?;
        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No choices in response"))?;

        Ok((
            choice.message.content.clone().unwrap_or_default(),
            choice.message.tool_calls.clone(),
        ))
    }
}

fn user_message(text: &str) -> Message {
    Message {
        role: User,
        content: Some(text.to_string()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }
}

fn assistant_message(content: String, tool_calls: Option<Vec<ToolCall>>) -> Message {
    Message {
        role: Assistant,
        content: Some(content),
        tool_calls,
        tool_call_id: None,
        name: None,
    }
}
