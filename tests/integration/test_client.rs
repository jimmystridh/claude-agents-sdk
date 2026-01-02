//! ClaudeClient and ClaudeClientBuilder tests.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{
    ClaudeAgentOptions, ClaudeClient, ClaudeClientBuilder, Message, PermissionMode,
    PermissionResult,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_stream::StreamExt;

use crate::integration::helpers::*;

/// Test that ClaudeClientBuilder API works correctly.
#[tokio::test]
async fn test_client_builder_api() {
    let mut client = ClaudeClientBuilder::new()
        .max_turns(2)
        .permission_mode(PermissionMode::Default)
        .build();

    client.connect().await.expect("Failed to connect");

    client
        .query("What is 3+3? Answer with just the number.")
        .await
        .expect("Failed to query");

    let (response, result) = client
        .receive_response()
        .await
        .expect("Failed to receive response");

    assert!(!result.is_error);
    assert!(
        response.contains('6'),
        "Response should contain '6', got: {}",
        response
    );

    client.disconnect().await.expect("Failed to disconnect");
}

/// Test ClaudeClientBuilder with tool permission callback.
#[tokio::test]
async fn test_builder_with_can_use_tool_callback() {
    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();

    let mut client = ClaudeClientBuilder::new()
        .max_turns(3)
        .permission_mode(PermissionMode::Default)
        .can_use_tool(move |tool_name, _input, _context| {
            let called = callback_called_clone.clone();
            async move {
                called.store(true, Ordering::SeqCst);
                if tool_name == "Read" {
                    PermissionResult::allow()
                } else {
                    PermissionResult::deny_with_message("Only Read tool is allowed")
                }
            }
        })
        .build();

    client.connect().await.expect("Failed to connect");

    // Query that doesn't need tools
    client
        .query("What is 5+5? Answer with just the number.")
        .await
        .expect("Failed to query");

    let (response, result) = client
        .receive_response()
        .await
        .expect("Failed to receive response");

    assert!(!result.is_error);
    assert!(
        response.contains("10"),
        "Response should contain '10', got: {}",
        response
    );

    client.disconnect().await.expect("Failed to disconnect");
}

/// Test connect and disconnect without sending queries.
#[tokio::test]
async fn test_connect_disconnect_no_queries() {
    let options = ClaudeAgentOptions::new().with_permission_mode(PermissionMode::Default);

    let mut client = ClaudeClient::new(Some(options), None);

    client.connect().await.expect("Failed to connect");
    client.disconnect().await.expect("Failed to disconnect");
}

/// Test receive_messages() streaming API.
#[tokio::test]
async fn test_receive_messages_streaming() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await.expect("Failed to connect");

    client
        .query("Count from 1 to 3.")
        .await
        .expect("Failed to query");

    let mut message_types = Vec::new();
    while let Some(msg) = client.receive_messages().next().await {
        let msg = msg.expect("Error in stream");
        message_types.push(match &msg {
            Message::System(_) => "system",
            Message::Assistant(_) => "assistant",
            Message::User(_) => "user",
            Message::Result(_) => "result",
            Message::StreamEvent(_) => "stream_event",
        });
        if matches!(msg, Message::Result(_)) {
            break;
        }
    }

    assert!(
        message_types.contains(&"assistant"),
        "Should receive assistant messages"
    );
    assert!(
        message_types.contains(&"result"),
        "Should receive result message"
    );

    client.disconnect().await.expect("Failed to disconnect");
}

/// Test partial messages option works without breaking.
#[tokio::test]
async fn test_partial_messages_option() {
    let options = ClaudeAgentOptions::new()
        .with_partial_messages()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await.expect("Failed to connect");

    client
        .query("Count from 1 to 5.")
        .await
        .expect("Failed to send query");

    let mut stream_event_count = 0;
    let mut got_result = false;

    while let Some(msg) = client.receive_messages().next().await {
        match msg.expect("Error in stream") {
            Message::StreamEvent(_) => stream_event_count += 1,
            Message::Result(result) => {
                got_result = true;
                assert!(!result.is_error);
                break;
            }
            _ => {}
        }
    }

    assert!(got_result, "Should receive result");
    // Stream events may or may not appear depending on response speed
    if stream_event_count > 0 {
        eprintln!("Received {} stream events", stream_event_count);
    }

    client.disconnect().await.expect("Failed to disconnect");
}

/// Test specifying a model.
#[tokio::test]
async fn test_model_selection() {
    let options = ClaudeAgentOptions::new()
        .with_model("claude-sonnet-4-5")
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let messages = collect_messages("Say 'model test'.", options)
        .await
        .expect("Query failed");

    let result = get_result(&messages).expect("Should have result");
    assert!(!result.is_error, "Query with explicit model should succeed");
}

/// Test max_turns limits conversation.
#[tokio::test]
async fn test_max_turns_limit() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let messages = collect_messages("What is 2+2?", options)
        .await
        .expect("Query failed");

    let result = get_result(&messages).expect("Should have result");
    assert!(
        result.num_turns <= 1,
        "Should respect max_turns=1, got {} turns",
        result.num_turns
    );
}
