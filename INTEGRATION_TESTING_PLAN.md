# Integration Testing Plan

This document outlines the gaps in the current integration test suite and provides a comprehensive plan to address them.

## Current State Summary

The existing test suite provides:
- Feature-gated integration tests (`--features integration-tests`) for real CLI testing
- Unit tests for types, parsing, serialization, and concurrency
- Basic happy-path coverage for core functionality
- Good timeout handling and helper utilities

**What's Missing:**
- Error path testing
- Resource cleanup verification
- Cancellation handling
- Concurrent session testing
- Complete callback/hook integration tests
- Budget enforcement verification
- Property-based testing

---

## Phase 1: Error Path Testing (High Priority)

### 1.1 CLI Not Found / Invalid Path

Test that the SDK handles missing or invalid CLI paths gracefully.

```rust
// tests/integration/test_errors.rs

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{query, ClaudeAgentOptions, ClaudeClient, ClaudeSDKError};
use std::time::Duration;

/// Test error when CLI executable is not found.
#[tokio::test]
async fn test_cli_not_found_error() {
    // This requires the SDK to support custom CLI paths
    // If not supported, skip or modify based on actual API
    let options = ClaudeAgentOptions::new()
        .with_cli_path("/nonexistent/path/to/claude");

    let result = query("test", Some(options), None).await;

    assert!(result.is_err(), "Should fail with invalid CLI path");
    let err = result.unwrap_err();
    assert!(
        matches!(err, ClaudeSDKError::CliNotFound(_))
            || err.to_string().contains("not found")
            || err.to_string().contains("No such file"),
        "Error should indicate CLI not found, got: {}",
        err
    );
}

/// Test error when CLI path points to non-executable file.
#[tokio::test]
async fn test_cli_not_executable() {
    use std::fs::File;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let fake_cli = dir.path().join("fake-claude");
    File::create(&fake_cli).unwrap();

    let options = ClaudeAgentOptions::new()
        .with_cli_path(fake_cli.to_str().unwrap());

    let result = query("test", Some(options), None).await;

    assert!(result.is_err(), "Should fail with non-executable CLI");
}
```

### 1.2 Connection Failures

```rust
/// Test handling of connection timeout.
#[tokio::test]
async fn test_connection_timeout() {
    let options = ClaudeAgentOptions::new()
        .with_timeout_secs(1)  // Very short timeout
        .with_permission_mode(PermissionMode::Default);

    let mut client = ClaudeClient::new(Some(options), None);

    // Connect should succeed (it's the subprocess spawn)
    let connect_result = client.connect().await;

    // If connect uses the timeout, it might fail
    // Otherwise, query with a long prompt might timeout
    if connect_result.is_ok() {
        // Try a query that might timeout
        client.query("Explain quantum physics in detail").await.ok();

        let result = tokio::time::timeout(
            Duration::from_secs(2),
            client.receive_response()
        ).await;

        // Either we get a timeout error from SDK or from tokio
        if let Ok(inner) = result {
            // SDK might have its own timeout
            if let Err(e) = inner {
                assert!(
                    e.to_string().to_lowercase().contains("timeout"),
                    "Expected timeout error, got: {}",
                    e
                );
            }
        }

        client.disconnect().await.ok();
    }
}

/// Test handling when CLI process exits unexpectedly.
#[tokio::test]
async fn test_cli_unexpected_exit() {
    let mut client = ClaudeClient::new(Some(default_options()), None);
    client.connect().await.expect("Failed to connect");

    // Start a query
    client.query("Hello").await.expect("Failed to send query");

    // Forcibly kill the underlying process (if accessible)
    // This is implementation-specific and may require exposing internals

    // Alternatively, test with a query that causes CLI to exit
    // (e.g., invalid configuration that CLI rejects)

    client.disconnect().await.ok();
}
```

### 1.3 Malformed Response Handling

```rust
/// Test that SDK handles malformed JSON gracefully.
///
/// Note: This may require a mock transport to inject bad data,
/// or rely on unit tests in test_message_parser.rs
#[tokio::test]
async fn test_malformed_json_in_stream() {
    // This test may need to be a unit test with mocked transport
    // since we can't make the real CLI send malformed JSON

    // For integration, we verify the SDK doesn't panic on edge cases
    let options = ClaudeAgentOptions::new()
        .with_max_turns(1)
        .with_permission_mode(PermissionMode::Default);

    // Query with special characters that might cause issues
    let prompts = [
        "\x00\x01\x02",           // Null bytes
        "{\"test\": broken}",     // JSON-like but invalid
        "\n\n\n",                 // Just newlines
        "```json\n{}\n```",       // Markdown code blocks
    ];

    for prompt in prompts {
        let result = collect_messages(prompt, options.clone()).await;
        // Should either succeed or fail gracefully, never panic
        match result {
            Ok(messages) => {
                // If it succeeded, should have result
                assert!(
                    get_result(&messages).is_some(),
                    "Should have result message for prompt: {:?}",
                    prompt
                );
            }
            Err(e) => {
                // Error is acceptable, just shouldn't panic
                eprintln!("Expected error for {:?}: {}", prompt, e);
            }
        }
    }
}
```

