//! Cancellation and cleanup tests.
//!
//! These tests verify that the SDK properly handles stream cancellation,
//! early disconnection, and tokio cancellation patterns.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{query, ClaudeAgentOptions, ClaudeClient, PermissionMode};
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::integration::helpers::*;

// ============================================================================
// Stream Cancellation
// ============================================================================

/// Test dropping a stream mid-query.
#[tokio::test]
async fn test_drop_stream_mid_query() {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(5)
        .with_permission_mode(PermissionMode::Default);

    let mut stream = query(
        "Write a story about a robot. Make it detailed with multiple paragraphs.",
        Some(options),
        None,
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

    // Brief pause for cleanup
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify we can start a new query (proves cleanup worked)
    let messages = collect_messages("Say 'cleanup worked'", default_options())
        .await
        .expect("Failed to query after cancellation");

    assert!(
        get_result(&messages).is_some(),
        "Should be able to query after stream drop"
    );
}

/// Test dropping stream immediately after creation.
#[tokio::test]
async fn test_drop_stream_immediately() {
    let options = default_options();

    let stream = query("Hello", Some(options), None)
        .await
        .expect("Failed to start query");

    // Drop immediately without reading anything
    drop(stream);

    // Brief pause
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Should still be able to query
    let messages = collect_messages("Say 'ok'", default_options())
        .await
        .expect("Failed to query after immediate drop");

    assert!(get_result(&messages).is_some());
}

// ============================================================================
// Client Disconnection
// ============================================================================

/// Test disconnect during active query.
#[tokio::test]
async fn test_disconnect_during_query() {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(5)
        .with_permission_mode(PermissionMode::Default);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await.expect("Failed to connect");

    // Start a query
    client
        .query("Explain the history of computing in great detail.")
        .await
        .expect("Failed to send query");

    // Read one message
    let _ = client.receive_messages().next().await;

    // Disconnect immediately
    let disconnect_result = client.disconnect().await;

    // Disconnect should succeed or fail gracefully (no panic)
    match disconnect_result {
        Ok(_) => eprintln!("Disconnect during query succeeded"),
        Err(e) => eprintln!("Disconnect during query error (acceptable): {}", e),
    }

    // Verify we can create a new client
    let mut client2 = ClaudeClient::new(Some(default_options()));
    client2
        .connect()
        .await
        .expect("Should be able to connect after forced disconnect");
    client2.disconnect().await.ok();
}

/// Test disconnect before any query.
#[tokio::test]
async fn test_disconnect_before_query() {
    let mut client = ClaudeClient::new(Some(default_options()));
    client.connect().await.expect("Failed to connect");

    // Disconnect without sending any query
    client
        .disconnect()
        .await
        .expect("Disconnect before query should succeed");
}

/// Test disconnect after query completion.
#[tokio::test]
async fn test_disconnect_after_completion() {
    let mut client = ClaudeClient::new(Some(default_options()));
    client.connect().await.expect("Failed to connect");

    client
        .query("Say 'complete'")
        .await
        .expect("Failed to send query");

    let (response, result) = client
        .receive_response()
        .await
        .expect("Failed to receive response");

    assert!(!result.is_error);
    assert!(!response.is_empty());

    // Disconnect after successful completion
    client
        .disconnect()
        .await
        .expect("Disconnect after completion should succeed");
}

// ============================================================================
// Tokio Cancellation Patterns
// ============================================================================

/// Test tokio::select! cancellation.
#[tokio::test]
async fn test_tokio_select_cancellation() {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(3)
        .with_permission_mode(PermissionMode::Default);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await.expect("Failed to connect");

    client
        .query("Write a long poem about the ocean with at least 20 lines.")
        .await
        .expect("Failed to send query");

    // Use select to race between response and timeout
    let completed = tokio::select! {
        _ = async {
            while let Some(_) = client.receive_messages().next().await {
                // Keep reading
            }
        } => {
            true  // Completed normally
        }
        _ = tokio::time::sleep(Duration::from_millis(500)) => {
            false  // Timeout - this is expected for a long query
        }
    };

    // Cleanup
    client.disconnect().await.ok();

    // Verify system is still functional
    let messages = collect_messages("Say 'after select'", default_options())
        .await
        .expect("System should work after select cancellation");

    assert!(get_result(&messages).is_some());

    eprintln!("Select test: completed={}", completed);
}

/// Test task abort.
#[tokio::test]
async fn test_task_abort() {
    let handle = tokio::spawn(async {
        let messages = collect_messages(
            "Write a very long explanation of quantum mechanics.",
            ClaudeAgentOptions::new()
                .with_max_turns(10)
                .with_permission_mode(PermissionMode::Default),
        )
        .await;
        messages
    });

    // Wait briefly then abort
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();

    // Wait for abort to process
    let result = handle.await;

    assert!(result.is_err(), "Aborted task should return error");

    // Verify system still works
    tokio::time::sleep(Duration::from_millis(100)).await;

    let messages = collect_messages("Say 'after abort'", default_options())
        .await
        .expect("System should work after task abort");

    assert!(get_result(&messages).is_some());
}

// ============================================================================
// Timeout Behavior
// ============================================================================

/// Test that SDK timeout is respected.
#[tokio::test]
async fn test_sdk_timeout_respected() {
    let options = ClaudeAgentOptions::new()
        .with_timeout_secs(5)
        .with_max_turns(10)
        .with_permission_mode(PermissionMode::Default);

    let start = std::time::Instant::now();

    // This query might exceed timeout
    let result = collect_messages(
        "Explain every major programming language in detail.",
        options,
    )
    .await;

    let elapsed = start.elapsed();

    match result {
        Ok(messages) => {
            // Completed within timeout
            let result = get_result(&messages);
            eprintln!("Completed in {:?}, result: {:?}", elapsed, result.is_some());
        }
        Err(e) => {
            // Might be timeout error
            if e.to_lowercase().contains("timeout") {
                // Verify timing is reasonable (should be around 5s, allow up to 15s)
                assert!(
                    elapsed.as_secs() <= 15,
                    "Timeout should occur around 5 seconds, took {:?}",
                    elapsed
                );
            }
            eprintln!("Error after {:?}: {}", elapsed, e);
        }
    }
}

/// Test that outer timeout catches hung operations.
#[tokio::test]
async fn test_outer_timeout_safety() {
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        collect_messages("Say 'timeout test'", default_options()).await
    })
    .await;

    match result {
        Ok(Ok(messages)) => {
            assert!(get_result(&messages).is_some());
        }
        Ok(Err(e)) => {
            eprintln!("Query error (within timeout): {}", e);
        }
        Err(_) => {
            panic!("Outer timeout hit - query took too long");
        }
    }
}

