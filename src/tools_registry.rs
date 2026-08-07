use crate::dtos::Tool as ToolDTO;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait::async_trait]
/// Represents a tool that can be registered in the ToolRegistry and executed by the agent.
/// Each tool must provide its name (by which agent calls), description (how this tool works),
/// will_callback (loop?), and execution logic.
pub trait Tool: Send + Sync {
    /// Returns the name of the tool, which is used to identify it in the registry and when agent calls it.
    fn name(&self) -> &str;
    /// Returns the description of the tool, which is a JSON object that describes the tool's functionality, parameters, and usage.
    fn description(&self) -> Value;
    /// Returns whether the tool will callback (loop).
    fn tool_callback(&self) -> bool;
    /// Executes the tool with the given arguments (as a JSON value) and returns the result as a string.
    async fn execute_tool(&self, args: Value) -> Result<String>;
}

/// A registry for managing and executing tools.
/// It allows registering new tools, retrieving tool definitions, checking if a tool will callback, and executing tools with given arguments.
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
    /// The tool must implement the Tool trait and will be stored in an Arc for thread-safe access.
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    /// Returns a vector of ToolDTOs representing the definitions of all registered tools.
    pub(crate) fn get_tool_definitions(&self) -> Vec<ToolDTO> {
        self.tools
            .values()
            .filter_map(|tool| {
                let desc = tool.description();
                serde_json::from_value(desc).ok()
            })
            .collect()
    }

    /// Checks if a tool with the given name is registered and returns whether it will callback (loop).
    pub(crate) fn check_tool_callback(&self, tool_name: &str) -> Result<bool> {
        match self.tools.get(tool_name) {
            Some(tool) => Ok(tool.tool_callback()),
            None => Err(anyhow!("Tool '{}' not found", tool_name)),
        }
    }

    /// Executes the tool with the given name and arguments (as a JSON value) and returns the result as a string.
    pub(crate) async fn execute(&self, tool_name: &str, args: Value) -> Result<String> {
        match self.tools.get(tool_name) {
            Some(tool) => tool.execute_tool(args).await,
            None => Err(anyhow!("Tool '{}' not found", tool_name)),
        }
    }
}
