//! Comprehensive examples of using ClaudeClient for streaming mode (Rust port of streaming_mode.py).
//!
//! This file demonstrates various patterns for building applications with
//! the ClaudeClient streaming interface.
//!
//! Run with: cargo run --example streaming_mode [example_name]
//!
//! Available examples:
//! - all: Run all examples
//! - basic_streaming
//! - multi_turn
//! - with_interrupt
//! - with_options
//! - bash_command
//! - control_protocol
//! - error_handling

use claude_agents_sdk::{ClaudeClient, ClaudeAgentOptions, ContentBlock, Message, UserMessageContent};
use std::env;
use std::future::Future;
use std::pin::Pin;
use tokio_stream::StreamExt;

/// Type alias for async example functions.
type ExampleFn = fn() -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send>>;

/// Display a message in a standardized format.
fn display_message(msg: &Message) {
    match msg {
        Message::User(user) => {
            match &user.content {
                UserMessageContent::Text(text) => {
                    println!("User: {}", text);
                }
                UserMessageContent::Blocks(blocks) => {
                    for block in blocks {
                        if let ContentBlock::Text(text) = block {
                            println!("User: {}", text.text);
                        }
                    }
                }
            }
        }
        Message::Assistant(asst) => {
            for block in &asst.content {
                if let ContentBlock::Text(text) = block {
                    println!("Claude: {}", text.text);
                }
            }
        }
        Message::System(_) => {
            // Ignore system messages
        }
        Message::Result(_) => {
            println!("Result ended");
        }
        Message::StreamEvent(_) => {
            // Streaming events handled separately
        }
    }
}

/// Basic streaming with ClaudeClient.
async fn example_basic_streaming() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic Streaming Example ===");

    let mut client = ClaudeClient::new(None, None);
    client.connect().await?;

    println!("User: What is 2+2?");
    client.query("What is 2+2?").await?;

    let (response, _) = client.receive_response().await?;
    println!("Claude: {}", response);

    client.disconnect().await?;
    println!("\n");

    Ok(())
}

/// Multi-turn conversation using receive_response helper.
async fn example_multi_turn_conversation() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Multi-Turn Conversation Example ===");

    let mut client = ClaudeClient::new(None, None);
    client.connect().await?;

    // First turn
    println!("User: What's the capital of France?");
    client.query("What's the capital of France?").await?;

    let (response, _) = client.receive_response().await?;
    println!("Claude: {}", response);

    // Second turn - follow-up
    println!("\nUser: What's the population of that city?");
    client.query("What's the population of that city?").await?;

    let (response, _) = client.receive_response().await?;
    println!("Claude: {}", response);

    client.disconnect().await?;
    println!("\n");

    Ok(())
}

