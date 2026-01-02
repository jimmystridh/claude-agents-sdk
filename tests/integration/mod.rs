//! Integration test modules for CLI tests.
//!
//! These tests require the Claude CLI to be installed and configured.
//! Run with: cargo test --features integration-tests
//!
//! These tests are behind a feature flag because they:
//! 1. Require the Claude CLI to be installed
//! 2. Make actual API calls (cost money)
//! 3. Take significant time to run

pub mod helpers;
pub mod test_core;
pub mod test_system_prompt;
pub mod test_client;
pub mod test_tools;
pub mod test_results;
pub mod test_edge_cases;
pub mod test_context;
