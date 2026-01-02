//! Simple example demonstrating stderr callback for capturing CLI debug output
//! (Rust port of stderr_callback_example.py).
//!
//! Run with: cargo run --example stderr_callback

use claude_agents_sdk::{query, ClaudeAgentOptions, ContentBlock, Message};
use std::sync::{Arc, Mutex};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect stderr messages
    let stderr_messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let stderr_clone = stderr_messages.clone();

    // Create the stderr callback
    let stderr_callback: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |message| {
        let mut messages = stderr_clone.lock().unwrap();
        messages.push(message.clone());

        // Optionally print specific messages
        if message.contains("[ERROR]") {
            println!("Error detected: {}", message);
        }
    });

    // Create options with stderr callback and enable debug mode
    let mut options = ClaudeAgentOptions::new();
    options.stderr = Some(stderr_callback);
    options.extra_args.insert("debug-to-stderr".to_string(), None); // Enable debug output

    // Run a query
    println!("Running query with stderr capture...");

    let mut stream = query("What is 2+2?", Some(options), None).await?;

    while let Some(message) = stream.next().await {
        if let Message::Assistant(msg) = message? {
            for block in &msg.content {
                if let ContentBlock::Text(text) = block {
                    println!("Response: {}", text.text);
                }
            }
        }
    }

    // Show what we captured
    let messages = stderr_messages.lock().unwrap();
    println!("\nCaptured {} stderr lines", messages.len());

    if !messages.is_empty() {
        let first_line = &messages[0];
        let preview = if first_line.len() > 100 {
            format!("{}...", &first_line[..100])
        } else {
            first_line.clone()
        };
        println!("First stderr line: {}", preview);
    }

    Ok(())
}