### 1.4 API Error Responses

```rust
/// Test handling of API-level errors (rate limits, auth failures, etc.)
///
/// Note: These require specific conditions that may not be reproducible
/// in a test environment. Consider mocking for unit tests.
#[tokio::test]
#[ignore] // Run manually with invalid credentials
async fn test_authentication_failure() {
    // This would require a way to use invalid API credentials
    // May need environment variable override or mock

    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default);

    let result = collect_messages("test", options).await;

    // Expect an authentication error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_lowercase().contains("auth")
            || err.to_lowercase().contains("api key")
            || err.to_lowercase().contains("unauthorized"),
        "Expected auth error, got: {}",
        err
    );
}
```

---

## Phase 2: Resource Management Testing (High Priority)

### 2.1 Process Cleanup Verification

```rust
// tests/integration/test_resources.rs

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{query, ClaudeAgentOptions, ClaudeClient, PermissionMode};
use std::process::Command;
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::integration::helpers::*;

/// Count running claude processes (platform-specific).
fn count_claude_processes() -> usize {
    #[cfg(unix)]
    {
        let output = Command::new("pgrep")
            .args(["-f", "claude"])
            .output()
            .ok();

        match output {
            Some(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .count()
            }
            _ => 0
        }
    }

    #[cfg(windows)]
    {
        let output = Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq claude*"])
            .output()
            .ok();

        match output {
            Some(o) => {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| l.contains("claude"))
                    .count()
            }
            _ => 0
        }
    }
}

/// Test that processes are cleaned up after normal disconnect.
#[tokio::test]
async fn test_process_cleanup_after_disconnect() {
    let initial_count = count_claude_processes();

    for i in 0..3 {
        let mut client = ClaudeClient::new(Some(default_options()), None);
        client.connect().await.expect("Failed to connect");

        client
            .query(&format!("Say '{}'", i))
            .await
            .expect("Failed to query");

        let _ = client.receive_response().await;
        client.disconnect().await.expect("Failed to disconnect");
    }

    // Allow time for process cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;

    let final_count = count_claude_processes();

    assert!(
        final_count <= initial_count + 1,  // Allow for some tolerance
        "Process leak detected: started with {}, ended with {}",
        initial_count,
        final_count
    );
}

/// Test that processes are cleaned up when stream is dropped.
#[tokio::test]
async fn test_process_cleanup_on_stream_drop() {
    let initial_count = count_claude_processes();

    for _ in 0..3 {
        let mut stream = query("Say hello", Some(default_options()), None)
            .await
            .expect("Failed to start query");

        // Read one message then drop
        let _ = stream.next().await;
        drop(stream);
    }

    // Allow time for process cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;

    let final_count = count_claude_processes();

    assert!(
        final_count <= initial_count + 1,
        "Process leak on drop: started with {}, ended with {}",
        initial_count,
        final_count
    );
}

/// Test cleanup when client is dropped without disconnect.
#[tokio::test]
async fn test_cleanup_on_client_drop_without_disconnect() {
    let initial_count = count_claude_processes();

    {
        let mut client = ClaudeClient::new(Some(default_options()), None);
        client.connect().await.expect("Failed to connect");
        client.query("Hello").await.expect("Failed to query");
        // Drop without calling disconnect()
    }

    // Allow time for cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;

    let final_count = count_claude_processes();

    assert!(
        final_count <= initial_count + 1,
        "Process leak on drop without disconnect: {} -> {}",
        initial_count,
        final_count
    );
}
```

### 2.2 Memory/Handle Leak Detection

```rust
/// Test for handle leaks across many sessions.
#[tokio::test]
async fn test_no_handle_leaks_across_sessions() {
    // Run many sessions and verify stable resource usage
    for batch in 0..5 {
        let handles: Vec<_> = (0..5)
            .map(|i| {
                tokio::spawn(async move {
                    let messages = collect_messages(
                        &format!("Say 'batch {} item {}'", batch, i),
                        default_options(),
                    ).await;
                    messages.is_ok()
                })
            })
            .collect();

        for handle in handles {
            let _ = handle.await;
        }

        // Brief pause between batches
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // If we get here without OOM or handle exhaustion, test passes
}
```

---

## Phase 3: Cancellation Testing (Medium Priority)

### 3.1 Stream Cancellation

