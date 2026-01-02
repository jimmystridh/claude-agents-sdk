//! Simple one-shot query example.
//!
//! This example demonstrates the simplest way to interact with Claude
//! using the `query` function.
//!
//! Run with: cargo run --example simple_query

use claude_agents_sdk::{query, ClaudeAgentOptions, Message};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional)
    tracing_subscriber::fmt::init();

    println!("Sending query to Claude...\n");

    // Configure options
    let options = ClaudeAgentOptions::new()
        .with_max_turns(3)
        .with_system_prompt("You are a helpful assistant. Be concise.");

    // Send query and process stream
    let mut stream = query("What is the capital of France?", Some(options), None).await?;

    while let Some(message) = stream.next().await {
        match message? {
            Message::Assistant(msg) => {
                // Print assistant's response
                print!("{}", msg.text());
            }
            Message::Result(result) => {
                // Print final stats
                println!("\n\n---");
                println!("Completed in {}ms", result.duration_ms);
                println!("Turns: {}", result.num_turns);
                if let Some(cost) = result.total_cost_usd {
                    println!("Cost: ${:.4}", cost);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
