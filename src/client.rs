//! Bidirectional Claude client for streaming interactions.
//!
//! This module provides [`ClaudeClient`], a full-featured client for
//! bidirectional communication with Claude. It supports:
//!
//! - Multiple queries in a single session
//! - Tool permission callbacks
//! - Hook callbacks
//! - Runtime model and permission changes
//! - File checkpointing and rewinding

use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};

use crate::_internal::client::InternalClient;
use crate::errors::{ClaudeSDKError, Result};
use crate::types::*;

/// Bidirectional client for streaming Claude interactions.
///
/// `ClaudeClient` provides a full-featured interface for interactive
/// conversations with Claude. Unlike the simple [`query`](crate::query)
/// function, this client maintains a persistent connection and supports
/// multiple queries, callbacks, and runtime configuration changes.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,no_run
/// use claude_agents_sdk::ClaudeClient;
/// use tokio_stream::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut client = ClaudeClient::new(None);
///     client.connect().await?;
///
///     // Send first query
///     client.query("What is Rust?").await?;
///
///     // Process responses
///     while let Some(msg) = client.receive_messages().next().await {
///         println!("{:?}", msg?);
///     }
///
///     client.disconnect().await?;
///     Ok(())
/// }
/// ```
///
/// ## With Tool Permission Callback
///
/// ```rust,no_run
/// use claude_agents_sdk::{ClaudeClient, ClaudeAgentOptions, PermissionResult};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::new()
///         .with_can_use_tool(|tool_name, input, _ctx| async move {
///             println!("Tool request: {} with {:?}", tool_name, input);
///             PermissionResult::allow()
///         });
///
///     let mut client = ClaudeClient::new(Some(options));
///     client.connect().await?;
///
///     // Queries will now invoke the callback for tool permissions
///
///     Ok(())
/// }
/// ```
pub struct ClaudeClient {
    /// Internal client implementation.
    internal: InternalClient,
    /// Message receiver from the internal client.
    message_rx: Option<mpsc::Receiver<Result<Message>>>,
}

impl ClaudeClient {
    /// Create a new Claude client.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional configuration for the client
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::{ClaudeClient, ClaudeAgentOptions, PermissionMode};
    ///
    /// // Default configuration
    /// let client = ClaudeClient::new(None);
    ///
    /// // With custom options
    /// let options = ClaudeAgentOptions::new()
    ///     .with_model("claude-3-opus")
    ///     .with_permission_mode(PermissionMode::AcceptEdits);
    /// let client = ClaudeClient::new(Some(options));
    /// ```
    pub fn new(options: Option<ClaudeAgentOptions>) -> Self {
        Self {
            internal: InternalClient::new(options.unwrap_or_default()),
            message_rx: None,
        }
    }

    /// Connect to the Claude CLI.
    ///
    /// This establishes a connection to the CLI process and initializes
    /// the streaming session. Must be called before sending queries.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The CLI is not found
    /// - The CLI version is incompatible
    /// - Connection fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::ClaudeClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = ClaudeClient::new(None);
    ///     client.connect().await?;
    ///     // Client is now ready for queries
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect(&mut self) -> Result<()> {
        self.internal.connect().await?;
        self.message_rx = self.internal.take_message_rx();
        Ok(())
    }