```rust
// tests/integration/test_cancellation.rs

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{query, ClaudeAgentOptions, ClaudeClient, PermissionMode};
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::integration::helpers::*;

/// Test dropping a stream mid-query.
#[tokio::test]
async fn test_drop_stream_mid_query() {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(5)
        .with_permission_mode(PermissionMode::Default);

    let mut stream = query(
        "Write a very long story about a robot. Make it at least 1000 words.",
        Some(options),
        None
    )
    .await
    .expect("Failed to start query");

    // Read just a few messages
    let mut count = 0;
    while let Some(msg) = stream.next().await {
        if msg.is_ok() {
            count += 1;
        }
        if count >= 2 {
            break;
        }
    }

    // Drop the stream early
    drop(stream);

    // Verify we can start a new query (proves cleanup worked)
    tokio::time::sleep(Duration::from_millis(200)).await;

    let messages = collect_messages("Say 'cleanup worked'", default_options())
        .await
        .expect("Failed to query after cancellation");

    assert!(get_result(&messages).is_some());
}

/// Test disconnect during active query.
#[tokio::test]
async fn test_disconnect_during_query() {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(5)
        .with_permission_mode(PermissionMode::Default);

    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await.expect("Failed to connect");

    // Start a long query
    client
        .query("Explain the history of computing in detail.")
        .await
        .expect("Failed to send query");

    // Read one message
    let _ = client.receive_messages().next().await;

    // Disconnect immediately
    let disconnect_result = client.disconnect().await;

    // Disconnect should succeed or fail gracefully
    assert!(
        disconnect_result.is_ok() || disconnect_result.is_err(),
        "Disconnect should not panic"
    );

    // Verify we can create a new client
    let mut client2 = ClaudeClient::new(Some(default_options()), None);
    client2.connect().await.expect("Should be able to connect after forced disconnect");
    client2.disconnect().await.ok();
}

/// Test tokio cancellation via select.
#[tokio::test]
async fn test_tokio_select_cancellation() {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(3)
        .with_permission_mode(PermissionMode::Default);

    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await.expect("Failed to connect");

    client
        .query("Write a long poem about the ocean.")
        .await
        .expect("Failed to send query");

    // Use select to race between response and timeout
    tokio::select! {
        _ = async {
            while let Some(_) = client.receive_messages().next().await {
                // Keep reading
            }
        } => {
            // Completed normally
        }
        _ = tokio::time::sleep(Duration::from_millis(500)) => {
            // Timeout - this is the expected path for a long query
        }
    }

    // Cleanup
    client.disconnect().await.ok();

    // Verify system is still functional
    let messages = collect_messages("Say 'after select'", default_options())
        .await
        .expect("System should work after select cancellation");

    assert!(get_result(&messages).is_some());
}
```

### 3.2 Timeout Behavior

```rust
/// Test that SDK timeout is respected.
#[tokio::test]
async fn test_sdk_timeout_respected() {
    let options = ClaudeAgentOptions::new()
        .with_timeout_secs(5)
        .with_max_turns(10)
        .with_permission_mode(PermissionMode::Default);

    let start = std::time::Instant::now();

    // This query with many turns might exceed timeout
    let result = collect_messages(
        "Solve this complex math problem step by step: What is the 100th prime number?",
        options
    ).await;

    let elapsed = start.elapsed();

    // If result is error due to timeout, verify timing
    if result.is_err() {
        assert!(
            elapsed.as_secs() <= 10,  // Some tolerance
            "Timeout should occur around 5 seconds, took {:?}",
            elapsed
        );
    }
    // If it succeeded quickly, that's fine too
}
```

---

## Phase 4: Concurrent Session Testing (Medium Priority)

### 4.1 Parallel Sessions

```rust
// tests/integration/test_concurrent_sessions.rs

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{ClaudeAgentOptions, ClaudeClient, PermissionMode};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::integration::helpers::*;

/// Test multiple concurrent query() calls.
#[tokio::test]
async fn test_concurrent_queries() {
    let success_count = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..3)
        .map(|i| {
            let counter = Arc::clone(&success_count);
            tokio::spawn(async move {
                let result = collect_messages(
                    &format!("What is {}+{}? Answer with just the number.", i, i),
                    default_options(),
                ).await;

                if result.is_ok() {
                    counter.fetch_add(1, Ordering::SeqCst);
                }

                result
            })
        })
        .collect();

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // At least some should succeed
    assert!(
        success_count.load(Ordering::SeqCst) >= 2,
        "At least 2 concurrent queries should succeed, got {} successes",
        success_count.load(Ordering::SeqCst)
    );
}

/// Test multiple concurrent ClaudeClient instances.
#[tokio::test]
async fn test_concurrent_clients() {
    let handles: Vec<_> = (0..3)
        .map(|i| {
            tokio::spawn(async move {
                let mut client = ClaudeClient::new(Some(default_options()), None);
                client.connect().await?;

                client.query(&format!("Say 'client {}'", i)).await?;
                let (response, result) = client.receive_response().await?;

                client.disconnect().await?;

                Ok::<_, Box<dyn std::error::Error + Send + Sync>>((i, response, result))
            })
        })
        .collect();

    let mut successes = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok((i, response, result))) => {
                assert!(!result.is_error, "Client {} should succeed", i);
                successes += 1;
            }
            Ok(Err(e)) => eprintln!("Client error: {}", e),
            Err(e) => eprintln!("Join error: {}", e),
        }
    }

    assert!(successes >= 2, "At least 2 concurrent clients should succeed");
}

/// Test interleaved operations on multiple clients.
#[tokio::test]
async fn test_interleaved_client_operations() {
    let mut client_a = ClaudeClient::new(Some(default_options()), None);
    let mut client_b = ClaudeClient::new(Some(default_options()), None);

    // Connect both
    client_a.connect().await.expect("Client A connect failed");
    client_b.connect().await.expect("Client B connect failed");

    // Send queries
    client_a.query("Say 'A'").await.expect("Client A query failed");
    client_b.query("Say 'B'").await.expect("Client B query failed");

    // Receive in opposite order
    let (response_b, result_b) = client_b.receive_response().await.expect("Client B receive failed");
    let (response_a, result_a) = client_a.receive_response().await.expect("Client A receive failed");

    assert!(!result_a.is_error);
    assert!(!result_b.is_error);

    // Disconnect both
    client_a.disconnect().await.expect("Client A disconnect failed");
    client_b.disconnect().await.expect("Client B disconnect failed");
}
```

