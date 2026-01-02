//! Streaming client example with multiple queries.
//!
//! This example demonstrates using the `ClaudeClient` for bidirectional
//! communication with multiple queries in a single session.
//!
//! Run with: cargo run --example streaming_client

use claude_agents_sdk::{ClaudeClient, ClaudeAgentOptions, Message, PermissionMode};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Claude Streaming Client Example ===\n");

    // Configure the client
    let options = ClaudeAgentOptions::new()
        .with_max_turns(5)
        .with_permission_mode(PermissionMode::Default);

    // Create and connect the client
    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await?;

    println!("Connected to Claude CLI\n");

    // First query
    println!("--- Query 1: Basic question ---");
    client.query("What is 2 + 2? Answer in one word.").await?;

    let (response, result) = client.receive_response().await?;
    println!("Response: {}", response.trim());
    println!("Duration: {}ms\n", result.duration_ms);

    // Second query (continuation)
    println!("--- Query 2: Follow-up ---");
    client.query("What is that number multiplied by 10?").await?;

    let (response, result) = client.receive_response().await?;
    println!("Response: {}", response.trim());
    println!("Duration: {}ms\n", result.duration_ms);

    // Third query with streaming output
    println!("--- Query 3: Streaming response ---");
    client.query("Count from 1 to 5, one number per line.").await?;

    print!("Response: ");
    while let Some(msg) = client.receive_messages().next().await {
        match msg? {
            Message::Assistant(asst) => {
                print!("{}", asst.text());
            }
            Message::Result(result) => {
                println!("\nDuration: {}ms", result.duration_ms);
                break;
            }
            _ => {}
        }
    }

    // Disconnect
    client.disconnect().await?;
    println!("\nDisconnected from Claude CLI");

    Ok(())
}
