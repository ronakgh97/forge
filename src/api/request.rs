use crate::api::agents::Agent;
use crate::api::dtos::{CompletionRequest, CompletionResponse, CompletionStreamResponse};
use anyhow::{Context, Result};
use colored::Colorize;
use eventsource_stream::Eventsource;
use futures_util::stream::{Stream, StreamExt};
use std::io::{self, Write};
use tokio::time::{Duration, sleep};

/// Sends a completion request to the specified URL with the given API key and request body, returns the deserialized completion response
pub async fn send_request(agent: &Agent, request: CompletionRequest) -> Result<CompletionResponse> {
    let response = agent
        .client
        .post(format!("{}/chat/completions", agent.url))
        .header("Authorization", format!("Bearer {}", agent.api_key))
        .json(&request)
        .send()
        .await
        .context("failed to send request")?
        .error_for_status()?;

    let completion: CompletionResponse = response
        .json()
        .await
        .context("failed to deserialize completion response")?;

    Ok(completion)
}

/// Sends a completion request and returns a stream of generated content chunks as they arrive, handles SSE streaming and JSON parsing of each chunk
pub async fn send_request_stream(
    agent: &Agent,
    request: CompletionRequest,
) -> Result<impl Stream<Item = Result<String>> + Send> {
    let response = agent
        .client
        .post(format!("{}/chat/completions", agent.url))
        .header("Authorization", format!("Bearer {}", agent.api_key))
        .json(&request)
        .send()
        .await?
        .error_for_status()?;

    let stream = response
        .bytes_stream()
        .eventsource() // Decodes SSE "data: ..."
        .map(|event| {
            let event = event.context("Stream error")?;
            if event.data == "[DONE]" {
                return Ok(None);
            }
            let parsed: CompletionStreamResponse =
                serde_json::from_str(&event.data).context("Failed to parse JSON")?;

            // Extract content safely
            let content = parsed.choices.first().and_then(|c| c.delta.content.clone());

            Ok(content)
        })
        .filter_map(|res| async {
            match res {
                Ok(Some(text)) => Some(Ok(text)), // Valid content
                Ok(None) => None,                 // [DONE] or empty delta
                Err(e) => Some(Err(e)),           // Parser error
            }
        });

    Ok(stream)
}

/// Takes a stream of text chunks, collects the full text for proper word wrapping, then prints it character by character, returns the full unwrapped text
pub async fn console_log(
    wrap_len: usize,
    mut stream: impl Stream<Item = Result<String>> + Unpin,
) -> Result<String> {
    // Collect the full text first for proper word wrapping
    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        full_text.push_str(&chunk?);
    }

    // Word wrap the text (trim start to avoid leading blank lines)
    let wrapped_text = text_wrapping(full_text.trim_start(), wrap_len);

    // Print character by character with typewriter effect
    for c in wrapped_text.chars() {
        print!("{}", c.to_string().bright_white());
        io::stdout().flush()?;
        sleep(Duration::from_millis(10)).await;
    }
    println!();
    Ok(full_text)
}

fn text_wrapping(text: &str, width: usize) -> String {
    let mut result = String::new();
    for line in text.lines() {
        // Check if line is empty/blank to preserve blank lines
        if line.trim().is_empty() {
            result.push('\n');
            continue;
        }

        let words: Vec<&str> = line.split_whitespace().collect();
        let mut current_line = String::new();
        for word in words {
            let word_len = word.len();
            let space_needed = if current_line.is_empty() { 0 } else { 1 };
            if current_line.len() + space_needed + word_len > width {
                if !current_line.is_empty() {
                    result.push_str(&current_line);
                    result.push('\n');
                    current_line = word.to_string();
                } else {
                    // Word is longer than width, hard break it
                    let mut remaining = word;
                    while !remaining.is_empty() {
                        let take = remaining.len().min(width);
                        result.push_str(&remaining[..take]);
                        result.push('\n');
                        remaining = &remaining[take..];
                    }
                    current_line.clear();
                }
            } else {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            }
        }
        if !current_line.is_empty() {
            result.push_str(&current_line);
            result.push('\n');
        }
    }
    result.trim_end().to_string() // Remove trailing newline
}
