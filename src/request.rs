use crate::agents::Agent;
use crate::dtos::{CompletionRequest, CompletionResponse};
use anyhow::{Context, Result};

/// Sends a completion request to the specified URL with the given API key and request body, returns the deserialized completion response
pub(crate) async fn send_network_request(
    agent: &Agent,
    request: CompletionRequest,
) -> Result<CompletionResponse> {
    let response = agent
        .client
        .post(format!("{}/chat/completions", agent.url))
        .header("Authorization", format!("Bearer {}", agent.api_key))
        .json(&request)
        .send()
        .await?
        .error_for_status()?;

    let completion: CompletionResponse = response
        .json()
        .await
        .context("Failed to deserialize completion response")?;

    Ok(completion)
}
