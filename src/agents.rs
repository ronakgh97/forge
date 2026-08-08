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
    /// * URL: "http://127.0.0.1:1234/v1"
    /// * API Key: "local-key"
    /// * System Prompt: "You are AI assistant, try helping the user using your all capabilities."
    /// * Temperature: 1.0
    fn default() -> Self {
        Agent {
            model: "google/gemma-4-e4b".to_string(),
            url: "http://127.0.0.1:1234/v1".to_string(),
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
    #[inline]
    /// Fresh init an agent, with all required parameters, and with no tools registered.
    pub fn init(
        model: String,
        url: String,
        api_key: String,
        system_prompt: String,
        temperature: f32,
        tool_registry: Option<ToolRegistry>,
    ) -> Self {
        Agent {
            model,
            url,
            api_key,
            system_prompt,
            temperature: temperature.clamp(0.0, 1.0),
            tool_registry: tool_registry.map(Arc::new),
            client: Default::default(),
            history: vec![],
        }
    }

    #[inline]
    /// Load an agent from a JSON string, which contains the agent's configuration (model, url, api_key, system_prompt, temperature).
    pub fn from_json_str(json_str: &str) -> Result<Self> {
        let agent_builder = serde_json::from_str::<Agent>(json_str)?;
        Ok(agent_builder)
    }

    #[inline]
    /// Serialize the agent's configuration (model, url, api_key, system_prompt, temperature) to a JSON string.
    pub fn to_json_str(&self) -> Result<String> {
        let json_str = serde_json::to_string_pretty(self)?;
        Ok(json_str)
    }

    #[inline]
    /// Get conversation history.
    pub fn get_history(&self) -> &[Message] {
        &self.history
    }

    #[inline]
    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Prompt the agent with a message, returns the agent's text response and optional reasoning, without using any tools.
    pub async fn prompt_with_no_tools(
        &mut self,
        message: &str,
    ) -> Result<(String, Option<String>)> {
        self.history.push(user_message(message));

        let (response, reasoning, _) = self.send_prompt(false).await?;

        self.history
            .push(assistant_message(response.clone(), reasoning.clone(), None));

        Ok((response, reasoning))
    }

    /// Prompt & execute tools once, returns the agent's text response and optional reasoning and optional tool_calls result from [`ToolRegistry::execute`].
    pub async fn prompt_with_tools_no_loop(
        &mut self,
        message: &str,
    ) -> Result<(String, Option<String>, Option<Vec<String>>)> {
        // Get all tools available
        let tools = match &self.tool_registry {
            Some(r) => r,
            None => return Err(anyhow!("No tools registered in the agent")),
        };

        // Add user message before anything
        self.history.push(user_message(message));

        // Make network request
        let (response, reasoning, calls) = self.send_prompt(true).await?;

        // Get tools if any
        let calls = match calls {
            Some(c) if !c.is_empty() => c,
            _ => {
                // Return the response and reasoning if no tools were called
                self.history
                    .push(assistant_message(response.clone(), reasoning.clone(), None));
                return Ok((response, reasoning, None));
            }
        };

        // Add the assistant message with tool calls to history
        self.history.push(assistant_message(
            response.clone(),
            reasoning.clone(),
            Some(calls.clone()),
        ));

        // Collect all tool responses, and add them to history as well
        let mut last_result = Vec::with_capacity(calls.len());

        for call in &calls {
            let tool_name = &call.function.name;
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments)?;
            let result = tools.execute(tool_name, args).await?;

            self.history.push(Message {
                role: Tool,
                content: Some(result.clone()),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: Some(call.id.clone()),
                name: Some(tool_name.clone()),
            });

            last_result.push(result);
        }

        Ok((response, reasoning, Some(last_result)))
    }

    /// Run the internal infamous **Loop** `Context -> AI -> Tool Calls -> History -> Loop`,
    /// returns the final agent's response text and optional reasoning text, when agent is done.
    pub async fn prompt_with_tools_loop(
        &mut self,
        message: &str,
    ) -> Result<(String, Option<String>)> {
        // Get tools
        let tools = match &self.tool_registry {
            Some(r) => r,
            None => return Err(anyhow!("No tools registered in the agent")),
        };

        self.history.push(user_message(message));

        // Loop until we are done, return final model `content` and `reasoning_content` text
        loop {
            // Call the agent
            let (response, reasoning, calls) = self.send_prompt(true).await?;

            let calls = match calls {
                Some(c) if !c.is_empty() => c,
                _ => {
                    // If no tools were called, return the response and reasoning
                    self.history
                        .push(assistant_message(response.clone(), reasoning.clone(), None));
                    return Ok((response, reasoning));
                }
            };

            // Add the assistant message with tool calls to history
            self.history
                .push(assistant_message(response, reasoning, Some(calls.clone())));

            // Iter and exec all tools until we are done
            for call in &calls {
                let tool_name = &call.function.name;
                let args: serde_json::Value = serde_json::from_str(&call.function.arguments)?;
                let result = tools.execute(tool_name, args).await?;

                self.history.push(Message {
                    role: Tool,
                    content: Some(result),
                    reasoning_content: None,
                    tool_calls: None,
                    tool_call_id: Some(call.id.clone()),
                    name: Some(tool_name.clone()),
                });
            }
            // Loop again
        }
    }

    /// Send the prompt to the model `(Network I/O)`, with or without tools, and return the model's response, reasoning, and any tool calls.
    async fn send_prompt(
        &self,
        include_tools: bool,
    ) -> Result<(String, Option<String>, Option<Vec<ToolCall>>)> {
        let mut messages = Vec::with_capacity(1 + self.history.len()); // Necessary Evil
        messages.push(Message {
            role: System,
            content: Some(self.system_prompt.clone()),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });
        // Add the conversation history to the messages
        messages.extend_from_slice(&self.history);

        // Build the request with or without tools
        let request = if include_tools {
            CompletionRequest {
                model: self.model.clone(),
                messages,
                tools: self
                    .tool_registry
                    .as_ref()
                    .map(|reg| reg.get_tool_definitions()),
                temperature: self.temperature,
                stream: Some(false),
            }
        } else {
            CompletionRequest {
                model: self.model.clone(),
                messages,
                tools: None,
                temperature: self.temperature,
                stream: Some(false),
            }
        };

        // Parse and return
        let response = send_network_request(self, request).await?;
        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No choices in response"))?;

        Ok((
            choice.message.content.clone().unwrap_or_default(),
            choice.message.reasoning_content.clone(),
            choice.message.tool_calls.clone(),
        ))
    }
}

#[inline(always)]
fn user_message(text: &str) -> Message {
    Message {
        role: User,
        content: Some(text.to_string()),
        reasoning_content: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }
}

#[inline(always)]
fn assistant_message(
    content: String,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
) -> Message {
    Message {
        role: Assistant,
        content: Some(content),
        reasoning_content,
        tool_calls,
        tool_call_id: None,
        name: None,
    }
}
