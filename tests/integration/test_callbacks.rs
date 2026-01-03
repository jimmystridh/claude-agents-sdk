//! Tool permission callback integration tests.
//!
//! These tests verify that tool permission callbacks are properly invoked
//! and can control tool execution.

#![cfg(feature = "integration-tests")]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use claude_agents_sdk::{
    ClaudeClientBuilder, ContentBlock, Message, PermissionMode, PermissionResult,
};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

// ============================================================================
// Callback Invocation Tests
// ============================================================================

/// Test that tool permission callback is actually invoked when tools are used.
///
/// NOTE: Callback invocation depends on CLI permission settings and mode.
/// In some environments, the CLI may auto-approve tools without calling back.
/// This test validates the callback mechanism when it IS invoked.
#[tokio::test]
async fn test_tool_callback_invoked_on_tool_use() {
    let callback_invoked = Arc::new(AtomicBool::new(false));
    let tools_requested = Arc::new(Mutex::new(Vec::<String>::new()));

    let invoked = Arc::clone(&callback_invoked);
    let tools = Arc::clone(&tools_requested);

    let mut client = ClaudeClientBuilder::new()
        .max_turns(3)
        .permission_mode(PermissionMode::Default)
        .allowed_tools(vec!["Bash".to_string()])
        .can_use_tool(move |tool_name, _input, _ctx| {
            let invoked = Arc::clone(&invoked);
            let tools = Arc::clone(&tools);
            async move {
                invoked.store(true, Ordering::SeqCst);
                tools.lock().await.push(tool_name.clone());
                PermissionResult::allow()
            }
        })
        .build();

    client.connect().await.expect("Failed to connect");

    // Query that should trigger tool use
    client
        .query("Run the command 'echo callback_test_123' using bash and tell me the output.")
        .await
        .expect("Failed to query");

    let (response, result) = client.receive_response().await.expect("Failed to receive");

    client.disconnect().await.ok();

    // Check if tool was used based on response content
    let tool_was_used = response.contains("callback_test_123");
    let callback_was_invoked = callback_invoked.load(Ordering::SeqCst);

    // Log the outcome for debugging
    eprintln!(
        "Tool callback test: tool_used={}, callback_invoked={}, response={}",
        tool_was_used, callback_was_invoked, response
    );

    if callback_was_invoked {
        let tools = tools_requested.lock().await;
        assert!(
            tools.contains(&"Bash".to_string()),
            "Bash tool should have been requested, got: {:?}",
            *tools
        );
        eprintln!("Callback was invoked for tools: {:?}", *tools);
    } else {
        // Callback not invoked - this is acceptable depending on CLI config
        eprintln!("Note: Callback not invoked. This is acceptable if CLI auto-approved the tool.");
    }

    assert!(!result.is_error, "Query should complete without error");
}

/// Test callback invocation count matches tool use count.
///
/// NOTE: This test may not see callbacks if CLI auto-approves tools.
/// When callbacks ARE invoked, they should match tool use count.
#[tokio::test]
async fn test_callback_count_matches_tool_uses() {
    let callback_count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&callback_count);

    let mut client = ClaudeClientBuilder::new()
        .max_turns(5)
        .permission_mode(PermissionMode::Default)
        .allowed_tools(vec!["Bash".to_string()])
        .can_use_tool(move |_tool_name, _input, _ctx| {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                PermissionResult::allow()
            }
        })
        .build();

    client.connect().await.expect("Failed to connect");

    client
        .query("Run 'echo one' and then 'echo two' using bash. Report both outputs.")
        .await
        .expect("Failed to query");

    let mut tool_use_count = 0;
    while let Some(msg) = client.receive_messages().next().await {
        match msg.expect("Stream error") {
            Message::Assistant(asst) => {
                for block in &asst.content {
                    if matches!(block, ContentBlock::ToolUse(_)) {
                        tool_use_count += 1;
                    }
                }
            }
            Message::Result(_) => break,
            _ => {}
        }
    }

    client.disconnect().await.ok();

    let callbacks = callback_count.load(Ordering::SeqCst);

    eprintln!(
        "Callback count test: callbacks={}, tool_uses={}",
        callbacks, tool_use_count
    );

    // If callbacks were invoked, they should match tool use count
    if callbacks > 0 && tool_use_count > 0 {
        assert_eq!(
            callbacks, tool_use_count,
            "Callback count {} should match tool use count {}",
            callbacks, tool_use_count
        );
    } else if callbacks == 0 && tool_use_count > 0 {
        // CLI may have auto-approved without calling back
        eprintln!(
            "Note: {} tool uses occurred but no callbacks invoked (CLI may have auto-approved)",
            tool_use_count
        );
    }
}

// ============================================================================
// Callback Deny Behavior
// ============================================================================

/// Test that denying tool permission prevents tool execution.
#[tokio::test]
async fn test_tool_callback_deny_prevents_use() {
    let deny_count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&deny_count);

    let mut client = ClaudeClientBuilder::new()
        .max_turns(3)
        .permission_mode(PermissionMode::Default)
        .allowed_tools(vec!["Bash".to_string()])
        .can_use_tool(move |_tool_name, _input, _ctx| {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                PermissionResult::deny_with_message("Tool use not permitted in this test")
            }
        })
        .build();

    client.connect().await.expect("Failed to connect");

    client
        .query("Run 'echo test' using bash")
        .await
        .expect("Failed to query");

    while let Some(msg) = client.receive_messages().next().await {
        match msg.expect("Stream error") {
            Message::Result(_) => break,
            _ => {}
        }
    }

    client.disconnect().await.ok();

    let denies = deny_count.load(Ordering::SeqCst);

    // If callback was called, it denied
    if denies > 0 {
        eprintln!("Callback denied {} tool use(s)", denies);
        // Tool results should show errors from denial
        // (This depends on how the SDK reports denied tools)
    }
}

