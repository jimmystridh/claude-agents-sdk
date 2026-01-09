//! Example: Tool Permission Callbacks (Rust port of tool_permission_callback.py).
//!
//! This example demonstrates how to use tool permission callbacks to control
//! which tools Claude can use and modify their inputs.
//!
//! Run with: cargo run --example tool_permission_callback

use claude_agents_sdk::{
    CanUseTool, ClaudeAgentOptions, ClaudeClient, ContentBlock, Message, PermissionResult,
    PermissionResultAllow, PermissionUpdate,
};
use std::sync::{Arc, Mutex};
use tokio_stream::StreamExt;

/// Tool usage log entry.
#[derive(Debug, Clone)]
struct ToolUsageEntry {
    tool: String,
    input: serde_json::Value,
    suggestions: Vec<PermissionUpdate>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "=".repeat(60));
    println!("Tool Permission Callback Example");
    println!("{}", "=".repeat(60));
    println!("\nThis example demonstrates how to:");
    println!("1. Allow/deny tools based on type");
    println!("2. Modify tool inputs for safety");
    println!("3. Log tool usage");
    println!("{}", "=".repeat(60));

    // Track tool usage for demonstration
    let tool_usage_log: Arc<Mutex<Vec<ToolUsageEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let log_clone = tool_usage_log.clone();

    // Create the permission callback
    let permission_callback: CanUseTool = Arc::new(move |tool_name, input_data, context| {
        let log = log_clone.clone();
        let tool_name_clone = tool_name.clone();
        let input_clone = input_data.clone();

        Box::pin(async move {
            // Log the tool request
            {
                let mut log_guard = log.lock().unwrap();
                log_guard.push(ToolUsageEntry {
                    tool: tool_name_clone.clone(),
                    input: input_clone.clone(),
                    suggestions: context.suggestions.clone(),
                });
            }

            println!("\n🔧 Tool Permission Request: {}", tool_name);
            println!(
                "   Input: {}",
                serde_json::to_string_pretty(&input_data).unwrap_or_default()
            );

            // Always allow read operations
            if matches!(tool_name.as_str(), "Read" | "Glob" | "Grep") {
                println!(
                    "   ✅ Automatically allowing {} (read-only operation)",
                    tool_name
                );
                return PermissionResult::allow();
            }

            // Deny write operations to system directories
            if matches!(tool_name.as_str(), "Write" | "Edit" | "MultiEdit") {
                let file_path = input_data
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if file_path.starts_with("/etc/") || file_path.starts_with("/usr/") {
                    println!("   ❌ Denying write to system directory: {}", file_path);
                    return PermissionResult::deny_with_message(format!(
                        "Cannot write to system directory: {}",
                        file_path
                    ));
                }

                // Redirect writes to a safe directory
                if !file_path.starts_with("/tmp/") && !file_path.starts_with("./") {
                    let safe_path = format!(
                        "./safe_output/{}",
                        file_path.rsplit('/').next().unwrap_or("file")
                    );
                    println!(
                        "   ⚠️  Redirecting write from {} to {}",
                        file_path, safe_path
                    );

                    let mut modified_input = input_data.clone();
                    if let Some(obj) = modified_input.as_object_mut() {
                        obj.insert("file_path".to_string(), serde_json::json!(safe_path));
                    }
                    return PermissionResult::Allow(PermissionResultAllow::with_updated_input(
                        modified_input,
                    ));
                }
            }

            // Check dangerous bash commands
            if tool_name == "Bash" {
                let command = input_data
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let dangerous_commands = ["rm -rf", "sudo", "chmod 777", "dd if=", "mkfs"];

                for dangerous in dangerous_commands {
                    if command.contains(dangerous) {
                        println!("   ❌ Denying dangerous command: {}", command);
                        return PermissionResult::deny_with_message(format!(
                            "Dangerous command pattern detected: {}",
                            dangerous
                        ));
                    }
                }

                // Allow but log the command
                println!("   ✅ Allowing bash command: {}", command);
                return PermissionResult::allow();
            }

            // For all other tools, allow by default
            println!("   ✅ Allowing tool: {}", tool_name);
            PermissionResult::allow()
        })
    });

    // Configure options with our callback
    let mut options = ClaudeAgentOptions::new();
    options.can_use_tool = Some(permission_callback);
    // Use default permission mode to ensure callbacks are invoked
    options.permission_mode = Some(claude_agents_sdk::PermissionMode::Default);
    options.cwd = Some(std::path::PathBuf::from("."));

    // Create client and send a query that will use multiple tools
    let mut client = ClaudeClient::new(Some(options));
    client.connect().await?;

    println!("\n📝 Sending query to Claude...");
    client
        .query(
            "Please do the following:\n\
             1. List the files in the current directory\n\
             2. Create a simple Rust hello world script at hello.rs\n\
             3. Run the script to test it",
        )
        .await?;

    println!("\n📨 Receiving response...");
    let mut message_count = 0;

    while let Some(msg) = client.receive_messages().next().await {
        let msg = msg?;
        message_count += 1;

        match &msg {
            Message::Assistant(asst) => {
                for block in &asst.content {
                    if let ContentBlock::Text(text) = block {
                        println!("\n💬 Claude: {}", text.text);
                    }
                }
            }
            Message::Result(result) => {
                println!("\n✅ Task completed!");
                println!("   Duration: {}ms", result.duration_ms);
                if let Some(cost) = result.total_cost_usd {
                    println!("   Cost: ${:.4}", cost);
                }
                println!("   Messages processed: {}", message_count);
                break;
            }
            _ => {}
        }
    }

    client.disconnect().await?;

    // Print tool usage summary
    println!("\n{}", "=".repeat(60));
    println!("Tool Usage Summary");
    println!("{}", "=".repeat(60));

    let log = tool_usage_log.lock().unwrap();
    for (i, usage) in log.iter().enumerate() {
        println!("\n{}. Tool: {}", i + 1, usage.tool);
        println!(
            "   Input: {}",
            serde_json::to_string_pretty(&usage.input).unwrap_or_default()
        );
        if !usage.suggestions.is_empty() {
            println!("   Suggestions: {:?}", usage.suggestions);
        }
    }

    Ok(())
}