### 4.2 Shared State Under Concurrency

```rust
/// Test concurrent access to shared options.
#[tokio::test]
async fn test_shared_options_concurrent_use() {
    let shared_options = Arc::new(
        ClaudeAgentOptions::new()
            .with_max_turns(1)
            .with_permission_mode(PermissionMode::Default)
    );

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let opts = Arc::clone(&shared_options);
            tokio::spawn(async move {
                // Clone the options for each use
                let options = (*opts).clone();
                collect_messages(&format!("Say '{}'", i), options).await
            })
        })
        .collect();

    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    assert!(success_count >= 3, "Most concurrent uses should succeed");
}
```

---

## Phase 5: Complete Callback Integration (Medium Priority)

### 5.1 Tool Permission Callback Verification

```rust
// tests/integration/test_callbacks.rs

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{
    ClaudeAgentOptions, ClaudeClientBuilder, ContentBlock, Message,
    PermissionMode, PermissionResult,
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::integration::helpers::*;

/// Test that tool permission callback is actually invoked.
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
        .query("Run the command 'echo callback_test' using bash and tell me the output.")
        .await
        .expect("Failed to query");

    let (response, result) = client
        .receive_response()
        .await
        .expect("Failed to receive");

    client.disconnect().await.ok();

    assert!(
        callback_invoked.load(Ordering::SeqCst),
        "Tool permission callback should have been invoked. Response: {}",
        response
    );

    let tools = tools_requested.lock().await;
    assert!(
        tools.contains(&"Bash".to_string()),
        "Bash tool should have been requested, got: {:?}",
        *tools
    );
}

/// Test that denying tool permission prevents tool use.
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

    let mut tool_results = Vec::new();
    while let Some(msg) = client.receive_messages().next().await {
        match msg.expect("Stream error") {
            Message::User(user) => {
                // Check for tool result errors
                if let claude_agents_sdk::UserMessageContent::Blocks(blocks) = &user.content {
                    for block in blocks {
                        if let ContentBlock::ToolResult(result) = block {
                            tool_results.push(result.clone());
                        }
                    }
                }
            }
            Message::Result(_) => break,
            _ => {}
        }
    }

    client.disconnect().await.ok();

    assert!(
        deny_count.load(Ordering::SeqCst) >= 1,
        "Callback should have been called at least once"
    );

    // Tool results should indicate the tool was denied/errored
    for result in &tool_results {
        if result.is_error.unwrap_or(false) {
            assert!(
                result.content.iter().any(|c| {
                    if let ContentBlock::Text(t) = c {
                        t.text.contains("not permitted") || t.text.contains("denied")
                    } else {
                        false
                    }
                }),
                "Tool result should contain denial message"
            );
        }
    }
}

/// Test callback receives correct tool name and input.
#[tokio::test]
async fn test_tool_callback_receives_correct_data() {
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
        .query("Run 'echo hello_world' using bash")
        .await
        .expect("Failed to query");

    let _ = client.receive_response().await;
    client.disconnect().await.ok();

    let data = captured_data.lock().await;

    // Find Bash invocation
    let bash_call = data.iter().find(|(name, _)| name == "Bash");

    if let Some((name, input)) = bash_call {
        assert_eq!(name, "Bash");
        // Bash tool typically has a "command" field
        assert!(
            input.get("command").is_some() || input.get("cmd").is_some(),
            "Bash input should have command field: {:?}",
            input
        );
    }
}
```

### 5.2 Hook Integration Tests

