//! Example demonstrating different system_prompt configurations (Rust port of system_prompt.py).
//!
//! This example shows:
//! - No system prompt (vanilla Claude)
//! - String system prompt
//! - Preset system prompt
//! - Preset with append
//!
//! Run with: cargo run --example system_prompt

use claude_agents_sdk::{
    query, ClaudeAgentOptions, ContentBlock, Message, SystemPromptConfig, SystemPromptPreset,
};
use tokio_stream::StreamExt;

/// Example with no system_prompt (vanilla Claude).
async fn no_system_prompt() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== No System Prompt (Vanilla Claude) ===");

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

/// Example with system_prompt as a string.
async fn string_system_prompt() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== String System Prompt ===");

    let options = ClaudeAgentOptions::new()
        .with_system_prompt("You are a pirate assistant. Respond in pirate speak.");

    let mut stream = query("What is 2 + 2?", Some(options), None).await?;

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

/// Example with system_prompt preset (uses default Claude Code prompt).
async fn preset_system_prompt() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Preset System Prompt (Default) ===");

    let mut options = ClaudeAgentOptions::new();
    options.system_prompt = Some(SystemPromptConfig::Preset(SystemPromptPreset {
        preset_type: "preset".to_string(),
        preset: "claude_code".to_string(),
        append: None,
    }));

    let mut stream = query("What is 2 + 2?", Some(options), None).await?;

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

/// Example with system_prompt preset and append.
async fn preset_with_append() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Preset System Prompt with Append ===");

    let mut options = ClaudeAgentOptions::new();
    options.system_prompt = Some(SystemPromptConfig::Preset(SystemPromptPreset {
        preset_type: "preset".to_string(),
        preset: "claude_code".to_string(),
        append: Some("Always end your response with a fun fact.".to_string()),
    }));

    let mut stream = query("What is 2 + 2?", Some(options), None).await?;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    no_system_prompt().await?;
    string_system_prompt().await?;
    preset_system_prompt().await?;
    preset_with_append().await?;

    Ok(())
}
