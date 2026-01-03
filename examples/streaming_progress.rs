//! Streaming with Progress Indicators Example.
//!
//! This example demonstrates how to show progress while streaming
//! responses from Claude, including spinners and progress bars.
//!
//! Run with: cargo run --example streaming_progress

use claude_agents_sdk::{query, ClaudeAgentOptions, Message, PermissionMode};
use std::io::{self, Write};
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Streaming with Progress Example ===\n");

    // Example 1: Simple spinner while waiting
    println!("--- Example 1: Spinner Progress ---");
    spinner_progress().await?;

    // Example 2: Character count progress
    println!("\n--- Example 2: Character Count Progress ---");
    char_count_progress().await?;

    // Example 3: Token-style streaming with timing
    println!("\n--- Example 3: Timed Streaming ---");
    timed_streaming().await?;

    // Example 4: Multi-query progress
    println!("\n--- Example 4: Multi-Query Progress ---");
    multi_query_progress().await?;

    println!("\n=== All examples completed ===");
    Ok(())
}

/// Show a spinner while waiting for the first response.
async fn spinner_progress() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let mut stream = query(
        "Write a haiku about Rust programming.",
        Some(options),
        None,
    )
    .await?;

    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut spinner_idx = 0;
    let mut first_response = true;
    let mut response = String::new();

    // Spinner task - runs until first content arrives
    let spinner_task = tokio::spawn(async move {
        loop {
            print!("\r{} Waiting for response...", spinner_chars[spinner_idx]);
            io::stdout().flush().ok();
            spinner_idx = (spinner_idx + 1) % spinner_chars.len();
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
    });

    while let Some(msg) = stream.next().await {
        match msg? {
            Message::Assistant(asst) => {
                if first_response {
                    spinner_task.abort();
                    print!("\r                              \r"); // Clear spinner line
                    first_response = false;
                }
                let text = asst.text();
                print!("{}", text);
                io::stdout().flush()?;
                response.push_str(&text);
            }
            Message::Result(result) => {
                println!();
                println!("  [Completed in {}ms]", result.duration_ms);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Show character count as response streams in.
async fn char_count_progress() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let mut stream = query(
        "List 5 benefits of learning Rust. Keep it brief.",
        Some(options),
        None,
    )
    .await?;

    let mut char_count = 0;
    let mut response = String::new();
    let start = Instant::now();

    println!("Streaming response:");
    println!("{}", "-".repeat(50));

    while let Some(msg) = stream.next().await {
        match msg? {
            Message::Assistant(asst) => {
                let text = asst.text();
                char_count += text.len();
                response.push_str(&text);
                print!("{}", text);
                io::stdout().flush()?;
            }
            Message::Result(result) => {
                let elapsed = start.elapsed();
                println!();
                println!("{}", "-".repeat(50));
                println!(
                    "  {} chars | {:.1} chars/sec | {}ms total",
                    char_count,
                    char_count as f64 / elapsed.as_secs_f64(),
                    result.duration_ms
                );
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Show timing information as tokens stream in.
async fn timed_streaming() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let mut stream = query("Explain what a closure is in one sentence.", Some(options), None).await?;

    let start = Instant::now();
    let mut last_update = start;
    let mut chunk_count = 0;
    let mut response = String::new();

    print!("Response: ");
    io::stdout().flush()?;

    while let Some(msg) = stream.next().await {
        match msg? {
            Message::Assistant(asst) => {
                let text = asst.text();
                response.push_str(&text);
                chunk_count += 1;

                // Print the text
                print!("{}", text);
                io::stdout().flush()?;

                // Show timing periodically
                let now = Instant::now();
                if now.duration_since(last_update) > Duration::from_secs(1) {
                    print!(" [{:.1}s]", start.elapsed().as_secs_f64());
                    io::stdout().flush()?;
                    last_update = now;
                }
            }
            Message::Result(result) => {
                let elapsed = start.elapsed();
                println!();
                println!(
                    "  [Done: {} chunks in {:.2}s, avg {:.0}ms/chunk]",
                    chunk_count,
                    elapsed.as_secs_f64(),
                    elapsed.as_millis() as f64 / chunk_count.max(1) as f64
                );
                if let Some(cost) = result.total_cost_usd {
                    println!("  [Cost: ${:.6}]", cost);
                }
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Show progress across multiple queries.
async fn multi_query_progress() -> Result<(), Box<dyn std::error::Error>> {
    let queries = [
        "What is 1+1? Answer with just the number.",
        "What is 2+2? Answer with just the number.",
        "What is 3+3? Answer with just the number.",
    ];

    let total = queries.len();
    let mut completed = 0;
    let mut total_time_ms = 0u64;

    for (i, q) in queries.iter().enumerate() {
        // Progress bar
        let progress = (i as f64 / total as f64 * 20.0) as usize;
        print!(
            "\r[{}{}] {}/{} queries ",
            "█".repeat(progress),
            "░".repeat(20 - progress),
            i,
            total
        );
        io::stdout().flush()?;

        let options = ClaudeAgentOptions::new()
            .with_permission_mode(PermissionMode::Default)
            .with_max_turns(1);

        match query(q, Some(options), None).await {
            Ok(mut stream) => {
                while let Some(msg) = stream.next().await {
                    if let Ok(Message::Result(result)) = msg {
                        completed += 1;
                        total_time_ms += result.duration_ms;
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("\nQuery {} failed: {}", i + 1, e);
            }
        }
    }

    // Final progress bar
    print!("\r[{}] {}/{} queries ", "█".repeat(20), total, total);
    println!("✓");
    println!(
        "  Completed {} queries in {}ms (avg {}ms/query)",
        completed,
        total_time_ms,
        total_time_ms / completed.max(1)
    );

    Ok(())
}
