//! Concurrent session integration tests.
//!
//! Tests for running multiple Claude sessions simultaneously.

#![cfg(feature = "integration-tests")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use claude_agents_sdk::{ClaudeAgentOptions, ClaudeClient, PermissionMode};
use tokio::sync::Barrier;

use crate::integration::helpers::{collect_messages, default_options};

// ============================================================================
// Concurrent Query Tests
// ============================================================================

/// Test two concurrent one-shot queries.
#[tokio::test]
async fn test_two_concurrent_queries() {
    let options1 = default_options();
    let options2 = default_options();

    let (result1, result2) = tokio::join!(
        collect_messages("What is 2+2? Reply with just the number.", options1),
        collect_messages("What is 3+3? Reply with just the number.", options2)
    );

    // Both should complete (success or timeout)
    let success_count = [&result1, &result2].iter().filter(|r| r.is_ok()).count();

    eprintln!(
        "Concurrent queries: result1={:?}, result2={:?}",
        result1.is_ok(),
        result2.is_ok()
    );

    // At least one should succeed
    assert!(
        success_count >= 1,
        "At least one concurrent query should succeed"
    );
}

/// Test multiple concurrent queries (stress test lite).
#[tokio::test]
async fn test_multiple_concurrent_queries() {
    let count = 3; // Keep low to avoid rate limiting
    let completed = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..count)
        .map(|i| {
            let completed = Arc::clone(&completed);
            tokio::spawn(async move {
                let options = default_options();
                let prompt = format!("What is {}+{}? Reply with just the number.", i, i);

                let result = tokio::time::timeout(
                    Duration::from_secs(90),
                    collect_messages(&prompt, options),
                )
                .await;

                match result {
                    Ok(Ok(_)) => {
                        completed.fetch_add(1, Ordering::SeqCst);
                        true
                    }
                    Ok(Err(e)) => {
                        eprintln!("Query {} failed: {}", i, e);
                        false
                    }
                    Err(_) => {
                        eprintln!("Query {} timed out", i);
                        false
                    }
                }
            })
        })
        .collect();

    // Wait for all to complete
    for handle in handles {
        let _ = handle.await;
    }

    let completed_count = completed.load(Ordering::SeqCst);
    eprintln!(
        "Multiple concurrent: {}/{} completed",
        completed_count, count
    );

    // At least half should complete
    assert!(
        completed_count >= count / 2,
        "At least half of concurrent queries should complete"
    );
}

// ============================================================================
// Concurrent Client Tests
// ============================================================================

/// Test two concurrent ClaudeClient sessions.
#[tokio::test]
async fn test_two_concurrent_clients() {
    let handle1 = tokio::spawn(async {
        let options = ClaudeAgentOptions::new()
            .with_permission_mode(PermissionMode::Default)
            .with_max_turns(1);

        let mut client = ClaudeClient::new(Some(options));

        if let Err(e) = client.connect().await {
            eprintln!("Client 1 connect failed: {}", e);
            return false;
        }

        if let Err(e) = client.query("Say 'client1'").await {
            eprintln!("Client 1 query failed: {}", e);
            client.disconnect().await.ok();
            return false;
        }

        let result = client.receive_response().await;
        client.disconnect().await.ok();

        result.is_ok()
    });

    let handle2 = tokio::spawn(async {
        let options = ClaudeAgentOptions::new()
            .with_permission_mode(PermissionMode::Default)
            .with_max_turns(1);

        let mut client = ClaudeClient::new(Some(options));

        if let Err(e) = client.connect().await {
            eprintln!("Client 2 connect failed: {}", e);
            return false;
        }

        if let Err(e) = client.query("Say 'client2'").await {
            eprintln!("Client 2 query failed: {}", e);
            client.disconnect().await.ok();
            return false;
        }

        let result = client.receive_response().await;
        client.disconnect().await.ok();

        result.is_ok()
    });

    let (result1, result2) = tokio::join!(handle1, handle2);

    let success1 = result1.unwrap_or(false);
    let success2 = result2.unwrap_or(false);

    eprintln!(
        "Concurrent clients: client1={}, client2={}",
        success1, success2
    );

    // At least one should succeed
    assert!(
        success1 || success2,
        "At least one concurrent client should succeed"
    );
}

