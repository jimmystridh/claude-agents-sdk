//! Common test helpers for CLI integration tests.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{
    query, ClaudeAgentOptions, ContentBlock, Message, PermissionMode, ResultMessage, ToolUseBlock,
};
use std::future::Future;
use std::process::Command;
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

/// Get combined response text from all assistant messages.
/// Alias for extract_assistant_text.
pub fn get_response_text(messages: &[Message]) -> String {
    extract_assistant_text(messages)
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
        let found = messages.iter().any(|m| {
            matches!(
                (m, *expected_type),
                (Message::System(_), "system")
                    | (Message::Assistant(_), "assistant")
                    | (Message::User(_), "user")
                    | (Message::Result(_), "result")
                    | (Message::StreamEvent(_), "stream_event")
            )
        });
        assert!(
            found,
            "Expected message type '{}' not found in messages",
            expected_type
        );
    }
}

// ============================================================================
// Additional Helpers (Phase 10.1)
// ============================================================================

/// Collect messages with detailed error reporting.
#[allow(dead_code)]
pub async fn collect_messages_verbose(
    prompt: &str,
    options: ClaudeAgentOptions,
) -> Result<Vec<Message>, String> {
    let result = tokio::time::timeout(TEST_TIMEOUT, async {
        let mut stream = query(prompt, Some(options), None)
            .await
            .map_err(|e| format!("Failed to start query: {:?}", e))?;

        let mut messages = Vec::new();
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(m) => messages.push(m),
                Err(e) => {
                    eprintln!("Stream error after {} messages: {:?}", messages.len(), e);
                    return Err(format!(
                        "Error in stream after {} messages: {:?}",
                        messages.len(),
                        e
                    ));
                }
            }
        }
        Ok(messages)
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(format!("Test timed out after {:?}", TEST_TIMEOUT)),
    }
}

/// Assert response contains expected text (case-insensitive).
#[allow(dead_code)]
pub fn assert_response_contains(messages: &[Message], expected: &str) {
    let response = extract_assistant_text(messages);
    assert!(
        response.to_lowercase().contains(&expected.to_lowercase()),
        "Expected response to contain '{}'\nActual response:\n{}",
        expected,
        response
    );
}

/// Extract all tool uses from messages.
#[allow(dead_code)]
pub fn extract_tool_uses(messages: &[Message]) -> Vec<&ToolUseBlock> {
    messages
        .iter()
        .filter_map(|m| {
            if let Message::Assistant(asst) = m {
                Some(&asst.content)
            } else {
                None
            }
        })
        .flatten()
        .filter_map(|block| {
            if let ContentBlock::ToolUse(tool) = block {
                Some(tool)
            } else {
                None
            }
        })
        .collect()
}

/// Count running claude processes (platform-specific).
#[cfg(unix)]
pub fn count_claude_processes() -> usize {
    let output = Command::new("pgrep").args(["-f", "claude"]).output().ok();

    match output {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).lines().count(),
        _ => 0,
    }
}

/// Count running claude processes (Windows).
#[cfg(windows)]
pub fn count_claude_processes() -> usize {
    let output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq claude*"])
        .output()
        .ok();

    match output {
        Some(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| l.to_lowercase().contains("claude"))
            .count(),
        _ => 0,
    }
}

/// Retry an async function with exponential backoff.
#[allow(dead_code)]
pub async fn with_retry<F, Fut, T, E>(max_attempts: usize, f: F) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut last_error = None;

    for attempt in 0..max_attempts {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                eprintln!("Attempt {} failed: {:?}", attempt + 1, e);
                last_error = Some(e);

                if attempt < max_attempts - 1 {
                    let delay = Duration::from_millis(100 * 2u64.pow(attempt as u32));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}

/// Create options for tests that require tool usage.
#[allow(dead_code)]
pub fn tool_test_options(allowed_tools: Vec<String>) -> ClaudeAgentOptions {
    ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::BypassPermissions)
        .with_allowed_tools(allowed_tools)
        .with_max_turns(3)
}

/// Short timeout options for quick tests.
pub fn quick_options() -> ClaudeAgentOptions {
    ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1)
        .with_timeout_secs(30)
}
