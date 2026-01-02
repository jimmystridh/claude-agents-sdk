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
//! use claude_agents_sdk::{query, ClaudeAgentOptions};
//! use tokio_stream::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut stream = query("Hello, Claude!", None, None).await?;
//!
//!     while let Some(message) = stream.next().await {
//!         match message? {
//!             claude_agents_sdk::Message::Assistant(msg) => {
//!                 println!("Claude: {:?}", msg.content);
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
//! use tokio_stream::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = ClaudeClient::new(None, None);
//!     client.connect().await?;
//!
//!     client.query("What is 2 + 2?").await?;
//!
//!     while let Some(message) = client.receive_messages().next().await {
//!         println!("Received: {:?}", message?);
//!     }
//!
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod errors;
mod types;
mod query;
mod client;

pub mod _internal;

// Re-export public API
pub use errors::*;
pub use types::*;
pub use query::{query, query_all, query_chunks, query_result};
pub use client::{ClaudeClient, ClaudeClientBuilder, ClientGuard};

// Re-export MCP tools when feature enabled
#[cfg(feature = "mcp")]
#[cfg_attr(docsrs, doc(cfg(feature = "mcp")))]
pub mod mcp;

#[cfg(feature = "mcp")]
pub use mcp::{SdkMcpTool, create_sdk_mcp_server, McpSdkServerConfig};

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Minimum required Claude CLI version
pub const MIN_CLI_VERSION: &str = "2.0.0";
