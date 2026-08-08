use crate::dtos::Tool as ToolDTO;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait::async_trait]
/// Represents a tool that can be registered in the ToolRegistry and executed by the agent.
pub trait Tool: Send + Sync {
    /// Returns the name of the tool, used to identify it in the registry and when the agent calls it.
    fn name(&self) -> &str;
    /// Returns the tool's JSON definition describing its functionality, parameters, and usage.
    fn description(&self) -> Value;
    /// Executes the tool with the given arguments (as a JSON value) and returns the result as a string.
    async fn execute_tool(&self, args: Value) -> Result<String>;
}

/// A registry for managing and executing tools.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool + Send + Sync>>,
}

impl ToolRegistry {
    /// Init the inner Hashmap with a capacity of 64
    pub fn init() -> Self {
        Self {
            tools: HashMap::with_capacity(64),
        }
    }

    /// Registers a new tool in the registry.
    /// The tool must implement the [Tool] trait and will be stored in an Arc for thread-safe access.
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    /// Returns tool definitions for the API request.
    pub(crate) fn get_tool_definitions(&self) -> Vec<ToolDTO> {
        self.tools
            .values()
            .filter_map(|tool| {
                let desc = tool.description();
                serde_json::from_value(desc).ok()
            })
            .collect()
    }

    /// Executes the tool with the given name and arguments (as a JSON value) and returns some form of result as a string.
    /// TODO; the return string loop back to agent, when loop, but if there is no loop, you (human) will get whatever this str fn return, I should think of better design
    pub(crate) async fn execute(&self, tool_name: &str, args: Value) -> Result<String> {
        match self.tools.get(tool_name) {
            Some(tool) => tool.execute_tool(args).await,
            None => Err(anyhow!("Tool '{}' not found", tool_name)),
        }
    }
}