/// Demonstrate interrupt capability.
async fn example_with_interrupt() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Interrupt Example ===");
    println!("IMPORTANT: Interrupts require active message consumption.");

    let mut client = ClaudeClient::new(None, None);
    client.connect().await?;

    // Start a long-running task
    println!("\nUser: Count from 1 to 100 slowly");
    client
        .query("Count from 1 to 100 slowly, with a brief pause between each number")
        .await?;

    // Spawn a task to consume messages for a short time
    let start = std::time::Instant::now();

    let mut message_stream = client.receive_messages();
    while start.elapsed() < std::time::Duration::from_secs(2) {
        tokio::select! {
            msg = message_stream.next() => {
                if let Some(Ok(msg)) = msg {
                    display_message(&msg);
                    if matches!(msg, Message::Result(_)) {
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
        }
    }
    drop(message_stream);

    println!("\n[After 2 seconds, sending interrupt...]");
    client.interrupt().await?;

    // Drain remaining messages
    while let Some(Ok(msg)) = client.receive_messages().next().await {
        if matches!(msg, Message::Result(_)) {
            break;
        }
    }

    // Send new instruction after interrupt
    println!("\nUser: Never mind, just tell me a quick joke");
    client.query("Never mind, just tell me a quick joke").await?;

    let (response, _) = client.receive_response().await?;
    println!("Claude: {}", response);

    client.disconnect().await?;
    println!("\n");

    Ok(())
}

/// Use ClaudeAgentOptions to configure the client.
async fn example_with_options() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Custom Options Example ===");

    let options = ClaudeAgentOptions::new()
        .with_allowed_tools(vec!["Read".to_string(), "Write".to_string()])
        .with_system_prompt("You are a helpful coding assistant.");

    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await?;

    println!("User: Create a simple hello.txt file with a greeting message");
    client
        .query("Create a simple hello.txt file with a greeting message")
        .await?;

    let mut tool_uses = Vec::new();

    while let Some(msg) = client.receive_messages().next().await {
        let msg = msg?;
        match &msg {
            Message::Assistant(asst) => {
                display_message(&msg);
                for block in &asst.content {
                    if let ContentBlock::ToolUse(tool) = block {
                        tool_uses.push(tool.name.clone());
                    }
                }
            }
            Message::Result(_) => {
                display_message(&msg);
                break;
            }
            _ => display_message(&msg),
        }
    }

    if !tool_uses.is_empty() {
        println!("Tools used: {}", tool_uses.join(", "));
    }

    client.disconnect().await?;
    println!("\n");

    Ok(())
}

/// Example showing tool use blocks when running bash commands.
async fn example_bash_command() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Bash Command Example ===");

    let options = ClaudeAgentOptions::new()
        .with_allowed_tools(vec!["Bash".to_string()]);

    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await?;

    println!("User: Run a bash echo command");
    client
        .query("Run a bash echo command that says 'Hello from bash!'")
        .await?;

    let mut message_types = Vec::new();

    while let Some(msg) = client.receive_messages().next().await {
        let msg = msg?;
        message_types.push(format!("{:?}", std::mem::discriminant(&msg)));

        match &msg {
            Message::User(user) => {
                match &user.content {
                    UserMessageContent::Text(text) => {
                        println!("User: {}", text);
                    }
                    UserMessageContent::Blocks(blocks) => {
                        for block in blocks {
                            match block {
                                ContentBlock::Text(text) => {
                                    println!("User: {}", text.text);
                                }
                                ContentBlock::ToolResult(result) => {
                                    let content_preview = result
                                        .content
                                        .as_ref()
                                        .map(|c| {
                                            let s = c.to_string();
                                            if s.len() > 100 {
                                                format!("{}...", &s[..100])
                                            } else {
                                                s
                                            }
                                        })
                                        .unwrap_or_else(|| "None".to_string());
                                    println!("Tool Result (id: {}): {}", result.tool_use_id, content_preview);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Message::Assistant(asst) => {
                for block in &asst.content {
                    match block {
                        ContentBlock::Text(text) => {
                            println!("Claude: {}", text.text);
                        }
                        ContentBlock::ToolUse(tool) => {
                            println!("Tool Use: {} (id: {})", tool.name, tool.id);
                            if tool.name == "Bash" {
                                if let Some(cmd) = tool.input.get("command") {
                                    println!("  Command: {}", cmd);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Message::Result(result) => {
                println!("Result ended");
                if let Some(cost) = result.total_cost_usd {
                    println!("Cost: ${:.4}", cost);
                }
                break;
            }
            _ => {}
        }
    }

    println!("\nMessage types received: {:?}", message_types.len());

    client.disconnect().await?;
    println!("\n");

    Ok(())
}

/// Demonstrate server info and interrupt capabilities.
async fn example_control_protocol() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Control Protocol Example ===");
    println!("Shows server info retrieval and interrupt capability\n");

    let mut client = ClaudeClient::new(None, None);
    client.connect().await?;

    // 1. Get server initialization info
    println!("1. Getting server info...");
    if let Some(server_info) = client.get_server_info().await {
        println!("✓ Server info retrieved successfully!");
        if let Some(commands) = server_info.get("commands").and_then(|v| v.as_array()) {
            println!("  - Available commands: {}", commands.len());
        }
        if let Some(style) = server_info.get("output_style") {
            println!("  - Output style: {}", style);
        }
    } else {
        println!("✗ No server info available (may not be in streaming mode)");
    }

    println!("\n2. Testing interrupt capability...");

    // Start a long-running task
    println!("User: Count from 1 to 20 slowly");
    client
        .query("Count from 1 to 20 slowly, pausing between each number")
        .await?;

    // Start consuming messages for a short time
    let start = std::time::Instant::now();
    let mut message_stream = client.receive_messages();
    while start.elapsed() < std::time::Duration::from_secs(2) {
        tokio::select! {
            msg = message_stream.next() => {
                if let Some(Ok(msg)) = msg {
                    if let Message::Assistant(asst) = &msg {
                        for block in &asst.content {
                            if let ContentBlock::Text(text) = block {
                                let preview = if text.text.len() > 50 {
                                    format!("{}...", &text.text[..50])
                                } else {
                                    text.text.clone()
                                };
                                println!("Claude: {}", preview);
                                break;
                            }
                        }
                    }
                    if matches!(msg, Message::Result(_)) {
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
        }
    }
    drop(message_stream);

    println!("\n[Sending interrupt after 2 seconds...]");

    match client.interrupt().await {
        Ok(()) => println!("✓ Interrupt sent successfully"),
        Err(e) => println!("✗ Interrupt failed: {}", e),
    }

    // Drain remaining messages
    while let Some(Ok(msg)) = client.receive_messages().next().await {
        if matches!(msg, Message::Result(_)) {
            break;
        }
    }

    // Send new query after interrupt
    println!("\nUser: Just say 'Hello!'");
    client.query("Just say 'Hello!'").await?;

    let (response, _) = client.receive_response().await?;
    println!("Claude: {}", response);

    client.disconnect().await?;
    println!("\n");

    Ok(())
}

/// Demonstrate proper error handling.
async fn example_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Error Handling Example ===");

    let mut client = ClaudeClient::new(None, None);

    match client.connect().await {
        Ok(()) => {
            // Send a message that will take time to process
            println!("User: Run a bash sleep command for 60 seconds");
            client
                .query("Run a bash sleep command for 60 seconds not in the background")
                .await?;

            // Try to receive response with a short timeout
            let mut messages_count = 0;
            let timeout = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                async {
                    while let Some(msg) = client.receive_messages().next().await {
                        let msg = msg?;
                        messages_count += 1;

                        match &msg {
                            Message::Assistant(asst) => {
                                for block in &asst.content {
                                    if let ContentBlock::Text(text) = block {
                                        let preview = if text.text.len() > 50 {
                                            format!("{}...", &text.text[..50])
                                        } else {
                                            text.text.clone()
                                        };
                                        println!("Claude: {}", preview);
                                    }
                                }
                            }
                            Message::Result(_) => {
                                display_message(&msg);
                                return Ok::<_, claude_agents_sdk::ClaudeSDKError>(());
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                }
            ).await;

            match timeout {
                Ok(Ok(())) => println!("Response completed successfully"),
                Ok(Err(e)) => println!("Error during response: {}", e),
                Err(_) => {
                    println!(
                        "\nResponse timeout after 10 seconds - demonstrating graceful handling"
                    );
                    println!("Received {} messages before timeout", messages_count);
                }
            }

            // Always disconnect
            client.disconnect().await?;
        }
        Err(e) => {
            println!("Connection error: {}", e);
        }
    }

    println!("\n");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let examples: Vec<(&str, ExampleFn)> = vec![
        ("basic_streaming", || Box::pin(example_basic_streaming())),
        ("multi_turn", || Box::pin(example_multi_turn_conversation())),
        ("with_interrupt", || Box::pin(example_with_interrupt())),
        ("with_options", || Box::pin(example_with_options())),
        ("bash_command", || Box::pin(example_bash_command())),
        ("control_protocol", || Box::pin(example_control_protocol())),
        ("error_handling", || Box::pin(example_error_handling())),
    ];

    if args.len() < 2 {
        println!("Usage: cargo run --example streaming_mode <example_name>");
        println!("\nAvailable examples:");
        println!("  all - Run all examples");
        for (name, _) in &examples {
            println!("  {}", name);
        }
        return Ok(());
    }

    let example_name = &args[1];

    if example_name == "all" {
        for (name, func) in &examples {
            println!("Running: {}", name);
            func().await?;
            println!("{}", "-".repeat(50));
        }
    } else {
        let found = examples.iter().find(|(name, _)| name == example_name);
        match found {
            Some((_, func)) => {
                func().await?;
            }
            None => {
                println!("Error: Unknown example '{}'", example_name);
                println!("\nAvailable examples:");
                println!("  all - Run all examples");
                for (name, _) in &examples {
                    println!("  {}", name);
                }
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
