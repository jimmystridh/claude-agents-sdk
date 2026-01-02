//! Core functionality tests (regression tests).

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{query, ClaudeAgentOptions, ClaudeClient, Message, PermissionMode};
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::integration::helpers::*;

/// Test that the one-shot query function works end-to-end.
///
/// This verifies:
/// - CLI spawns correctly
/// - Message parsing works for system, assistant, and result messages
/// - Stream closes properly after completion
#[tokio::test]
async fn test_oneshot_query_end_to_end() {
    let messages = collect_messages("What is 2+2? Answer with just the number.", default_options())
        .await
        .expect("Query failed");

    assert_message_types(&messages, &["system", "assistant", "result"]);

    let response = extract_assistant_text(&messages);
    assert!(
        response.contains('4'),
        "Response should contain '4', got: {}",
        response
    );

    let result = get_result(&messages).expect("No result message");
    assert!(!result.is_error, "Query should not have errored");
    assert!(result.num_turns >= 1, "Should have at least 1 turn");
}

/// Test that the streaming client works for multi-turn conversations.
///
/// This verifies:
/// - Streaming mode initialization works
/// - Control protocol (initialize) works correctly
/// - User message format is correct (the bug we fixed)
/// - Multiple queries in same session work
/// - Conversation context is maintained
#[tokio::test]
async fn test_streaming_client_multi_turn() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(3);

    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await.expect("Failed to connect");

    // First query
    client
        .query("What is 2+2? Answer with just the number.")
        .await
        .expect("Failed to send first query");

    let (response1, result1) = client
        .receive_response()
        .await
        .expect("Failed to receive first response");

    assert!(
        response1.contains('4'),
        "First response should contain '4', got: {}",
        response1
    );
    assert!(!result1.is_error);

    // Second query (follow-up using conversation context)
    client
        .query("Multiply that by 10. Answer with just the number.")
        .await
        .expect("Failed to send second query");

    let (response2, result2) = client
        .receive_response()
        .await
        .expect("Failed to receive second response");

    assert!(
        response2.contains("40"),
        "Second response should use context and contain '40', got: {}",
        response2
    );
    assert!(!result2.is_error);

    client.disconnect().await.expect("Failed to disconnect");
}

/// Test that the streaming client properly handles the user message format.
///
/// Regression test for bug where we sent:
///   {"type": "user", "content": "..."}
/// Instead of the correct format:
///   {"type": "user", "message": {"role": "user", "content": "..."}, ...}
#[tokio::test]
async fn test_streaming_user_message_format() {
    let options = default_options();
    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await.expect("Failed to connect");

    client
        .query("Say 'hello' and nothing else.")
        .await
        .expect("Failed to send query");

    let (response, result) = client
        .receive_response()
        .await
        .expect("Failed to receive response - this may indicate incorrect message format");

    assert!(!result.is_error, "Query should succeed");
    assert!(
        response.to_lowercase().contains("hello"),
        "Response should contain 'hello', got: {}",
        response
    );

    client.disconnect().await.expect("Failed to disconnect");
}

/// Test that stream properly closes after CLI process exits.
///
/// Regression test for bug where stream would hang because message_tx
/// was cloned instead of moved to the reader task.
#[tokio::test]
async fn test_stream_closes_after_completion() {
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        let mut stream = query("Say 'done'.", Some(default_options()), None)
            .await
            .expect("Failed to start query");

        let mut count = 0;
        while let Some(msg) = stream.next().await {
            msg.expect("Error in stream");
            count += 1;
        }
        count
    })
    .await;

    let count = result.expect("Stream should close within timeout, not hang forever");
    assert!(
        count >= 3,
        "Should receive at least 3 messages (system, assistant, result), got: {}",
        count
    );
}

/// Test that non-streaming mode uses correct stdin handling.
///
/// Regression test for bug where Stdio::piped() was used for stdin
/// in --print mode, causing the CLI to wait for input.
#[tokio::test]
async fn test_nonstreaming_stdin_handling() {
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        collect_messages("Say 'ok'.", default_options()).await
    })
    .await;

    let messages = result
        .expect("Query should complete within timeout")
        .expect("Query failed");

    assert!(
        get_result(&messages).is_some(),
        "Should receive result message"
    );
}
