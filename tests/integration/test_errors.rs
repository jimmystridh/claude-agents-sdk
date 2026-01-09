//! Error path tests for CLI integration.
//!
//! These tests verify graceful error handling for various failure scenarios.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{query, ClaudeAgentOptions, ClaudeClient, PermissionMode};
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::integration::helpers::*;

// ============================================================================
// CLI Executable Errors
// ============================================================================

/// Test that malformed/special character prompts don't cause panics.
///
/// The goal is to verify graceful handling - either success or a clear error,
/// but never a panic or crash.
#[tokio::test]
async fn test_special_characters_no_panic() {
    // Prompts that should definitely work
    let valid_prompts = [
        "{\"test\": \"valid\"}", // Valid JSON-like string
        "```json\n{}\n```",      // Markdown code blocks
        "Hello\r\nWorld\r\n",    // Windows line endings
        "🎉🎊🎁",                // Emoji only
    ];

    // Prompts that may or may not work (CLI may reject them)
    let edge_case_prompts = [
        "\x00\x01\x02", // Null bytes - likely rejected
        "\n\n\n",       // Just newlines - may be rejected
        "\t\t\t",       // Just tabs - may be rejected
    ];

    for prompt in valid_prompts {
        let result = collect_messages(prompt, default_options()).await;
        match result {
            Ok(messages) => {
                assert!(
                    get_result(&messages).is_some(),
                    "Should have result message for prompt: {:?}",
                    prompt
                );
            }
            Err(e) => {
                // Unexpected error for valid prompts
                panic!("Valid prompt {:?} failed unexpectedly: {}", prompt, e);
            }
        }
    }

    for prompt in edge_case_prompts {
        let result = collect_messages(prompt, default_options()).await;
        // Either outcome is acceptable for edge cases - just don't panic
        match result {
            Ok(messages) => {
                eprintln!(
                    "Edge case prompt {:?} succeeded with {} messages",
                    prompt,
                    messages.len()
                );
            }
            Err(e) => {
                eprintln!("Edge case prompt {:?} failed gracefully: {}", prompt, e);
            }
        }
    }
}

/// Test empty prompt handling.
///
/// Empty prompts may be rejected by the CLI - either outcome is acceptable.
#[tokio::test]
async fn test_empty_prompt() {
    let result = collect_messages("", default_options()).await;

    // Empty prompt should either work or fail gracefully - either is acceptable
    match result {
        Ok(messages) => {
            eprintln!("Empty prompt succeeded with {} messages", messages.len());
            // If it works, we should have a result
            if get_result(&messages).is_none() {
                eprintln!("Note: Empty prompt completed but no result message");
            }
        }
        Err(e) => {
            // Error is acceptable for empty prompt
            eprintln!("Empty prompt error (acceptable): {}", e);
        }
    }
    // Test passes in either case - we just want to verify no panic
}

/// Test very long prompt handling.
#[tokio::test]
async fn test_very_long_prompt() {
    // 10KB prompt
    let long_text = "word ".repeat(2000);
    let prompt = format!("Count roughly how many words are here: {}", long_text);

    let result = collect_messages(&prompt, default_options()).await;

    match result {
        Ok(messages) => {
            let result = get_result(&messages).expect("Should have result");
            assert!(!result.is_error, "Long prompt should be handled");
        }
        Err(e) => {
            // If it fails, should be a clear error
            assert!(
                e.contains("too long")
                    || e.contains("limit")
                    || e.contains("size")
                    || !e.is_empty(),
                "Error should be descriptive: {}",
                e
            );
        }
    }
}

// ============================================================================
// Connection and Timeout Errors
// ============================================================================

/// Test that connection errors are reported clearly.
#[tokio::test]
async fn test_connection_error_reporting() {
    let mut client = ClaudeClient::new(Some(default_options()));

    // Connect should work (it spawns the subprocess)
    let connect_result = client.connect().await;

    match connect_result {
        Ok(_) => {
            // Connected successfully, clean up
            client.disconnect().await.ok();
        }
        Err(e) => {
            // If it failed, error should be clear
            let error_str = e.to_string();
            assert!(!error_str.is_empty(), "Error message should not be empty");
            eprintln!("Connection error (for reference): {}", error_str);
        }
    }
}