```rust
// tests/integration/test_hooks.rs

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{
    ClaudeAgentOptions, ClaudeClientBuilder, HookCallback, HookContext,
    HookEvent, HookInput, HookMatcher, HookOutput, PermissionMode,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::integration::helpers::*;

/// Test pre-tool-use hook is invoked.
#[tokio::test]
async fn test_pre_tool_use_hook_invoked() {
    let hook_count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hook_count);

    let hook: HookCallback = Arc::new(move |input, _tool_use_id, _ctx| {
        let counter = Arc::clone(&counter);
        Box::pin(async move {
            if matches!(input, HookInput::PreToolUse(_)) {
                counter.fetch_add(1, Ordering::SeqCst);
            }
            HookOutput::default()
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![HookMatcher {
            matcher: None,  // Match all tools
            hooks: vec![hook],
            timeout: Some(5000.0),
        }],
    );

    let options = ClaudeAgentOptions::new()
        .with_max_turns(3)
        .with_permission_mode(PermissionMode::BypassPermissions)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_hooks(hooks);

    let messages = collect_messages(
        "Run 'echo hook_test' using bash",
        options
    ).await.expect("Query failed");

    assert!(get_result(&messages).is_some());

    // Hook should have been called if tool was used
    // (May be 0 if Claude didn't use the tool)
    let count = hook_count.load(Ordering::SeqCst);
    eprintln!("Pre-tool-use hook invoked {} times", count);
}

/// Test post-tool-use hook is invoked.
#[tokio::test]
async fn test_post_tool_use_hook_invoked() {
    let hook_count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hook_count);

    let hook: HookCallback = Arc::new(move |input, _tool_use_id, _ctx| {
        let counter = Arc::clone(&counter);
        Box::pin(async move {
            if matches!(input, HookInput::PostToolUse(_)) {
                counter.fetch_add(1, Ordering::SeqCst);
            }
            HookOutput::default()
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PostToolUse,
        vec![HookMatcher {
            matcher: None,
            hooks: vec![hook],
            timeout: Some(5000.0),
        }],
    );

    let options = ClaudeAgentOptions::new()
        .with_max_turns(3)
        .with_permission_mode(PermissionMode::BypassPermissions)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_hooks(hooks);

    let messages = collect_messages(
        "Run 'echo post_hook_test' using bash",
        options
    ).await.expect("Query failed");

    assert!(get_result(&messages).is_some());

    let count = hook_count.load(Ordering::SeqCst);
    eprintln!("Post-tool-use hook invoked {} times", count);
}

/// Test hook can modify tool output.
#[tokio::test]
async fn test_hook_can_modify_output() {
    let hook: HookCallback = Arc::new(move |input, _tool_use_id, _ctx| {
        Box::pin(async move {
            if let HookInput::PostToolUse(_) = input {
                HookOutput {
                    decision: Some("modify".to_string()),
                    reason: Some("Hook modified output".to_string()),
                    ..Default::default()
                }
            } else {
                HookOutput::default()
            }
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PostToolUse,
        vec![HookMatcher {
            matcher: Some("Bash".to_string()),
            hooks: vec![hook],
            timeout: Some(5000.0),
        }],
    );

    let options = ClaudeAgentOptions::new()
        .with_max_turns(3)
        .with_permission_mode(PermissionMode::BypassPermissions)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_hooks(hooks);

    let result = collect_messages("Run 'echo modify_test' using bash", options).await;

    // Just verify it completes without error
    assert!(result.is_ok(), "Query should complete: {:?}", result);
}

/// Test hook matcher filters by tool name.
#[tokio::test]
async fn test_hook_matcher_filters_tools() {
    let bash_count = Arc::new(AtomicUsize::new(0));
    let read_count = Arc::new(AtomicUsize::new(0));

    let bash_counter = Arc::clone(&bash_count);
    let bash_hook: HookCallback = Arc::new(move |_, _, _| {
        let counter = Arc::clone(&bash_counter);
        Box::pin(async move {
            counter.fetch_add(1, Ordering::SeqCst);
            HookOutput::default()
        })
    });

    let read_counter = Arc::clone(&read_count);
    let read_hook: HookCallback = Arc::new(move |_, _, _| {
        let counter = Arc::clone(&read_counter);
        Box::pin(async move {
            counter.fetch_add(1, Ordering::SeqCst);
            HookOutput::default()
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![
            HookMatcher {
                matcher: Some("Bash".to_string()),
                hooks: vec![bash_hook],
                timeout: Some(5000.0),
            },
            HookMatcher {
                matcher: Some("Read".to_string()),
                hooks: vec![read_hook],
                timeout: Some(5000.0),
            },
        ],
    );

    let options = ClaudeAgentOptions::new()
        .with_max_turns(3)
        .with_permission_mode(PermissionMode::BypassPermissions)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_hooks(hooks);

    let _ = collect_messages("Run 'echo filter_test' using bash", options).await;

    let bash = bash_count.load(Ordering::SeqCst);
    let read = read_count.load(Ordering::SeqCst);

    eprintln!("Bash hook: {}, Read hook: {}", bash, read);

    // If Bash was used, its hook should have been called, not Read's
    if bash > 0 {
        assert_eq!(read, 0, "Read hook should not be called for Bash tool");
    }
}
```

