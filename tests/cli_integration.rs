//! CLI integration tests.
//!
//! These tests verify end-to-end functionality with the actual Claude CLI.
//! They require the Claude CLI to be installed and configured.
//!
//! Run with: cargo test --features integration-tests
//!
//! These tests are behind a feature flag because they:
//! 1. Require the Claude CLI to be installed
//! 2. Make actual API calls (cost money)
//! 3. Take significant time to run

#![cfg(feature = "integration-tests")]

mod integration;
