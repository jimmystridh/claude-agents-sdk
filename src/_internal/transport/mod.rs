//! Transport layer for communicating with the Claude CLI.
//!
//! This module provides the [`Transport`] trait for abstracting communication
//! with the CLI process, and [`SubprocessTransport`] as the concrete implementation.

mod subprocess;

pub use subprocess::SubprocessTransport;

use async_trait::async_trait;
use tokio_stream::Stream;
use std::pin::Pin;

use crate::errors::Result;

/// Abstract transport trait for CLI communication.
///
/// This trait defines the interface for bidirectional communication with
/// the Claude CLI process. Implementations handle process lifecycle,
/// message serialization, and stream management.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect to the CLI process.
    async fn connect(&mut self) -> Result<()>;

    /// Write a message to the CLI.
    async fn write(&self, data: &str) -> Result<()>;

    /// Get a stream of messages from the CLI.
    fn message_stream(&self) -> Pin<Box<dyn Stream<Item = Result<serde_json::Value>> + Send + '_>>;

    /// Close the connection gracefully.
    async fn close(&mut self) -> Result<()>;

    /// Signal end of input (close stdin).
    async fn end_input(&self) -> Result<()>;

    /// Check if the transport is ready for communication.
    fn is_ready(&self) -> bool;
}
