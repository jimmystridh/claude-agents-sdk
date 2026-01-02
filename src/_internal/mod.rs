//! Internal implementation details for the Claude Agents SDK.
//!
//! This module contains internal types and functions that are not part of the public API.
//! While exposed for advanced use cases, the API here may change between versions.

pub mod transport;
pub mod message_parser;
pub mod query;
pub mod client;

pub use transport::{Transport, SubprocessTransport};
pub use message_parser::parse_message;
pub use query::Query;
pub use client::InternalClient;
