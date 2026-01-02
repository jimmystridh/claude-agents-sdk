//! One-shot query function for simple interactions.
//!
//! This module provides the [`query`] function for simple, one-shot queries
//! to Claude. For more complex interactions requiring bidirectional
//! communication, use [`ClaudeClient`](crate::ClaudeClient).

use std::pin::Pin;
use tokio_stream::Stream;

use crate::_internal::client::InternalClient;
use crate::_internal::transport::Transport;
use crate::errors::Result;
use crate::types::{ClaudeAgentOptions, Message};

/// Execute a one-shot query to Claude.
///
/// This is the simplest way to interact with Claude. It sends a prompt
/// and returns a stream of messages. The stream completes when Claude
/// finishes responding.
///
/// # Arguments
///
/// * `prompt` - The prompt to send to Claude
/// * `options` - Optional configuration for the query
/// * `transport` - Optional custom transport (uses subprocess by default)
///
/// # Returns
///
/// An async stream of [`Message`]s from Claude.
///
/// # Examples
///
/// ```rust,no_run
/// use claude_agents_sdk::{query, ClaudeAgentOptions, Message};
/// use tokio_stream::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::new()
///         .with_model("claude-3-sonnet")
///         .with_max_turns(3);
///
///     let mut stream = query("Hello, Claude!", Some(options), None).await?;
///
///     while let Some(message) = stream.next().await {
///         match message? {
///             Message::Assistant(msg) => {
///                 println!("Claude: {}", msg.text());
///             }
///             Message::Result(result) => {
///                 println!("Completed in {}ms", result.duration_ms);
///             }
///             _ => {}
///         }
///     }
///
///     Ok(())
/// }
/// ```
///
/// # Notes
///
/// - For queries requiring tool permission callbacks or hooks, the SDK
///   automatically uses streaming mode internally.
/// - The returned stream must be fully consumed to ensure proper cleanup.
pub async fn query(
    prompt: &str,
    options: Option<ClaudeAgentOptions>,
    _transport: Option<Box<dyn Transport>>,
) -> Result<Pin<Box<dyn Stream<Item = Result<Message>> + Send>>> {
    let options = options.unwrap_or_default();
    InternalClient::process_query(options, prompt).await
}

/// Execute a query and collect all messages.
///
/// This is a convenience function that collects all messages from a query
/// into a vector. Useful for simple cases where you want to process all
/// results at once.
///
/// # Arguments
///
/// * `prompt` - The prompt to send to Claude
/// * `options` - Optional configuration for the query
///
/// # Returns
///
/// A vector of all [`Message`]s from the query.
///
/// # Examples
///
/// ```rust,no_run
/// use claude_agents_sdk::{query_all, ClaudeAgentOptions, Message};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let messages = query_all("What is 2 + 2?", None).await?;
///
///     for msg in messages {
///         if let Message::Assistant(asst) = msg {
///             println!("{}", asst.text());
///         }
///     }
///
///     Ok(())
/// }
/// ```
pub async fn query_all(
    prompt: &str,
    options: Option<ClaudeAgentOptions>,
) -> Result<Vec<Message>> {
    use tokio_stream::StreamExt;

    let mut stream = query(prompt, options, None).await?;
    let mut messages = Vec::new();

    while let Some(result) = stream.next().await {
        messages.push(result?);
    }

    Ok(messages)
}

/// Execute a query with a prompt built from chunks.
///
/// This is useful when you want to build a prompt from multiple parts,
/// such as reading from a file or combining multiple sources. This is
/// the Rust equivalent of Python's `AsyncIterable[str]` prompt support.
///
/// For true streaming input patterns where you need to send multiple
/// messages over time, use [`ClaudeClient`](crate::ClaudeClient) instead.
///
/// # Arguments
///
/// * `chunks` - An iterator of string chunks that will be joined to form the prompt
/// * `options` - Optional configuration for the query
///
/// # Returns
///
/// An async stream of [`Message`]s from Claude.
///
/// # Examples
///
/// ```rust,no_run
/// use claude_agents_sdk::{query_chunks, Message};
/// use tokio_stream::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let prompt_parts = vec![
///         "Please analyze this code:\n",
///         "```rust\n",
///         "fn main() { println!(\"Hello\"); }\n",
///         "```\n",
///     ];
///
///     let mut stream = query_chunks(prompt_parts, None).await?;
///
///     while let Some(message) = stream.next().await {
///         if let Ok(Message::Assistant(msg)) = message {
///             println!("{}", msg.text());
///         }
///     }
///
///     Ok(())
/// }
/// ```
pub async fn query_chunks<'a, I>(
    chunks: I,
    options: Option<ClaudeAgentOptions>,
) -> Result<Pin<Box<dyn Stream<Item = Result<Message>> + Send>>>
where
    I: IntoIterator<Item = &'a str>,
{
    let prompt: String = chunks.into_iter().collect();
    query(&prompt, options, None).await
}

/// Get the final result from a query.
///
/// This is a convenience function that runs a query and returns only the
/// final result message, which contains cost and usage information.
///
/// # Arguments
///
/// * `prompt` - The prompt to send to Claude
/// * `options` - Optional configuration for the query
///
/// # Returns
///
/// The final text response and the result message with metadata.
///
/// # Examples
///
/// ```rust,no_run
/// use claude_agents_sdk::query_result;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let (response, result) = query_result("What is the capital of France?", None).await?;
///
///     println!("Response: {}", response);
///     if let Some(cost) = result.total_cost_usd {
///         println!("Cost: ${:.4}", cost);
///     }
///
///     Ok(())
/// }
/// ```
pub async fn query_result(
    prompt: &str,
    options: Option<ClaudeAgentOptions>,
) -> Result<(String, crate::types::ResultMessage)> {
    use tokio_stream::StreamExt;

    let mut stream = query(prompt, options, None).await?;
    let mut response_parts: Vec<String> = Vec::new();
    let mut result_message = None;

    while let Some(result) = stream.next().await {
        match result? {
            Message::Assistant(msg) => {
                let text = msg.text();
                if !text.is_empty() {
                    response_parts.push(text);
                }
            }
            Message::Result(result) => {
                result_message = Some(result);
                break;
            }
            _ => {}
        }
    }

    let result = result_message.ok_or_else(|| {
        crate::errors::ClaudeSDKError::internal("Query completed without result message")
    })?;

    Ok((response_parts.concat(), result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_options_builder() {
        let options = ClaudeAgentOptions::new()
            .with_model("claude-3-sonnet")
            .with_max_turns(5)
            .with_system_prompt("You are a helpful assistant.");

        assert_eq!(options.model, Some("claude-3-sonnet".to_string()));
        assert_eq!(options.max_turns, Some(5));
    }

    #[test]
    fn test_query_chunks_builds_prompt() {
        // Test that query_chunks correctly joins chunks
        let chunks = vec!["Hello, ", "world", "!"];
        let prompt: String = chunks.into_iter().collect();
        assert_eq!(prompt, "Hello, world!");
    }

    #[test]
    fn test_query_chunks_with_empty() {
        let chunks: Vec<&str> = vec![];
        let prompt: String = chunks.into_iter().collect();
        assert_eq!(prompt, "");
    }

    #[test]
    fn test_query_chunks_with_single() {
        let chunks = vec!["Single chunk"];
        let prompt: String = chunks.into_iter().collect();
        assert_eq!(prompt, "Single chunk");
    }
}
