//! Example demonstrating the tools option and verifying tools in system message
//! (Rust port of tools_option.py).
//!
//! Run with: cargo run --example tools_option

use claude_agents_sdk::{
    query, ClaudeAgentOptions, ContentBlock, Message, ToolsConfig, ToolsPreset,
};
use tokio_stream::StreamExt;

/// Example with tools as array of specific tool names.
async fn tools_array_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Tools Array Example ===");
    println!("Setting tools=['Read', 'Glob', 'Grep']");
    println!();

    let mut options = ClaudeAgentOptions::new();
    options.tools = Some(ToolsConfig::List(vec![
        "Read".to_string(),
        "Glob".to_string(),
        "Grep".to_string(),
    ]));
    options.max_turns = Some(1);

    let mut stream = query(
        "What tools do you have available? Just list them briefly.",
        Some(options),
        None,
    )
    .await?;

    while let Some(message) = stream.next().await {
        match message? {
            Message::System(sys) => {
                if sys.subtype == "init" {
                    if let Some(tools) = sys.data.get("tools").and_then(|v| v.as_array()) {
                        let tool_names: Vec<&str> = tools
                            .iter()
                            .filter_map(|t| t.as_str())
                            .collect();
                        println!("Tools from system message: {:?}", tool_names);
                        println!();
                    }
                }
            }
            Message::Assistant(msg) => {
                for block in &msg.content {
                    if let ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result(result) => {
                if let Some(cost) = result.total_cost_usd {
                    println!("\nCost: ${:.4}", cost);
                }
            }
            _ => {}
        }
    }
    println!();

    Ok(())
}

/// Example with tools as empty array (disables all built-in tools).
async fn tools_empty_array_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Tools Empty Array Example ===");
    println!("Setting tools=[] (disables all built-in tools)");
    println!();

    let mut options = ClaudeAgentOptions::new();
    options.tools = Some(ToolsConfig::List(Vec::new()));
    options.max_turns = Some(1);

    let mut stream = query(
        "What tools do you have available? Just list them briefly.",
        Some(options),
        None,
    )
    .await?;

    while let Some(message) = stream.next().await {
        match message? {
            Message::System(sys) => {
                if sys.subtype == "init" {
                    if let Some(tools) = sys.data.get("tools").and_then(|v| v.as_array()) {
                        let tool_names: Vec<&str> = tools
                            .iter()
                            .filter_map(|t| t.as_str())
                            .collect();
                        println!("Tools from system message: {:?}", tool_names);
                        println!();
                    }
                }
            }
            Message::Assistant(msg) => {
                for block in &msg.content {
                    if let ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result(result) => {
                if let Some(cost) = result.total_cost_usd {
                    println!("\nCost: ${:.4}", cost);
                }
            }
            _ => {}
        }
    }
    println!();

    Ok(())
}

/// Example with tools preset (all default Claude Code tools).
async fn tools_preset_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Tools Preset Example ===");
    println!("Setting tools preset to 'claude_code'");
    println!();

    let mut options = ClaudeAgentOptions::new();
    options.tools = Some(ToolsConfig::Preset(ToolsPreset {
        preset_type: "preset".to_string(),
        preset: "claude_code".to_string(),
    }));
    options.max_turns = Some(1);

    let mut stream = query(
        "What tools do you have available? Just list them briefly.",
        Some(options),
        None,
    )
    .await?;

    while let Some(message) = stream.next().await {
        match message? {
            Message::System(sys) => {
                if sys.subtype == "init" {
                    if let Some(tools) = sys.data.get("tools").and_then(|v| v.as_array()) {
                        let tool_names: Vec<&str> = tools
                            .iter()
                            .filter_map(|t| t.as_str())
                            .take(5)
                            .collect();
                        println!(
                            "Tools from system message ({} tools): {:?}...",
                            tools.len(),
                            tool_names
                        );
                        println!();
                    }
                }
            }
            Message::Assistant(msg) => {
                for block in &msg.content {
                    if let ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result(result) => {
                if let Some(cost) = result.total_cost_usd {
                    println!("\nCost: ${:.4}", cost);
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
    tools_array_example().await?;
    tools_empty_array_example().await?;
    tools_preset_example().await?;

    Ok(())
}
