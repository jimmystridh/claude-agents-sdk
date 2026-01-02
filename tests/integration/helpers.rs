//! Common test helpers for CLI integration tests.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{
    query, ClaudeAgentOptions, Message, PermissionMode, ResultMessage,
};
use std::time::Duration;
use tokio_stream::StreamExt;

/// Default timeout for integration tests.
pub const TEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Create default options for simple tests.
pub fn default_options() -> ClaudeAgentOptions {
    ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1)
}

/// Collect all messages from a query stream with timeout.
pub async fn collect_messages(
    prompt: &str,
    options: ClaudeAgentOptions,
) -> Result<Vec<Message>, String> {
    let result = tokio::time::timeout(TEST_TIMEOUT, async {
        let mut stream = query(prompt, Some(options), None)
            .await
            .map_err(|e| format!("Failed to start query: {}", e))?;

        let mut messages = Vec::new();
        while let Some(msg) = stream.next().await {
            messages.push(msg.map_err(|e| format!("Error in stream: {}", e))?);
        }
        Ok(messages)
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err("Test timed out".to_string()),
    }
}

/// Extract text from all assistant messages.
pub fn extract_assistant_text(messages: &[Message]) -> String {
    messages
        .iter()
        .filter_map(|m| {
            if let Message::Assistant(asst) = m {
                Some(asst.text())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Get the result message from a message list.
pub fn get_result(messages: &[Message]) -> Option<&ResultMessage> {
    messages.iter().find_map(|m| {
        if let Message::Result(r) = m {
            Some(r)
        } else {
            None
        }
    })
}

/// Assert that messages contain expected message types.
pub fn assert_message_types(messages: &[Message], expected: &[&str]) {
    for expected_type in expected {
        let found = messages.iter().any(|m| match (m, *expected_type) {
            (Message::System(_), "system") => true,
            (Message::Assistant(_), "assistant") => true,
            (Message::User(_), "user") => true,
            (Message::Result(_), "result") => true,
            (Message::StreamEvent(_), "stream_event") => true,
            _ => false,
        });
        assert!(
            found,
            "Expected message type '{}' not found in messages",
            expected_type
        );
    }
}
