//! Example of using hooks with Claude Agents SDK (Rust port of hooks.py).
//!
//! This file demonstrates basic hook patterns using the hooks configuration
//! in ClaudeAgentOptions. Note: The hook system allows you to intercept and
//! modify Claude's behavior at various points in the execution flow.
//!
//! Run with: cargo run --example hooks
//!
//! Note: This is a simplified demonstration. The full hook system supports:
//! - PreToolUse: Before a tool is executed
//! - PostToolUse: After a tool completes
//! - UserPromptSubmit: When a user submits a prompt
//! - Stop: When execution is stopping
//! - SubagentStop: When a subagent stops
//! - PreCompact: Before context compaction

use claude_agents_sdk::{
    ClaudeAgentOptions, ClaudeClient, ContentBlock, HookCallback, HookEvent, HookInput,
    HookMatcher, HookOutput, Message, SyncHookOutput,
};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

/// Display a message in a standardized format.
#[allow(dead_code)]
fn display_message(msg: &Message) {
    match msg {
        Message::Assistant(asst) => {
            for block in &asst.content {
                if let ContentBlock::Text(text) = block {
                    println!("Claude: {}", text.text);
                }
            }
        }
        Message::Result(_) => {
            println!("Result ended");
        }
        _ => {}
    }
}

/// Create a hook callback that logs and allows all tool uses.
fn logging_hook() -> HookCallback {
    Arc::new(|input, _tool_use_id, _context| {
        Box::pin(async move {
            // Log the hook being called
            match input {
                HookInput::PreToolUse(pre) => {
                    println!("🔧 Hook: PreToolUse for tool: {}", pre.tool_name);
                    println!("   Tool input: {:?}", pre.tool_input);
                }
                HookInput::PostToolUse(post) => {
                    println!("✓ Hook: PostToolUse for tool: {}", post.tool_name);
                }
                HookInput::UserPromptSubmit(_submit) => {
                    println!("📝 Hook: UserPromptSubmit");
                }
                HookInput::Stop(stop) => {
                    println!("🛑 Hook: Stop - active: {:?}", stop.stop_hook_active);
                }
                _ => {
                    println!("📌 Hook: Other event");
                }
            }

            // Return default output (continue execution)
            HookOutput::Sync(SyncHookOutput::default())
        })
    })
}

/// Create a hook callback that blocks certain dangerous commands.
fn security_hook() -> HookCallback {
    Arc::new(|input, _tool_use_id, _context| {
        Box::pin(async move {
            if let HookInput::PreToolUse(pre) = input {
                // Check for dangerous patterns
                if pre.tool_name == "Bash" {
                    if let Some(command) = pre.tool_input.get("command").and_then(|v| v.as_str()) {
                        let dangerous_patterns = ["rm -rf", "sudo", "chmod 777"];
                        for pattern in dangerous_patterns {
                            if command.contains(pattern) {
                                println!(
                                    "⚠️  Security hook: Blocking dangerous command: {}",
                                    command
                                );
                                return HookOutput::Sync(SyncHookOutput {
                                    continue_: Some(false),
                                    decision: Some("block".to_string()),
                                    reason: Some(format!(
                                        "Command contains dangerous pattern: {}",
                                        pattern
                                    )),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }

            // Allow by default
            HookOutput::Sync(SyncHookOutput::default())
        })
    })
}

/// Example demonstrating basic hook usage.
async fn example_basic_hooks() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic Hooks Example ===");
    println!("This example shows hooks logging tool usage.\n");

    // Configure hooks using HashMap<HookEvent, Vec<HookMatcher>>
    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![HookMatcher {
            matcher: None, // Match all tools
            hooks: vec![logging_hook()],
            timeout: None,
        }],
    );
    hooks.insert(
        HookEvent::PostToolUse,
        vec![HookMatcher {
            matcher: None,
            hooks: vec![logging_hook()],
            timeout: None,
        }],
    );

    let mut options = ClaudeAgentOptions::new();
    options.allowed_tools = vec!["Bash".to_string()];
    options.hooks = Some(hooks);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await?;

    println!("User: Run echo hello");
    client
        .query("Run the bash command: echo 'Hello from hooks!'")
        .await?;

    let (response, _) = client.receive_response().await?;
    println!("Claude: {}\n", response);

    client.disconnect().await?;

    Ok(())
}

/// Example demonstrating security hooks.
async fn example_security_hooks() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Security Hooks Example ===");
    println!("This example shows how hooks can block dangerous commands.\n");

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![HookMatcher {
            matcher: Some("Bash".to_string()),
            hooks: vec![security_hook()],
            timeout: None,
        }],
    );

    let mut options = ClaudeAgentOptions::new();
    options.allowed_tools = vec!["Bash".to_string()];
    options.hooks = Some(hooks);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await?;

    // Test 1: Safe command
    println!("Test 1: Safe command (should work)");
    println!("User: Run ls");
    client.query("Run: ls -la").await?;
    let (response, _) = client.receive_response().await?;
    println!("Claude: {}\n", response);

    // Test 2: A command that mentions dangerous pattern
    println!("Test 2: Command with dangerous pattern (should be blocked)");
    println!("User: Run rm -rf /tmp/test");
    client.query("Run: rm -rf /tmp/test").await?;
    let (response, _) = client.receive_response().await?;
    println!("Claude: {}\n", response);

    client.disconnect().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    println!("Claude SDK Hooks Examples");
    println!("{}\n", "=".repeat(50));

    if args.len() < 2 {
        println!("Usage: cargo run --example hooks <example_name>");
        println!("\nAvailable examples:");
        println!("  all        - Run all examples");
        println!("  basic      - Basic hook logging");
        println!("  security   - Security hooks demo");
        return Ok(());
    }

    match args[1].as_str() {
        "all" => {
            example_basic_hooks().await?;
            println!("{}\n", "-".repeat(50));
            example_security_hooks().await?;
        }
        "basic" => {
            example_basic_hooks().await?;
        }
        "security" => {
            example_security_hooks().await?;
        }
        other => {
            println!("Unknown example: {}", other);
            println!("Available: all, basic, security");
        }
    }

    Ok(())
}