/// Test concurrent clients with barrier synchronization.
#[tokio::test]
async fn test_synchronized_concurrent_clients() {
    let barrier = Arc::new(Barrier::new(2));
    let completed = Arc::new(AtomicUsize::new(0));

    let b1 = Arc::clone(&barrier);
    let c1 = Arc::clone(&completed);
    let handle1 = tokio::spawn(async move {
        let options = ClaudeAgentOptions::new()
            .with_permission_mode(PermissionMode::Default)
            .with_max_turns(1);

        let mut client = ClaudeClient::new(Some(options));

        // Wait for both clients to be ready
        b1.wait().await;

        if client.connect().await.is_ok() {
            if client.query("Say 'sync1'").await.is_ok() {
                if client.receive_response().await.is_ok() {
                    c1.fetch_add(1, Ordering::SeqCst);
                }
            }
            client.disconnect().await.ok();
        }
    });

    let b2 = Arc::clone(&barrier);
    let c2 = Arc::clone(&completed);
    let handle2 = tokio::spawn(async move {
        let options = ClaudeAgentOptions::new()
            .with_permission_mode(PermissionMode::Default)
            .with_max_turns(1);

        let mut client = ClaudeClient::new(Some(options));

        // Wait for both clients to be ready
        b2.wait().await;

        if client.connect().await.is_ok() {
            if client.query("Say 'sync2'").await.is_ok() {
                if client.receive_response().await.is_ok() {
                    c2.fetch_add(1, Ordering::SeqCst);
                }
            }
            client.disconnect().await.ok();
        }
    });

    // Add timeout
    let result = tokio::time::timeout(Duration::from_secs(120), async {
        let _ = handle1.await;
        let _ = handle2.await;
    })
    .await;

    let completed_count = completed.load(Ordering::SeqCst);
    eprintln!("Synchronized concurrent: {}/2 completed", completed_count);

    match result {
        Ok(_) => {
            assert!(
                completed_count >= 1,
                "At least one synchronized client should complete"
            );
        }
        Err(_) => {
            eprintln!("Synchronized test timed out");
        }
    }
}

// ============================================================================
// Resource Isolation Tests
// ============================================================================

/// Test that concurrent sessions don't share state.
#[tokio::test]
async fn test_concurrent_sessions_isolated() {
    let session1_response = Arc::new(tokio::sync::Mutex::new(String::new()));
    let session2_response = Arc::new(tokio::sync::Mutex::new(String::new()));

    let r1 = Arc::clone(&session1_response);
    let handle1 = tokio::spawn(async move {
        let options = ClaudeAgentOptions::new()
            .with_permission_mode(PermissionMode::Default)
            .with_max_turns(1);

        let mut client = ClaudeClient::new(Some(options));

        if client.connect().await.is_err() {
            return;
        }

        if client
            .query("The secret code is ALPHA123. Acknowledge with 'Code received: ALPHA123'")
            .await
            .is_err()
        {
            client.disconnect().await.ok();
            return;
        }

        if let Ok((response, _)) = client.receive_response().await {
            *r1.lock().await = response;
        }

        client.disconnect().await.ok();
    });

    let r2 = Arc::clone(&session2_response);
    let handle2 = tokio::spawn(async move {
        // Small delay to ensure sessions are distinct
        tokio::time::sleep(Duration::from_millis(100)).await;

        let options = ClaudeAgentOptions::new()
            .with_permission_mode(PermissionMode::Default)
            .with_max_turns(1);

        let mut client = ClaudeClient::new(Some(options));

        if client.connect().await.is_err() {
            return;
        }

        // This session should NOT know about ALPHA123
        if client
            .query("What is the secret code? If you don't know, say 'No code known'")
            .await
            .is_err()
        {
            client.disconnect().await.ok();
            return;
        }

        if let Ok((response, _)) = client.receive_response().await {
            *r2.lock().await = response;
        }

        client.disconnect().await.ok();
    });

    let _ = tokio::time::timeout(Duration::from_secs(120), async {
        let _ = handle1.await;
        let _ = handle2.await;
    })
    .await;

    let resp1 = session1_response.lock().await;
    let resp2 = session2_response.lock().await;

    eprintln!("Session 1 response: {}", *resp1);
    eprintln!("Session 2 response: {}", *resp2);

    // Session 2 should NOT contain ALPHA123 (proving isolation)
    if !resp1.is_empty() && !resp2.is_empty() {
        // Only check if session 1 acknowledged the code
        if resp1.contains("ALPHA123") {
            // Session 2 should not know about it
            let session2_knows = resp2.contains("ALPHA123");
            assert!(
                !session2_knows,
                "Session 2 should not know Session 1's secret (sessions should be isolated)"
            );
        }
    }
}

// ============================================================================
// Sequential After Concurrent Tests
// ============================================================================

/// Test that sequential operations work after concurrent ones.
#[tokio::test]
async fn test_sequential_after_concurrent() {
    // First, run concurrent queries
    let (r1, r2) = tokio::join!(
        collect_messages("Say 'concurrent1'", default_options()),
        collect_messages("Say 'concurrent2'", default_options())
    );

    eprintln!("Concurrent phase: r1={:?}, r2={:?}", r1.is_ok(), r2.is_ok());

    // Then run sequential query
    let sequential = collect_messages("Say 'sequential'", default_options()).await;

    eprintln!("Sequential phase: {:?}", sequential.is_ok());

    // Sequential should work
    assert!(
        sequential.is_ok(),
        "Sequential query after concurrent should succeed"
    );
}

/// Test rapid sequential queries after concurrent burst.
#[tokio::test]
async fn test_rapid_sequential_after_burst() {
    // Concurrent burst
    let handles: Vec<_> = (0..2)
        .map(|i| {
            tokio::spawn(async move {
                let options = default_options();
                collect_messages(&format!("Say 'burst{}'", i), options).await
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.await;
    }

    // Now rapid sequential
    let mut success_count = 0;
    for i in 0..3 {
        let result = collect_messages(&format!("Say 'seq{}'", i), default_options()).await;
        if result.is_ok() {
            success_count += 1;
        }
    }

    eprintln!("Rapid sequential: {}/3 succeeded", success_count);

    assert!(
        success_count >= 2,
        "Most rapid sequential queries should succeed"
    );
}