    /// Send a query to Claude.
    ///
    /// Sends a new prompt to Claude. Responses can be received using
    /// [`receive_messages`](Self::receive_messages) or
    /// [`receive_response`](Self::receive_response).
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to send
    ///
    /// # Errors
    ///
    /// Returns an error if the client is not connected.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::ClaudeClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = ClaudeClient::new(None);
    ///     client.connect().await?;
    ///
    ///     client.query("Hello!").await?;
    ///     client.query("Follow-up question").await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn query(&mut self, prompt: &str) -> Result<()> {
        self.internal.send_message(prompt).await
    }

    /// Get a stream of messages from the current query.
    ///
    /// Returns a stream that yields messages as they are received from
    /// Claude. The stream ends when a result message is received.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::{ClaudeClient, Message};
    /// use tokio_stream::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = ClaudeClient::new(None);
    ///     client.connect().await?;
    ///     client.query("Tell me a joke").await?;
    ///
    ///     while let Some(msg) = client.receive_messages().next().await {
    ///         match msg? {
    ///             Message::Assistant(asst) => println!("{}", asst.text()),
    ///             Message::Result(_) => break,
    ///             _ => {}
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn receive_messages(&mut self) -> impl Stream<Item = Result<Message>> + '_ {
        futures::stream::poll_fn(move |cx| {
            if let Some(ref mut rx) = self.message_rx {
                Pin::new(rx).poll_recv(cx)
            } else {
                std::task::Poll::Ready(None)
            }
        })
    }

    /// Receive the complete response for the current query.
    ///
    /// Collects all messages until a result message is received and returns
    /// the combined response text along with the result metadata.
    ///
    /// # Returns
    ///
    /// A tuple of (response_text, result_message).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::ClaudeClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = ClaudeClient::new(None);
    ///     client.connect().await?;
    ///     client.query("What is 2 + 2?").await?;
    ///
    ///     let (response, result) = client.receive_response().await?;
    ///     println!("Response: {}", response);
    ///     println!("Turns: {}", result.num_turns);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn receive_response(&mut self) -> Result<(String, ResultMessage)> {
        let mut response_parts: Vec<String> = Vec::new();

        while let Some(msg) = self.receive_messages().next().await {
            match msg? {
                Message::Assistant(asst) => {
                    let text = asst.text();
                    if !text.is_empty() {
                        response_parts.push(text);
                    }
                }
                Message::Result(result) => {
                    return Ok((response_parts.concat(), result));
                }
                _ => {}
            }
        }

        Err(ClaudeSDKError::internal("Connection closed without result"))
    }

    /// Interrupt the current operation.
    ///
    /// Sends an interrupt signal to Claude, stopping the current response.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::ClaudeClient;
    /// use tokio::time::{timeout, Duration};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = ClaudeClient::new(None);
    ///     client.connect().await?;
    ///     client.query("Write a very long story").await?;
    ///
    ///     // Interrupt after 5 seconds
    ///     tokio::time::sleep(Duration::from_secs(5)).await;
    ///     client.interrupt().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn interrupt(&self) -> Result<()> {
        self.internal.interrupt().await
    }

    /// Change the permission mode for the session.
    ///
    /// # Arguments
    ///
    /// * `mode` - The new permission mode
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::{ClaudeClient, PermissionMode};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = ClaudeClient::new(None);
    ///     client.connect().await?;
    ///
    ///     // Switch to accept edits mode
    ///     client.set_permission_mode(PermissionMode::AcceptEdits).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn set_permission_mode(&self, mode: PermissionMode) -> Result<()> {
        self.internal.set_permission_mode(mode).await
    }

    /// Change the model for the session.
    ///
    /// # Arguments
    ///
    /// * `model` - The new model identifier
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::ClaudeClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = ClaudeClient::new(None);
    ///     client.connect().await?;
    ///
    ///     // Switch to a different model
    ///     client.set_model("claude-3-opus").await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn set_model(&self, model: impl Into<String>) -> Result<()> {
        self.internal.set_model(model).await
    }

    /// Rewind files to a specific user message.
    ///
    /// This is only available when file checkpointing is enabled.
    ///
    /// # Arguments
    ///
    /// * `user_message_id` - The UUID of the user message to rewind to
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::{ClaudeClient, ClaudeAgentOptions};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut options = ClaudeAgentOptions::new();
    ///     options.enable_file_checkpointing = true;
    ///
    ///     let mut client = ClaudeClient::new(Some(options));
    ///     client.connect().await?;
    ///
    ///     // Later, rewind to a previous state
    ///     client.rewind_files("user-message-uuid").await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn rewind_files(&self, user_message_id: impl Into<String>) -> Result<()> {
        self.internal.rewind_files(user_message_id).await
    }

    /// Get server initialization info.
    ///
    /// Returns the initialization response from the CLI, which includes
    /// available commands, output styles, and server capabilities.
    ///
    /// # Returns
    ///
    /// `Some(Value)` with server info if connected and initialized, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::ClaudeClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = ClaudeClient::new(None);
    ///     client.connect().await?;
    ///
    ///     if let Some(info) = client.get_server_info().await {
    ///         println!("Commands: {:?}", info.get("commands"));
    ///         println!("Output style: {:?}", info.get("output_style"));
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_server_info(&self) -> Option<serde_json::Value> {
        self.internal.get_server_info().await
    }

    /// Disconnect from the Claude CLI.
    ///
    /// Gracefully closes the connection to the CLI process.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_agents_sdk::ClaudeClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = ClaudeClient::new(None);
    ///     client.connect().await?;
    ///
    ///     // Use the client...
    ///
    ///     client.disconnect().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn disconnect(&mut self) -> Result<()> {
        self.message_rx = None;
        self.internal.disconnect().await
    }

    /// Check if the client is connected.
    pub fn is_connected(&self) -> bool {
        self.internal.is_connected()
    }
}

/// Builder for creating a [`ClaudeClient`] with configuration.
///
/// Provides a fluent API for configuring the client before connecting.
///
/// # Examples
///
/// ```rust,no_run
/// use claude_agents_sdk::{ClaudeClientBuilder, PermissionMode, PermissionResult};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut client = ClaudeClientBuilder::new()
///         .model("claude-3-sonnet")
///         .permission_mode(PermissionMode::AcceptEdits)
///         .max_turns(10)
///         .can_use_tool(|tool, input, _| async move {
///             println!("Tool: {} Input: {:?}", tool, input);
///             PermissionResult::allow()
///         })
///         .build();
///
///     client.connect().await?;
///     Ok(())
/// }
/// ```
pub struct ClaudeClientBuilder {
    options: ClaudeAgentOptions,
}

