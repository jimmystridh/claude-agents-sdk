//! # Claude Agents SDK
//!
//! A Rust SDK for building agents that interact with the Claude Code CLI.
//!
//! This SDK provides two main entry points:
//!
//! - [`query`]: One-shot, unidirectional queries that return an async stream of messages
//! - [`ClaudeClient`]: Full bidirectional client with control protocol support
//!
//! ## Quick Start
//!
//! ### Simple Query
//!
//! ```rust,no_run
//! use claude_agents_sdk::{query, ClaudeAgentOptions, Message};
//! use tokio_stream::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let options = ClaudeAgentOptions::new()
//!         .with_max_turns(3);
//!
//!     let mut stream = query("What is 2 + 2?", Some(options)).await?;
//!
//!     while let Some(message) = stream.next().await {
//!         match message? {
//!             Message::Assistant(msg) => print!("{}", msg.text()),
//!             Message::Result(result) => {
//!                 println!("\nCost: ${:.4}", result.total_cost_usd.unwrap_or(0.0));
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Bidirectional Client
//!
//! ```rust,no_run
//! use claude_agents_sdk::ClaudeClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = ClaudeClient::new(None);
//!     client.connect().await?;
//!
//!     // First query
//!     client.query("What is the capital of France?").await?;
//!     let (response, _) = client.receive_response().await?;
//!     println!("Response: {}", response);
//!
//!     // Follow-up query (maintains context)
//!     client.query("What's its population?").await?;
//!     let (response, _) = client.receive_response().await?;
//!     println!("Response: {}", response);
//!
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Tool Permission Callbacks
//!
//! Control which tools Claude can use by providing a permission callback:
//!
//! ```rust,no_run
//! use claude_agents_sdk::{ClaudeClientBuilder, PermissionResult, PermissionMode};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = ClaudeClientBuilder::new()
//!         .permission_mode(PermissionMode::Default)
//!         .can_use_tool(|tool_name, input, _ctx| async move {
//!             println!("Tool requested: {} with {:?}", tool_name, input);
//!
//!             // Allow Read, deny dangerous Bash commands
//!             if tool_name == "Bash" {
//!                 if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
//!                     if cmd.contains("rm -rf") {
//!                         return PermissionResult::deny_with_message("Dangerous command");
//!                     }
//!                 }
//!             }
//!
//!             PermissionResult::allow()
//!         })
//!         .build();
//!
//!     client.connect().await?;
//!     // ... use client
//!     Ok(())
//! }
//! ```
//!
//! ## Error Handling
//!
//! The SDK provides [`ClaudeSDKError`] for comprehensive error handling:
//!
//! ```rust,no_run
//! use claude_agents_sdk::{query, ClaudeAgentOptions, ClaudeSDKError, PermissionMode};
//!
//! #[tokio::main]
//! async fn main() {
//!     let options = ClaudeAgentOptions::new()
//!         .with_permission_mode(PermissionMode::Default)
//!         .with_timeout_secs(30);
//!
//!     match query("Hello", Some(options)).await {
//!         Ok(stream) => {
//!             // Process stream...
//!         }
//!         Err(ClaudeSDKError::CLINotFound { message }) => {
//!             eprintln!("Claude CLI not installed: {}", message);
//!         }
//!         Err(ClaudeSDKError::Timeout { duration_ms }) => {
//!             eprintln!("Operation timed out after {}ms", duration_ms);
//!         }
//!         Err(e) => {
//!             eprintln!("Error: {}", e);
//!         }
//!     }
//! }
//! ```
//!
//! ## Configuration Options
//!
//! Configure queries with [`ClaudeAgentOptions`]:
//!
//! ```rust
//! use claude_agents_sdk::{ClaudeAgentOptions, PermissionMode};
//!
//! let options = ClaudeAgentOptions::new()
//!     .with_model("claude-sonnet-4-20250514")
//!     .with_system_prompt("You are a helpful coding assistant.")
//!     .with_max_turns(10)
//!     .with_permission_mode(PermissionMode::AcceptEdits)
//!     .with_allowed_tools(vec!["Read".into(), "Write".into()])
//!     .with_timeout_secs(60);
//! ```
//!
//! ## Feature Flags
//!
//! - **default**: Core SDK functionality
//! - **mcp**: Enables MCP (Model Context Protocol) tool support for defining custom tools

#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod client;
mod errors;
mod query;
mod types;

pub mod _internal;

// Re-export public API
pub use client::{ClaudeClient, ClaudeClientBuilder, ClientGuard};
pub use errors::*;
pub use query::{query, query_all, query_chunks, query_result};
pub use types::*;

// Re-export MCP tools when feature enabled
#[cfg(feature = "mcp")]
#[cfg_attr(docsrs, doc(cfg(feature = "mcp")))]
pub mod mcp;

#[cfg(feature = "mcp")]
pub use mcp::{create_sdk_mcp_server, McpSdkServerConfig, SdkMcpTool};

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Minimum required Claude CLI version
pub const MIN_CLI_VERSION: &str = "2.0.0";