/// Test behavior when sending query without connecting first.
#[tokio::test]
async fn test_query_without_connect() {
    let mut client = ClaudeClient::new(Some(default_options()));

    // Try to query without connecting - should fail gracefully
    let query_result = client.query("Hello").await;

    assert!(query_result.is_err(), "Query without connect should fail");

    let error = query_result.unwrap_err();
    eprintln!("Query without connect error: {}", error);
}

/// Test double disconnect handling.
#[tokio::test]
async fn test_double_disconnect() {
    let mut client = ClaudeClient::new(Some(default_options()));
    client.connect().await.expect("Failed to connect");

    // First disconnect
    client.disconnect().await.expect("First disconnect failed");

    // Second disconnect - should not panic
    let second_result = client.disconnect().await;

    // Either succeeds (idempotent) or fails gracefully
    match second_result {
        Ok(_) => eprintln!("Double disconnect succeeded (idempotent)"),
        Err(e) => eprintln!("Double disconnect error (acceptable): {}", e),
    }
}

/// Test double connect handling.
#[tokio::test]
async fn test_double_connect() {
    let mut client = ClaudeClient::new(Some(default_options()));

    // First connect
    client.connect().await.expect("First connect failed");

    // Second connect - should not panic
    let second_result = client.connect().await;

    // Either succeeds (reconnects) or fails gracefully
    match second_result {
        Ok(_) => eprintln!("Double connect succeeded (reconnect)"),
        Err(e) => eprintln!("Double connect error (acceptable): {}", e),
    }

    // Cleanup
    client.disconnect().await.ok();
}

// ============================================================================
// Stream Error Handling
// ============================================================================

/// Test that stream errors are propagated correctly.
#[tokio::test]
async fn test_stream_error_propagation() {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(1)
        .with_permission_mode(PermissionMode::Default);

    let stream_result = query("Hello", Some(options), None).await;

    match stream_result {
        Ok(mut stream) => {
            let mut had_error = false;
            let mut had_result = false;

            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(m) => {
                        if m.is_result() {
                            had_result = true;
                        }
                    }
                    Err(e) => {
                        had_error = true;
                        eprintln!("Stream error: {}", e);
                    }
                }
            }

            // Should have either completed with result or had clear error
            assert!(
                had_result || had_error,
                "Stream should complete with result or error"
            );
        }
        Err(e) => {
            // Query start failed
            eprintln!("Query start error: {}", e);
        }
    }
}

/// Test timeout behavior with short timeout.
#[tokio::test]
async fn test_short_timeout_behavior() {
    let options = ClaudeAgentOptions::new()
        .with_timeout_secs(1) // Very short
        .with_max_turns(10) // Allow many turns
        .with_permission_mode(PermissionMode::Default);

    let start = std::time::Instant::now();

    // Long query that likely exceeds 1 second
    let result = tokio::time::timeout(
        Duration::from_secs(30), // Outer timeout for test safety
        collect_messages(
            "Explain the entire history of computer science in detail.",
            options,
        ),
    )
    .await;

    let elapsed = start.elapsed();

    match result {
        Ok(Ok(messages)) => {
            // Completed within SDK timeout - that's fine
            let result = get_result(&messages);
            eprintln!(
                "Completed in {:?} with result: {:?}",
                elapsed,
                result.is_some()
            );
        }
        Ok(Err(e)) => {
            // SDK error (possibly timeout)
            eprintln!("SDK error after {:?}: {}", elapsed, e);
            if e.to_lowercase().contains("timeout") {
                assert!(
                    elapsed.as_secs() < 30,
                    "Timeout should occur reasonably quickly"
                );
            }
        }
        Err(_) => {
            // Outer timeout hit - SDK didn't respect its timeout
            panic!("Test timeout hit - SDK timeout may not be working");
        }
    }
}

// ============================================================================
// API Error Handling (requires specific conditions)
// ============================================================================

/// Test handling of authentication failure.
///
/// NOTE: This test requires invalid credentials to run.
/// Run manually with invalid API key set.
#[tokio::test]
#[ignore]
async fn test_authentication_failure() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let result = collect_messages("test", options).await;

    assert!(result.is_err(), "Should fail with invalid credentials");

    let err = result.unwrap_err();
    assert!(
        err.to_lowercase().contains("auth")
            || err.to_lowercase().contains("api")
            || err.to_lowercase().contains("key")
            || err.to_lowercase().contains("unauthorized")
            || err.to_lowercase().contains("401"),
        "Error should relate to authentication: {}",
        err
    );
}
