//! Example demonstrating max_budget_usd option for cost control
//! (Rust port of max_budget_usd.py).
//!
//! Run with: cargo run --example max_budget_usd

use claude_agents_sdk::{query, ClaudeAgentOptions, ContentBlock, Message};
use tokio_stream::StreamExt;

/// Example without budget limit.
async fn without_budget() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Without Budget Limit ===");

    let mut stream = query("What is 2 + 2?", None).await?;

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
                    println!("Total cost: ${:.4}", cost);
                }
                println!("Status: {:?}", result.subtype);
            }
            _ => {}
        }
    }
    println!();

    Ok(())
}

/// Example with budget that won't be exceeded.
async fn with_reasonable_budget() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== With Reasonable Budget ($0.10) ===");

    let mut options = ClaudeAgentOptions::new();
    options.max_budget_usd = Some(0.10); // 10 cents - plenty for a simple query

    let mut stream = query("What is 2 + 2?", Some(options)).await?;

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
                    println!("Total cost: ${:.4}", cost);
                }
                println!("Status: {:?}", result.subtype);
            }
            _ => {}
        }
    }
    println!();

    Ok(())
}

/// Example with very tight budget that will likely be exceeded.
async fn with_tight_budget() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== With Tight Budget ($0.0001) ===");

    let mut options = ClaudeAgentOptions::new();
    options.max_budget_usd = Some(0.0001); // Very small budget - will be exceeded quickly

    let mut stream = query("Read the README.md file and summarize it", Some(options)).await?;

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
                    println!("Total cost: ${:.4}", cost);
                }
                println!("Status: {:?}", result.subtype);

                // Check if budget was exceeded
                if result.subtype == "error_max_budget_usd" {
                    println!("⚠️  Budget limit exceeded!");
                    println!("Note: The cost may exceed the budget by up to one API call's worth");
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
    println!("This example demonstrates using max_budget_usd to control API costs.\n");

    without_budget().await?;
    with_reasonable_budget().await?;
    with_tight_budget().await?;

    println!(
        "\nNote: Budget checking happens after each API call completes,\n\
         so the final cost may slightly exceed the specified budget.\n"
    );

    Ok(())
}