impl ClaudeClientBuilder {
    /// Create a new builder with default options.
    pub fn new() -> Self {
        Self {
            options: ClaudeAgentOptions::new(),
        }
    }

    /// Set the model to use.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.options.model = Some(model.into());
        self
    }

    /// Set the system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.options.system_prompt = Some(SystemPromptConfig::Text(prompt.into()));
        self
    }

    /// Set the permission mode.
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.options.permission_mode = Some(mode);
        self
    }

    /// Set the maximum number of turns.
    pub fn max_turns(mut self, turns: u32) -> Self {
        self.options.max_turns = Some(turns);
        self
    }

    /// Set the maximum budget in USD.
    pub fn max_budget_usd(mut self, budget: f64) -> Self {
        self.options.max_budget_usd = Some(budget);
        self
    }

    /// Set the working directory.
    pub fn cwd(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.options.cwd = Some(path.into());
        self
    }

    /// Set the tool permission callback.
    pub fn can_use_tool<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, serde_json::Value, ToolPermissionContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = PermissionResult> + Send + 'static,
    {
        self.options = self.options.with_can_use_tool(callback);
        self
    }

    /// Enable partial message streaming.
    pub fn include_partial_messages(mut self) -> Self {
        self.options.include_partial_messages = true;
        self
    }

    /// Enable file checkpointing.
    pub fn enable_file_checkpointing(mut self) -> Self {
        self.options.enable_file_checkpointing = true;
        self
    }

    /// Set allowed tools.
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.allowed_tools = tools;
        self
    }

    /// Set disallowed tools.
    pub fn disallowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.disallowed_tools = tools;
        self
    }

    /// Build the client.
    pub fn build(self) -> ClaudeClient {
        ClaudeClient::new(Some(self.options))
    }
}

impl Default for ClaudeClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A guard that automatically disconnects a [`ClaudeClient`] when dropped.
///
/// This provides RAII-style resource management for the client connection,
/// similar to Python's async context manager (`async with`).
///
/// # Examples
///
/// ```rust,no_run
/// use claude_agents_sdk::{ClaudeClient, ClientGuard};
/// use tokio_stream::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut client = ClaudeClient::new(None);
///     client.connect().await?;
///
///     // Create guard - client will be disconnected when guard is dropped
///     let mut guard = ClientGuard::new(client);
///
///     guard.client_mut().query("Hello!").await?;
///     let (response, _) = guard.client_mut().receive_response().await?;
///     println!("{}", response);
///
///     // Client automatically disconnected when guard goes out of scope
///     Ok(())
/// }
/// ```
///
/// Or using the convenience method:
///
/// ```rust,no_run
/// use claude_agents_sdk::ClaudeClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut client = ClaudeClient::new(None);
///     client.connect().await?;
///
///     let guard = client.into_guard();
///     // Use guard.client() for all operations
///     // Automatically disconnects on drop
///
///     Ok(())
/// }
/// ```
pub struct ClientGuard {
    client: Option<ClaudeClient>,
    runtime: Option<tokio::runtime::Handle>,
}

impl ClientGuard {
    /// Create a new guard for the client.
    ///
    /// # Note
    /// If called outside of a Tokio runtime context, the guard will still work
    /// but automatic disconnect on drop will be skipped (with a warning logged).
    pub fn new(client: ClaudeClient) -> Self {
        Self {
            client: Some(client),
            runtime: tokio::runtime::Handle::try_current().ok(),
        }
    }

    /// Get a reference to the client.
    pub fn client(&self) -> &ClaudeClient {
        self.client.as_ref().expect("Client already taken")
    }

    /// Get a mutable reference to the client.
    pub fn client_mut(&mut self) -> &mut ClaudeClient {
        self.client.as_mut().expect("Client already taken")
    }

    /// Take ownership of the client, preventing automatic disconnect.
    pub fn into_inner(mut self) -> ClaudeClient {
        self.client.take().expect("Client already taken")
    }
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        if let Some(mut client) = self.client.take() {
            // Spawn a task to disconnect - we can't do async in drop
            if let Some(runtime) = &self.runtime {
                runtime.spawn(async move {
                    let _ = client.disconnect().await;
                });
            } else {
                // No runtime available - skip async disconnect
                // The underlying transport will be dropped anyway
                tracing::warn!(
                    "ClientGuard dropped without Tokio runtime - skipping async disconnect"
                );
            }
        }
    }
}

impl ClaudeClient {
    /// Convert this client into a guard that automatically disconnects on drop.
    ///
    /// This is the Rust equivalent of Python's `async with ClaudeSDKClient() as client:`
    /// pattern.
    pub fn into_guard(self) -> ClientGuard {
        ClientGuard::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = ClaudeClientBuilder::new()
            .model("claude-3-sonnet")
            .max_turns(5)
            .build();

        assert!(!client.is_connected());
    }
}
