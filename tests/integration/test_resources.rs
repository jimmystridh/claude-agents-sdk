//! Resource management and cleanup tests.
//!
//! These tests verify that the SDK properly cleans up resources like
//! subprocess handles and doesn't leak processes.

#![cfg(feature = "integration-tests")]

use std::time::Duration;

use claude_agents_sdk::{query, ClaudeClient};
use tokio_stream::StreamExt;

use crate::integration::helpers::{
    collect_messages, count_claude_processes, default_options, get_result, quick_options,
};

// ============================================================================
// Process Cleanup Tests
// ============================================================================

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

    // Allow some tolerance (1-2 processes) since counting may include other claude instances
    assert!(
        final_count <= initial_count + 2,
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

        // Brief pause between iterations
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Allow time for process cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;

    let final_count = count_claude_processes();

    assert!(
        final_count <= initial_count + 2,
        "Process leak on drop: started with {}, ended with {}",
        initial_count,
        final_count
    );
}

/// Test cleanup when client is dropped without calling disconnect.
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
        final_count <= initial_count + 2,
        "Process leak on drop without disconnect: {} -> {}",
        initial_count,
        final_count
    );
}

// ============================================================================
// Handle/Resource Leak Tests
// ============================================================================

/// Test for handle leaks across many sequential sessions.
#[tokio::test]
async fn test_no_handle_leaks_sequential() {
    // Run many sessions and verify stable resource usage
    for i in 0..10 {
        let messages = collect_messages(&format!("Say 'session {}'", i), default_options()).await;

        match messages {
            Ok(msgs) => {
                assert!(get_result(&msgs).is_some(), "Session {} should complete", i);
            }
            Err(e) => {
                eprintln!("Session {} error (may be transient): {}", i, e);
            }
        }

        // Brief pause between sessions
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // If we get here without OOM or handle exhaustion, test passes
}

/// Test for handle leaks with concurrent sessions.
#[tokio::test]
async fn test_no_handle_leaks_concurrent() {
    for batch in 0..3 {
        let handles: Vec<_> = (0..3)
            .map(|i| {
                tokio::spawn(async move {
                    let messages = collect_messages(
                        &format!("Say 'batch {} item {}'", batch, i),
                        default_options(),
                    )
                    .await;
                    messages.is_ok()
                })
            })
            .collect();

        let mut successes = 0;
        for handle in handles {
            if handle.await.unwrap_or(false) {
                successes += 1;
            }
        }

        // At least some should succeed
        assert!(
            successes >= 1,
            "Batch {} should have at least 1 success, got {}",
            batch,
            successes
        );

        // Brief pause between batches
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // If we get here without issues, test passes
}

// ============================================================================
// Cleanup After Errors
// ============================================================================

/// Test cleanup occurs even after query errors.
#[tokio::test]
async fn test_cleanup_after_error() {
    let initial_count = count_claude_processes();

    // Try some operations that might fail
    for _ in 0..3 {
        let mut client = ClaudeClient::new(Some(default_options()), None);

        if client.connect().await.is_ok() {
            // Send query
            let _ = client.query("Test").await;

            // Don't wait for response, just disconnect
            let _ = client.disconnect().await;
        }
    }

    // Allow cleanup time
    tokio::time::sleep(Duration::from_millis(500)).await;

    let final_count = count_claude_processes();

    assert!(
        final_count <= initial_count + 2,
        "Cleanup after errors failed: {} -> {}",
        initial_count,
        final_count
    );
}

/// Test cleanup when stream is exhausted normally.
#[tokio::test]
async fn test_cleanup_after_stream_exhausted() {
    let initial_count = count_claude_processes();

    for _ in 0..3 {
        let result = collect_messages("Say 'done'", default_options()).await;

        // Whether it succeeded or failed, cleanup should happen
        match result {
            Ok(msgs) => assert!(get_result(&msgs).is_some()),
            Err(e) => eprintln!("Stream error (acceptable): {}", e),
        }
    }

    // Allow cleanup time
    tokio::time::sleep(Duration::from_millis(500)).await;

    let final_count = count_claude_processes();

    assert!(
        final_count <= initial_count + 2,
        "Cleanup after exhaustion failed: {} -> {}",
        initial_count,
        final_count
    );
}

// ============================================================================
// Stress Tests (behind ignore flag)
// ============================================================================

/// Stress test: many rapid sessions.
#[tokio::test]
#[ignore]
async fn stress_test_rapid_sessions() {
    let initial_count = count_claude_processes();

    for i in 0..50 {
        let _ = collect_messages(&format!("Say {}", i), quick_options()).await;
    }

    tokio::time::sleep(Duration::from_secs(1)).await;

    let final_count = count_claude_processes();

    assert!(
        final_count <= initial_count + 5,
        "Stress test leak: {} -> {}",
        initial_count,
        final_count
    );
}

/// Stress test: concurrent session bursts.
#[tokio::test]
#[ignore]
async fn stress_test_concurrent_bursts() {
    let initial_count = count_claude_processes();

    for burst in 0..5 {
        let handles: Vec<_> = (0..10)
            .map(|i| {
                tokio::spawn(async move {
                    collect_messages(&format!("Burst {} item {}", burst, i), quick_options()).await
                })
            })
            .collect();

        for handle in handles {
            let _ = handle.await;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tokio::time::sleep(Duration::from_secs(2)).await;

    let final_count = count_claude_processes();

    assert!(
        final_count <= initial_count + 5,
        "Burst stress test leak: {} -> {}",
        initial_count,
        final_count
    );
}