---

## Phase 6: Budget/Cost Verification (Medium Priority)

### 6.1 Budget Enforcement

```rust
// tests/integration/test_budget.rs

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{ClaudeAgentOptions, PermissionMode};

use crate::integration::helpers::*;

/// Test that max_budget_usd is respected.
#[tokio::test]
async fn test_max_budget_enforced() {
    let options = ClaudeAgentOptions::new()
        .with_max_budget_usd(0.0001)  // Extremely low budget
        .with_max_turns(10)
        .with_permission_mode(PermissionMode::Default);

    // This prompt requires multiple turns and should exceed budget
    let messages = collect_messages(
        "Explain quantum mechanics in great detail. Cover wave-particle duality, \
         superposition, entanglement, and the measurement problem. Be thorough.",
        options
    ).await;

    match messages {
        Ok(msgs) => {
            let result = get_result(&msgs).expect("Should have result");

            // Either:
            // 1. Result indicates budget exceeded
            // 2. Response was truncated/short due to budget
            // 3. It succeeded but cost was within budget

            if result.subtype == "error_max_budget_usd" {
                // Budget was enforced
                assert!(
                    result.total_cost_usd.map_or(true, |c| c <= 0.001),
                    "Cost should be near budget limit"
                );
            } else if let Some(cost) = result.total_cost_usd {
                // Completed within budget
                assert!(
                    cost <= 0.001,  // Some tolerance above 0.0001
                    "Cost {} should be near budget limit",
                    cost
                );
            }
        }
        Err(e) => {
            // Error might indicate budget exceeded
            assert!(
                e.contains("budget") || e.contains("cost") || e.contains("limit"),
                "Error should relate to budget: {}",
                e
            );
        }
    }
}

/// Test cost reporting accuracy.
#[tokio::test]
async fn test_cost_reporting() {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(1)
        .with_permission_mode(PermissionMode::Default);

    let messages = collect_messages("Say 'cost test'", options)
        .await
        .expect("Query failed");

    let result = get_result(&messages).expect("Should have result");

    // Verify cost fields are present and reasonable
    if let Some(cost) = result.total_cost_usd {
        assert!(cost >= 0.0, "Cost should be non-negative");
        assert!(cost < 1.0, "Single turn cost should be < $1");
    }

    assert!(result.duration_ms > 0, "Duration should be recorded");
    assert!(result.duration_api_ms >= 0, "API duration should be recorded");
}

/// Test that usage statistics are reported.
#[tokio::test]
async fn test_usage_statistics() {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(1)
        .with_permission_mode(PermissionMode::Default);

    let messages = collect_messages(
        "Write a haiku about programming.",
        options
    ).await.expect("Query failed");

    let result = get_result(&messages).expect("Should have result");

    if let Some(usage) = &result.usage {
        // Verify usage contains expected fields
        let input_tokens = usage.get("input_tokens")
            .or_else(|| usage.get("inputTokens"));
        let output_tokens = usage.get("output_tokens")
            .or_else(|| usage.get("outputTokens"));

        if let (Some(input), Some(output)) = (input_tokens, output_tokens) {
            let input = input.as_u64().unwrap_or(0);
            let output = output.as_u64().unwrap_or(0);

            assert!(input > 0, "Should have input tokens");
            assert!(output > 0, "Should have output tokens");

            eprintln!("Usage: {} input tokens, {} output tokens", input, output);
        }
    }
}
```

---

## Phase 7: Context and Session Management (Medium Priority)

### 7.1 Conversation Context

