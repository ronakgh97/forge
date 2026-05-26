use crate::api::dtos::Role::{ASSISTANT, SYSTEM, TOOL};
use crate::api::dtos::{CompletionRequest, Message, ToolCall};
use crate::api::request::{send_completion_request, send_request_stream};
use crate::api::tools_registry::ToolRegistry;
use anyhow::{Result, anyhow};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone)]
pub struct Agent {
    model: String,
    url: String,
    api_key: String,
    system_prompt: String,
    temperature: f32,
    #[serde(skip_serializing, skip_deserializing, default)]
    tool_registry: Option<Arc<ToolRegistry>>,
    top_p: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentBuilder {
    model: Option<String>,
    url: String,
    api_key: String,
    system_prompt: String,
    temperature: f32,
    #[serde(skip_serializing, skip_deserializing, default)]
    tool_registry: Option<Arc<ToolRegistry>>,
    top_p: f32,
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self {
            model: None,
            url: "http://localhost:1234/v1".to_string(),
            api_key: "local".to_string(),
            system_prompt: "Strictly follow user instructions!!!".to_string(),
            tool_registry: None,
            temperature: 0.7,
            top_p: 0.9,
        }
    }
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_json(path: &str) -> Result<Self> {
        let config_str = std::fs::read_to_string(path)?;
        let agent_builder = serde_json::from_str::<AgentBuilder>(&config_str)?;
        Ok(agent_builder)
    }

    pub fn to_json_str(&self) -> Result<String> {
        let toml_str = serde_json::to_string_pretty(self)?;
        Ok(toml_str)
    }

    pub fn convert_to_builder(agent: &Agent) -> AgentBuilder {
        AgentBuilder {
            model: Some(agent.model.clone()),
            url: agent.url.clone(),
            api_key: agent.api_key.clone(),
            system_prompt: agent.system_prompt.clone(),
            temperature: agent.temperature,
            tool_registry: agent.tool_registry.clone(),
            top_p: agent.top_p,
        }
    }

    pub fn model(mut self, model: &str) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = url.to_string();
        self
    }

    pub fn api_key(mut self, api_key: &str) -> Self {
        self.api_key = api_key.to_string();
        self
    }

    pub fn system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = prompt.to_string();
        self
    }

    pub fn tool_registry(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p;
        self
    }

    pub fn build(self) -> Result<Agent> {
        Ok(Agent {
            model: self
                .model
                .ok_or_else(|| anyhow::anyhow!("Model is required"))?,
            url: self.url,
            api_key: self.api_key,
            system_prompt: self.system_prompt,
            temperature: self.temperature,
            tool_registry: self.tool_registry,
            top_p: self.top_p,
        })
    }
}

/// Low level function to send a prompt with system prompt both as vec<history> format and response from the agent.
/// returns response in vec<history> and tool_calls if any
pub async fn prompt(
    agent: &Agent,
    history: Vec<Message>,
) -> Result<(String, Option<Vec<ToolCall>>)> {
    let mut history = history;
    history.insert(
        0,
        Message {
            role: SYSTEM,
            content: Some(agent.system_prompt.clone()),
            multi_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
    );

    let request = CompletionRequest {
        model: agent.clone().model,
        messages: history,
        tools: agent
            .tool_registry
            .as_ref()
            .map(|reg| reg.get_tool_definitions()),
        temperature: agent.temperature,
        top_p: Some(agent.top_p),
        stream: Some(false),
    };

    let response =
        send_completion_request(agent.url.clone(), agent.api_key.clone(), request).await?;

    let get_content = &response
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("No choices in response"))?
        .message
        .content
        .clone()
        .unwrap_or_default();

    let tool_call = &response
        .choices
        .first()
        .ok_or_else(|| anyhow!("No choices in response"))?
        .message
        .tool_calls;

    Ok((get_content.clone(), tool_call.clone()))
}

