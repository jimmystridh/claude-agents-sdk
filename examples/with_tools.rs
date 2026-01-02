//! Example with tool permission callback.
//!
//! This example demonstrates how to use the tool permission callback
//! to control which tools Claude can use.
//!
//! Run with: cargo run --example with_tools

use claude_agents_sdk::{
    ClaudeClientBuilder,
    Message, PermissionResult, PermissionMode,
};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Claude Tool Permission Example ===\n");

    // Build client with tool permission callback
    let mut client = ClaudeClientBuilder::new()
        .max_turns(10)
        .permission_mode(PermissionMode::Default)
        .can_use_tool(|tool_name, input, _context| async move {
            println!("\n📋 Tool permission requested:");
            println!("   Tool: {}", tool_name);
            println!("   Input: {}", serde_json::to_string_pretty(&input).unwrap_or_default());

            // Example: Allow Read tool, deny Bash for dangerous commands
            match tool_name.as_str() {
                "Read" => {
                    println!("   ✅ Allowing Read tool");
                    PermissionResult::allow()
                }
                "Bash" => {
                    // Check if the command looks dangerous
                    if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                        if cmd.contains("rm ") || cmd.contains("sudo") {
                            println!("   ❌ Denying dangerous Bash command");
                            return PermissionResult::deny_with_message(
                                "Dangerous commands are not allowed"
                            );
                        }
                    }
                    println!("   ✅ Allowing Bash tool");
                    PermissionResult::allow()
                }
                _ => {
                    println!("   ✅ Allowing {} tool", tool_name);
                    PermissionResult::allow()
                }
            }
        })
        .build();

    // Connect
    client.connect().await?;
    println!("Connected to Claude CLI\n");

    // Send a query that will likely use tools
    println!("--- Sending query that may trigger tool use ---\n");
    client.query("List the files in the current directory using ls.").await?;

    // Process the response
    while let Some(msg) = client.receive_messages().next().await {
        match msg? {
            Message::Assistant(asst) => {
                // Print text content
                let text = asst.text();
                if !text.is_empty() {
                    println!("\n💬 Claude: {}", text);
                }

                // Show tool use
                for tool_use in asst.tool_uses() {
                    println!("\n🔧 Claude wants to use tool: {}", tool_use.name);
                }
            }
            Message::Result(result) => {
                println!("\n---");
                println!("Completed in {}ms", result.duration_ms);
                println!("Turns: {}", result.num_turns);
                break;
            }
            _ => {}
        }
    }

    // Disconnect
    client.disconnect().await?;
    println!("\nDisconnected from Claude CLI");

    Ok(())
}