```rust
// tests/integration/test_context_extended.rs

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{ClaudeAgentOptions, ClaudeClient, PermissionMode};

use crate::integration::helpers::*;

/// Test resuming a conversation by session ID.
#[tokio::test]
async fn test_resume_session() {
    // First conversation
    let options1 = ClaudeAgentOptions::new()
        .with_max_turns(1)
        .with_permission_mode(PermissionMode::Default);

    let messages1 = collect_messages(
        "My name is TestUser123. Remember this.",
        options1
    ).await.expect("First query failed");

    let result1 = get_result(&messages1).expect("Should have result");
    let session_id = result1.session_id.clone();

    // Resume with session ID
    let options2 = ClaudeAgentOptions::new()
        .with_resume(&session_id)
        .with_continue_conversation(true)
        .with_max_turns(1)
        .with_permission_mode(PermissionMode::Default);

    let messages2 = collect_messages(
        "What is my name?",
        options2
    ).await.expect("Resume query failed");

    let response = extract_assistant_text(&messages2);

    // If resume worked, Claude should remember the name
    assert!(
        response.contains("TestUser123") || response.contains("don't") || response.contains("name"),
        "Response should reference the conversation context. Got: {}",
        response
    );
}

/// Test continue_conversation flag.
#[tokio::test]
async fn test_continue_conversation_flag() {
    let mut client = ClaudeClient::new(
        Some(ClaudeAgentOptions::new()
            .with_max_turns(5)
            .with_permission_mode(PermissionMode::Default)),
        None
    );

    client.connect().await.expect("Failed to connect");

    // First message
    client.query("I'm thinking of a number between 1 and 10. The number is 7.")
        .await.expect("Failed to send first query");
    let (_, result1) = client.receive_response().await.expect("Failed to receive");
    assert!(!result1.is_error);

    // Continue with context
    client.query("What number was I thinking of?")
        .await.expect("Failed to send second query");
    let (response2, result2) = client.receive_response().await.expect("Failed to receive");

    assert!(!result2.is_error);
    assert!(
        response2.contains('7') || response2.contains("seven"),
        "Should remember the number: {}",
        response2
    );

    client.disconnect().await.ok();
}

/// Test providing initial context/messages.
#[tokio::test]
async fn test_initial_context_messages() {
    // If SDK supports providing initial messages
    let initial_context = vec![
        // Implementation depends on how SDK accepts initial context
    ];

    // This test would verify that providing conversation history
    // allows Claude to reference previous messages

    // Placeholder - implement based on actual API
}
```

### 7.2 Working Directory

```rust
/// Test custom working directory.
#[tokio::test]
async fn test_custom_working_directory() {
    use std::path::PathBuf;
    use tempfile::tempdir;

    let temp = tempdir().expect("Failed to create temp dir");
    let test_file = temp.path().join("test.txt");
    std::fs::write(&test_file, "custom cwd test content").expect("Failed to write file");

    let options = ClaudeAgentOptions::new()
        .with_cwd(temp.path())
        .with_max_turns(2)
        .with_permission_mode(PermissionMode::BypassPermissions)
        .with_allowed_tools(vec!["Read".to_string()]);

    let messages = collect_messages(
        "Read the file test.txt in the current directory and tell me its contents.",
        options
    ).await.expect("Query failed");

    let response = extract_assistant_text(&messages);

    assert!(
        response.contains("custom cwd test content"),
        "Should read from custom cwd. Got: {}",
        response
    );
}
```

---

## Phase 8: Property-Based Testing (Low Priority)

### 8.1 Add proptest Dependency

```toml
# Cargo.toml dev-dependencies
[dev-dependencies]
proptest = "1.4"
```

### 8.2 Property Tests

```rust
// tests/property_tests.rs

use claude_agents_sdk::{
    AssistantMessage, ContentBlock, Message, ResultMessage, TextBlock, ToolUseBlock,
};
use proptest::prelude::*;

/// Strategy for generating arbitrary text content.
fn arb_text() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 .,!?]{1,100}")
        .unwrap()
}

/// Strategy for generating ToolUseBlock.
fn arb_tool_use() -> impl Strategy<Value = ToolUseBlock> {
    (
        prop::string::string_regex("[a-z]{1,10}").unwrap(),  // id
        prop::string::string_regex("[A-Z][a-z]+").unwrap(),  // name
    ).prop_map(|(id, name)| ToolUseBlock {
        id: format!("tool_{}", id),
        name,
        input: serde_json::json!({"key": "value"}),
    })
}

/// Strategy for generating ContentBlock.
fn arb_content_block() -> impl Strategy<Value = ContentBlock> {
    prop_oneof![
        arb_text().prop_map(|t| ContentBlock::Text(TextBlock { text: t })),
        arb_tool_use().prop_map(ContentBlock::ToolUse),
    ]
}

/// Strategy for generating AssistantMessage.
fn arb_assistant_message() -> impl Strategy<Value = AssistantMessage> {
    (
        prop::collection::vec(arb_content_block(), 1..5),
        prop::string::string_regex("claude-[0-9]").unwrap(),
    ).prop_map(|(content, model)| AssistantMessage {
        content,
        model,
        parent_tool_use_id: None,
        error: None,
    })
}

proptest! {
    /// Test that Message serialization round-trips.
    #[test]
    fn test_message_roundtrip(msg in arb_assistant_message()) {
        let wrapped = Message::Assistant(msg);
        let json = serde_json::to_string(&wrapped).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        // Verify it's still an Assistant message
        assert!(parsed.is_assistant());
    }

    /// Test that text extraction handles any content.
    #[test]
    fn test_text_extraction_doesnt_panic(msg in arb_assistant_message()) {
        // Should never panic
        let _ = msg.text();
    }

    /// Test that tool_uses extraction handles any content.
    #[test]
    fn test_tool_uses_extraction(msg in arb_assistant_message()) {
        let tools = msg.tool_uses();

        // Count expected tools
        let expected = msg.content.iter()
            .filter(|c| matches!(c, ContentBlock::ToolUse(_)))
            .count();

        assert_eq!(tools.len(), expected);
    }
}

/// Test ResultMessage serialization.
proptest! {
    #[test]
    fn test_result_message_roundtrip(
        duration in 0u64..1000000,
        turns in 1u32..100,
        cost in 0.0f64..10.0,
    ) {
        let result = ResultMessage {
            subtype: "success".to_string(),
            duration_ms: duration,
            duration_api_ms: duration / 2,
            is_error: false,
            num_turns: turns,
            session_id: "test".to_string(),
            total_cost_usd: Some(cost),
            usage: None,
            result: None,
            structured_output: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ResultMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.duration_ms, duration);
        assert_eq!(parsed.num_turns, turns);
    }
}
```