/// Stream version of [`prompt`].
/// returns stream of response content only (no intermediate tool calls or assistant messages).
pub async fn prompt_stream(
    agent: &Agent,
    history: Vec<Message>,
) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
    let mut history = history;
    history.insert(
        0,
        Message {
            role: SYSTEM,
            content: Some(agent.clone().system_prompt),
            multi_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
    );

    let request = CompletionRequest {
        model: agent.clone().model,
        messages: history,
        tools: agent
            .tool_registry
            .as_ref()
            .map(|reg| reg.get_tool_definitions()),
        temperature: agent.temperature,
        top_p: Some(agent.top_p),
        stream: Some(true),
    };

    let stream = send_request_stream(agent.url.clone(), agent.api_key.clone(), request).await?;

    Ok(Box::pin(stream))
}

/// High-level helper.
///
/// - Runs the tool loop internally until no more tool calls are needed.
/// - Does NOT expose intermediate tool or assistant messages.
/// - Suitable for stateless, one-shot queries.
/// - If you need full control over history or tools, use [`prompt`] directly.
pub async fn prompt_with_tools(
    agent: &Agent,
    mut history: Vec<Message>,
    max_loop: usize,
) -> Result<String> {
    // TODO: Return history?
    let registry = match &agent.tool_registry {
        Some(r) => r,
        None => return Err(anyhow::anyhow!("No tool registry")),
    };

    for _ in 0..max_loop {
        let (response, tools_list) = prompt(&agent.clone(), history.clone()).await?;

        // No tool calls? STOP!!
        if tools_list.is_none() {
            return Ok(response);
        }

        let calls = tools_list.unwrap(); // Safe unwrap

        // Add assistant message with tool_calls FIRST
        history.push(Message {
            role: ASSISTANT,
            content: Some(response.clone()),
            multi_content: None,
            tool_calls: Some(calls.clone()),
            tool_call_id: None,
            name: None,
        });

        let mut should_loop = false;

        // Execute each tool
        for call in calls {
            let tool_name = &call.function.name;
            let should_callback = registry.check_tool_callback(tool_name)?;

            let args: serde_json::Value = serde_json::from_str(&call.function.arguments)?;
            let result = registry.execute(tool_name, args).await?;

            if !should_callback {
                return Ok(result);
            }

            history.push(Message {
                role: TOOL,
                content: Some(result),
                multi_content: None,
                tool_calls: None,
                tool_call_id: Some(call.id.clone()),
                name: Some(tool_name.clone()),
            });

            should_loop = true;
        }

        if !should_loop {
            // No tools wanted callback
            return Ok(response);
        }
    }

    Err(anyhow::anyhow!("Max iterations ({}) reached", max_loop))
}

/// High-level streaming with automatic tool execution.
///
/// - Executes tools silently (non-streaming)
/// - Returns stream of final answer only
/// - Compatible with [`prompt_with_tools`] design
pub async fn prompt_with_tools_stream(
    agent: &Agent,
    mut history: Vec<Message>,
    max_loop: usize,
) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
    let registry = agent
        .tool_registry
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No tool registry"))?;

    for _ in 0..max_loop {
        let (response, tools_list) = prompt(&agent.clone(), history.clone()).await?;

        // No tool calls? STOP!!
        if tools_list.is_none() {
            return prompt_stream(agent, history).await;
        }

        let calls = tools_list.unwrap(); // Safe unwrap

        history.push(Message {
            role: ASSISTANT,
            content: Some(response.clone()),
            multi_content: None,
            tool_calls: Some(calls.clone()),
            tool_call_id: None,
            name: None,
        });

        let mut should_loop = false;

        for call in calls {
            let tool_name = &call.function.name;
            let should_callback = registry.check_tool_callback(tool_name)?;

            let args: serde_json::Value = serde_json::from_str(&call.function.arguments)?;
            let result = registry.execute(tool_name, args).await?;

            if !should_callback {
                use futures_util::stream;
                let stream = stream::once(async move { Ok(result) });
                return Ok(Box::pin(stream));
            }

            history.push(Message {
                role: TOOL,
                content: Some(result),
                multi_content: None,
                tool_calls: None,
                tool_call_id: Some(call.id.clone()),
                name: Some(tool_name.clone()),
            });

            should_loop = true;
        }

        if !should_loop {
            use futures_util::stream;
            let stream = stream::once(async move { Ok(response) });
            return Ok(Box::pin(stream));
        }
    }

    Err(anyhow::anyhow!("Max iterations ({}) reached", max_loop))
}