// ============================================================================
// Rapid Reconnection
// ============================================================================

/// Test rapid connect/disconnect cycles.
#[tokio::test]
async fn test_rapid_reconnect_cycles() {
    for i in 0..5 {
        let mut client = ClaudeClient::new(Some(default_options()));

        client
            .connect()
            .await
            .expect(&format!("Connect {} failed", i));

        // Optional: send a quick query
        if i % 2 == 0 {
            client.query("Hi").await.ok();
        }

        client
            .disconnect()
            .await
            .expect(&format!("Disconnect {} failed", i));

        // Very brief pause
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Final verification
    let messages = collect_messages("Say 'cycles complete'", default_options())
        .await
        .expect("Should work after rapid cycles");

    assert!(get_result(&messages).is_some());
}

/// Test reconnect after error.
#[tokio::test]
async fn test_reconnect_after_error() {
    let mut client = ClaudeClient::new(Some(default_options()));
    client.connect().await.expect("Failed to connect");

    // Force an error by disconnecting during operation
    client.query("Start").await.ok();
    client.disconnect().await.ok();

    // Reconnect should work
    client.connect().await.expect("Reconnect failed");

    client
        .query("After reconnect")
        .await
        .expect("Query after reconnect failed");

    let (response, result) = client
        .receive_response()
        .await
        .expect("Receive after reconnect failed");

    assert!(!result.is_error);
    assert!(!response.is_empty());

    client.disconnect().await.ok();
}
