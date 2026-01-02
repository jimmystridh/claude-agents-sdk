//! Quick start example for Claude Agents SDK (Rust port of quick_start.py).
//!
//! This example demonstrates the basic usage patterns:
//! - Basic query with no options
//! - Query with custom options
//! - Query with tools enabled
//!
//! Run with: cargo run --example quick_start

use claude_agents_sdk::{query, ClaudeAgentOptions, ContentBlock, Message};
use tokio_stream::StreamExt;

/// Basic example - simple question.
async fn basic_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic Example ===");

    let mut stream = query("What is 2 + 2?", None, None).await?;

    while let Some(message) = stream.next().await {
        if let Message::Assistant(msg) = message? {
            for block in &msg.content {
                if let ContentBlock::Text(text) = block {
                    println!("Claude: {}", text.text);
                }
            }
        }
    }
    println!();

    Ok(())
}

/// Example with custom options.
async fn with_options_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== With Options Example ===");

    let options = ClaudeAgentOptions::new()
        .with_system_prompt("You are a helpful assistant that explains things simply.")
        .with_max_turns(1);

    let mut stream = query(
        "Explain what Rust is in one sentence.",
        Some(options),
        None,
    )
    .await?;

    while let Some(message) = stream.next().await {
        if let Message::Assistant(msg) = message? {
            for block in &msg.content {
                if let ContentBlock::Text(text) = block {
                    println!("Claude: {}", text.text);
                }
            }
        }
    }
    println!();

    Ok(())
}

/// Example using tools.
async fn with_tools_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== With Tools Example ===");

    let options = ClaudeAgentOptions::new()
        .with_allowed_tools(vec!["Read".to_string(), "Write".to_string()])
        .with_system_prompt("You are a helpful file assistant.");

    let mut stream = query(
        "Create a file called hello.txt with 'Hello, World!' in it",
        Some(options),
        None,
    )
    .await?;

    while let Some(message) = stream.next().await {
        match message? {
            Message::Assistant(msg) => {
                for block in &msg.content {
                    if let ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result(result) => {
                if let Some(cost) = result.total_cost_usd {
                    if cost > 0.0 {
                        println!("\nCost: ${:.4}", cost);
                    }
                }
            }
            _ => {}
        }
    }
    println!();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    basic_example().await?;
    with_options_example().await?;
    with_tools_example().await?;

    Ok(())
}
