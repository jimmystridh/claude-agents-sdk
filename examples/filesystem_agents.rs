//! Example of loading filesystem-based agents via setting_sources
//! (Rust port of filesystem_agents.py).
//!
//! This example demonstrates how to load agents defined in .claude/agents/ files
//! using the setting_sources option. This is different from inline AgentDefinition
//! objects - these agents are loaded from markdown files on disk.
//!
//! Run with: cargo run --example filesystem_agents

use claude_agents_sdk::{ClaudeAgentOptions, ClaudeClient, ContentBlock, Message, SettingSource};
use std::env;
use tokio_stream::StreamExt;

/// Extract agent names from system message init data.
fn extract_agents(msg: &claude_agents_sdk::SystemMessage) -> Vec<String> {
    if msg.subtype == "init" {
        if let Some(agents) = msg.data.get("agents") {
            if let Some(arr) = agents.as_array() {
                return arr
                    .iter()
                    .filter_map(|a| {
                        if let Some(s) = a.as_str() {
                            Some(s.to_string())
                        } else if let Some(obj) = a.as_object() {
                            obj.get("name")
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }
    }
    Vec::new()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Filesystem Agents Example ===");
    println!("Testing: setting_sources=['project'] with .claude/agents/test-agent.md");
    println!();

    // Use the SDK repo directory which might have .claude/agents/test-agent.md
    let sdk_dir = env::current_dir()?;

    let mut options = ClaudeAgentOptions::new();
    options.setting_sources = Some(vec![SettingSource::Project]);
    options.cwd = Some(sdk_dir);

    let mut message_types: Vec<String> = Vec::new();
    let mut agents_found: Vec<String> = Vec::new();

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await?;

    client.query("Say hello in exactly 3 words").await?;

    while let Some(msg) = client.receive_messages().next().await {
        let msg = msg?;
        message_types.push(format!("{:?}", std::mem::discriminant(&msg)));

        match &msg {
            Message::System(sys) => {
                if sys.subtype == "init" {
                    agents_found = extract_agents(sys);
                    println!("Init message received. Agents loaded: {:?}", agents_found);
                }
            }
            Message::Assistant(asst) => {
                for block in &asst.content {
                    if let ContentBlock::Text(text) = block {
                        println!("Assistant: {}", text.text);
                    }
                }
            }
            Message::Result(result) => {
                println!(
                    "Result: subtype={:?}, cost=${:.4}",
                    result.subtype,
                    result.total_cost_usd.unwrap_or(0.0)
                );
                break;
            }
            _ => {}
        }
    }

    client.disconnect().await?;

    println!();
    println!("=== Summary ===");
    println!("Message types received: {:?}", message_types);
    println!("Total messages: {}", message_types.len());

    // Validate the results
    let has_init = message_types.iter().any(|t| t.contains("System"));
    let has_assistant = message_types.iter().any(|t| t.contains("Assistant"));
    let has_result = message_types.iter().any(|t| t.contains("Result"));
    let has_test_agent = agents_found.contains(&"test-agent".to_string());

    println!();
    if has_init && has_assistant && has_result {
        println!("SUCCESS: Received full response (init, assistant, result)");
    } else {
        println!("FAILURE: Did not receive full response");
        println!("  - Init: {}", has_init);
        println!("  - Assistant: {}", has_assistant);
        println!("  - Result: {}", has_result);
    }

    if has_test_agent {
        println!("SUCCESS: test-agent was loaded from filesystem");
    } else {
        println!("WARNING: test-agent was NOT loaded (may not exist in .claude/agents/)");
    }

    Ok(())
}