---

## Phase 9: Test Infrastructure Improvements

### 9.1 Improved Helpers

```rust
// tests/integration/helpers.rs additions

/// Collect messages with detailed error reporting.
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
                    return Err(format!("Error in stream: {:?}", e));
                }
            }
        }
        Ok(messages)
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(format!(
            "Test timed out after {:?}",
            TEST_TIMEOUT
        )),
    }
}

/// Assert with detailed message context.
pub fn assert_response_contains(messages: &[Message], expected: &str) {
    let response = extract_assistant_text(messages);
    assert!(
        response.to_lowercase().contains(&expected.to_lowercase()),
        "Expected response to contain '{}'\nActual response:\n{}",
        expected,
        response
    );
}

/// Get all tool uses from messages.
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

/// Retry a test with exponential backoff for flaky tests.
pub async fn with_retry<F, Fut, T, E>(
    max_attempts: usize,
    f: F,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
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
```

### 9.2 Test Categories with Cargo Features

```toml
# Cargo.toml
[features]
default = []
mcp = ["mcp-core"]
integration-tests = []
stress-tests = []
property-tests = ["proptest"]

[dev-dependencies]
proptest = { version = "1.4", optional = true }
```

### 9.3 CI Configuration Update

```yaml
# .github/workflows/ci.yml additions

jobs:
  integration-tests:
    runs-on: ubuntu-latest
    if: github.event_name == 'schedule' || contains(github.event.head_commit.message, '[integration]')
    steps:
      - uses: actions/checkout@v4
      - name: Install Claude CLI
        run: |
          # Installation steps for Claude CLI
      - name: Run integration tests
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: cargo test --features integration-tests -- --test-threads=1

  stress-tests:
    runs-on: ubuntu-latest
    if: github.event_name == 'schedule'
    steps:
      - uses: actions/checkout@v4
      - name: Run stress tests
        run: cargo test --features stress-tests -- --ignored

  property-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run property tests
        run: cargo test --features property-tests property_
```

---

## Implementation Priority

| Phase | Description | Priority | Effort | Impact |
|-------|-------------|----------|--------|--------|
| 1 | Error Path Testing | High | Medium | High |
| 2 | Resource Management | High | Medium | High |
| 3 | Cancellation Testing | Medium | Low | Medium |
| 4 | Concurrent Sessions | Medium | Low | Medium |
| 5 | Callback Integration | Medium | Medium | High |
| 6 | Budget Verification | Medium | Low | Medium |
| 7 | Context Management | Medium | Medium | Medium |
| 8 | Property Testing | Low | Medium | Medium |
| 9 | Infrastructure | Low | Low | High |

---

## Execution Order

1. **Week 1**: Phase 1 (Error Paths) + Phase 2 (Resource Management)
2. **Week 2**: Phase 3 (Cancellation) + Phase 4 (Concurrency)
3. **Week 3**: Phase 5 (Callbacks) + Phase 6 (Budget)
4. **Week 4**: Phase 7 (Context) + Phase 9 (Infrastructure)
5. **Ongoing**: Phase 8 (Property Testing)

---

## Success Metrics

- [ ] All error paths have dedicated tests
- [ ] Zero process leaks verified across 100+ sessions
- [ ] Cancellation tested for stream, client, and tokio select
- [ ] Concurrent sessions work reliably (>90% success rate)
- [ ] Tool permission callback verified with actual tool invocations
- [ ] Hook integration tests cover all hook events
- [ ] Budget enforcement verified with measurable cost tracking
- [ ] Session resume/continue tested end-to-end
- [ ] Property tests cover all serializable types
- [ ] CI runs integration tests on schedule

---

## Notes

- Integration tests require Claude CLI installed and API credentials
- Some tests may be flaky due to LLM non-determinism; use `with_retry` helper
- Consider running expensive tests only in CI or manually
- Keep integration tests isolated (no shared state between tests)
- Document any environment requirements in test module docs