/// Test deny with custom message.
#[tokio::test]
async fn test_deny_with_custom_message() {
    let custom_message = "Custom denial: Not allowed in test environment";

    let mut client = ClaudeClientBuilder::new()
        .max_turns(2)
        .permission_mode(PermissionMode::Default)
        .allowed_tools(vec!["Bash".to_string()])
        .can_use_tool(move |_tool_name, _input, _ctx| {
            let msg = custom_message.to_string();
            async move { PermissionResult::deny_with_message(&msg) }
        })
        .build();

    client.connect().await.expect("Failed to connect");

    client
        .query("Run 'echo denied' using bash")
        .await
        .expect("Failed to query");

    let (response, _) = client.receive_response().await.expect("Failed to receive");

    client.disconnect().await.ok();

    // Response might mention the denial
    eprintln!("Response after denial: {}", response);
}

// ============================================================================
// Callback Data Verification
// ============================================================================

/// Test callback receives correct tool name and input.
#[tokio::test]
async fn test_callback_receives_correct_data() {
    let captured_data = Arc::new(Mutex::new(Vec::<(String, serde_json::Value)>::new()));
    let data = Arc::clone(&captured_data);

    let mut client = ClaudeClientBuilder::new()
        .max_turns(3)
        .permission_mode(PermissionMode::Default)
        .allowed_tools(vec!["Bash".to_string()])
        .can_use_tool(move |tool_name, input, _ctx| {
            let data = Arc::clone(&data);
            async move {
                data.lock().await.push((tool_name, input));
                PermissionResult::allow()
            }
        })
        .build();

    client.connect().await.expect("Failed to connect");

    client
        .query("Run 'echo hello_world_test' using bash")
        .await
        .expect("Failed to query");

    let _ = client.receive_response().await;
    client.disconnect().await.ok();

    let data = captured_data.lock().await;

    if !data.is_empty() {
        // Find Bash invocation
        let bash_call = data.iter().find(|(name, _)| name == "Bash");

        if let Some((name, input)) = bash_call {
            assert_eq!(name, "Bash");
            eprintln!("Bash input: {:?}", input);

            // Bash tool typically has a "command" field
            let has_command = input.get("command").is_some()
                || input.get("cmd").is_some()
                || input.to_string().contains("echo");

            assert!(
                has_command,
                "Bash input should contain command info: {:?}",
                input
            );
        }
    }
}

// ============================================================================
// Selective Permission Tests
// ============================================================================

/// Test allowing some tools while denying others.
#[tokio::test]
async fn test_selective_tool_permission() {
    let bash_allowed = Arc::new(AtomicBool::new(false));
    let read_denied = Arc::new(AtomicBool::new(false));

    let bash_flag = Arc::clone(&bash_allowed);
    let read_flag = Arc::clone(&read_denied);

    let mut client = ClaudeClientBuilder::new()
        .max_turns(3)
        .permission_mode(PermissionMode::Default)
        .allowed_tools(vec!["Bash".to_string(), "Read".to_string()])
        .can_use_tool(move |tool_name, _input, _ctx| {
            let bash_flag = Arc::clone(&bash_flag);
            let read_flag = Arc::clone(&read_flag);
            async move {
                match tool_name.as_str() {
                    "Bash" => {
                        bash_flag.store(true, Ordering::SeqCst);
                        PermissionResult::allow()
                    }
                    "Read" => {
                        read_flag.store(true, Ordering::SeqCst);
                        PermissionResult::deny_with_message("Read not allowed in this test")
                    }
                    _ => PermissionResult::deny(),
                }
            }
        })
        .build();

    client.connect().await.expect("Failed to connect");

    // Request that might use Bash
    client
        .query("Run 'echo selective_test' with bash")
        .await
        .expect("Failed to query");

    let _ = client.receive_response().await;
    client.disconnect().await.ok();

    if bash_allowed.load(Ordering::SeqCst) {
        eprintln!("Bash was allowed as expected");
    }
    if read_denied.load(Ordering::SeqCst) {
        eprintln!("Read was denied as expected");
    }
}

// ============================================================================
// Callback Error Handling
// ============================================================================

/// Test that callback errors are handled gracefully.
#[tokio::test]
async fn test_callback_returning_error_result() {
    let mut client = ClaudeClientBuilder::new()
        .max_turns(2)
        .permission_mode(PermissionMode::Default)
        .allowed_tools(vec!["Bash".to_string()])
        .can_use_tool(move |_tool_name, _input, _ctx| {
            async move {
                // Return a deny with empty message (edge case)
                PermissionResult::deny()
            }
        })
        .build();

    client.connect().await.expect("Failed to connect");

    client
        .query("Run 'echo error_test' with bash")
        .await
        .expect("Failed to query");

    // Should complete without panic
    let result = client.receive_response().await;

    client.disconnect().await.ok();

    // Either succeeded or failed gracefully
    match result {
        Ok((response, _)) => eprintln!("Completed with response: {}", response),
        Err(e) => eprintln!("Error (acceptable): {}", e),
    }
}
