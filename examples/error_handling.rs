//! Error Handling Patterns Example.
//!
//! This example demonstrates best practices for handling errors
//! when using the Claude Agents SDK.
//!
//! Run with: cargo run --example error_handling

use claude_agents_sdk::{
    query, ClaudeAgentOptions, ClaudeClient, ClaudeSDKError, Message, PermissionMode,
};
use std::time::Duration;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Error Handling Patterns Example ===\n");

    // Example 1: Handle connection errors
    println!("--- Example 1: Connection Error Handling ---");
    connection_error_handling().await;

    // Example 2: Handle query errors with recovery
    println!("\n--- Example 2: Query Error Recovery ---");
    query_error_recovery().await;

    // Example 3: Handle timeout errors
    println!("\n--- Example 3: Timeout Handling ---");
    timeout_handling().await;

    // Example 4: Handle stream errors gracefully
    println!("\n--- Example 4: Stream Error Handling ---");
    stream_error_handling().await;

    // Example 5: Using Result combinators
    println!("\n--- Example 5: Result Combinators ---");
    result_combinators().await;

    println!("\n=== All examples completed ===");
    Ok(())
}

/// Demonstrate connection error handling.
async fn connection_error_handling() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let mut client = ClaudeClient::new(Some(options), None);

    match client.connect().await {
        Ok(_) => {
            println!("✓ Connected successfully");

            // Clean up
            if let Err(e) = client.disconnect().await {
                eprintln!("  Warning: disconnect error: {}", e);
            }
        }
        Err(e) => {
            // Handle different error types
            match &e {
                ClaudeSDKError::CLINotFound { message } => {
                    eprintln!("✗ Claude CLI not found: {}", message);
                    eprintln!("  Run: npm install -g @anthropic-ai/claude-code");
                }
                ClaudeSDKError::CLIConnection { message, .. } => {
                    eprintln!("✗ Connection failed: {}", message);
                    eprintln!("  Check your authentication and network connection.");
                }
                _ => {
                    eprintln!("✗ Unexpected error: {}", e);
                }
            }
        }
    }
}

/// Demonstrate query error recovery with retries.
async fn query_error_recovery() {
    const MAX_RETRIES: u32 = 3;
    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
        println!("  Attempt {}/{}...", attempt, MAX_RETRIES);

        let options = ClaudeAgentOptions::new()
            .with_permission_mode(PermissionMode::Default)
            .with_max_turns(1)
            .with_timeout_secs(30);

        match query("Say 'success'", Some(options), None).await {
            Ok(mut stream) => {
                let mut response = String::new();
                let mut got_result = false;

                while let Some(msg) = stream.next().await {
                    match msg {
                        Ok(Message::Assistant(asst)) => {
                            response.push_str(&asst.text());
                        }
                        Ok(Message::Result(result)) => {
                            if result.is_error {
                                last_error = Some(format!("Query returned error: {}", result.subtype));
                                break;
                            }
                            got_result = true;
                            break;
                        }
                        Err(e) => {
                            last_error = Some(e.to_string());
                            break;
                        }
                        _ => {}
                    }
                }

                if got_result {
                    println!("✓ Query succeeded: {}", response.trim());
                    return;
                }
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }

        // Exponential backoff before retry
        if attempt < MAX_RETRIES {
            let delay = Duration::from_millis(100 * 2u64.pow(attempt - 1));
            println!("  Retrying in {:?}...", delay);
            tokio::time::sleep(delay).await;
        }
    }

    eprintln!(
        "✗ All {} attempts failed. Last error: {:?}",
        MAX_RETRIES, last_error
    );
}

/// Demonstrate timeout handling.
async fn timeout_handling() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1)
        .with_timeout_secs(60); // SDK timeout

    // Application-level timeout
    let app_timeout = Duration::from_secs(30);

    let query_future = async {
        let mut stream = query("What is 1+1?", Some(options), None).await?;
        let mut response = String::new();

        while let Some(msg) = stream.next().await {
            match msg? {
                Message::Assistant(asst) => response.push_str(&asst.text()),
                Message::Result(_) => break,
                _ => {}
            }
        }

        Ok::<String, ClaudeSDKError>(response)
    };

    match tokio::time::timeout(app_timeout, query_future).await {
        Ok(Ok(response)) => {
            println!("✓ Query completed: {}", response.trim());
        }
        Ok(Err(e)) => {
            // SDK/API error
            eprintln!("✗ Query failed: {}", e);
        }
        Err(_) => {
            // Application timeout
            eprintln!("✗ Query timed out after {:?}", app_timeout);
            eprintln!("  Consider increasing timeout or simplifying the query.");
        }
    }
}

/// Demonstrate graceful stream error handling.
async fn stream_error_handling() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    match query("Say 'hello'", Some(options), None).await {
        Ok(mut stream) => {
            let mut message_count = 0;
            let mut error_count = 0;

            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Assistant(asst)) => {
                        message_count += 1;
                        print!("{}", asst.text());
                    }
                    Ok(Message::Result(result)) => {
                        println!();
                        if result.is_error {
                            eprintln!("  Stream ended with error: {}", result.subtype);
                            error_count += 1;
                        }
                        break;
                    }
                    Ok(_) => {
                        // Other message types (system, user, etc.)
                        message_count += 1;
                    }
                    Err(e) => {
                        eprintln!("\n  Stream error: {}", e);
                        error_count += 1;

                        // Decide whether to continue or abort
                        if error_count >= 3 {
                            eprintln!("  Too many errors, aborting stream.");
                            break;
                        }
                    }
                }
            }

            println!(
                "✓ Stream completed: {} messages, {} errors",
                message_count, error_count
            );
        }
        Err(e) => {
            eprintln!("✗ Failed to start query: {}", e);
        }
    }
}

/// Demonstrate using Result combinators for cleaner error handling.
async fn result_combinators() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    // Using map_err for error transformation
    let result = query("Say 'test'", Some(options), None)
        .await
        .map_err(|e| format!("Query init failed: {}", e));

    match result {
        Ok(mut stream) => {
            // Collect all text using functional style
            let mut texts = Vec::new();

            while let Some(msg) = stream.next().await {
                if let Ok(Message::Assistant(asst)) = msg {
                    texts.push(asst.text());
                } else if let Ok(Message::Result(_)) = msg {
                    break;
                }
            }

            let response = texts.join("");
            println!("✓ Response: {}", response.trim());
        }
        Err(e) => {
            eprintln!("✗ {}", e);
        }
    }
}
