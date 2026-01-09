//! Example demonstrating setting sources control (Rust port of setting_sources.py).
//!
//! This example shows how to use the setting_sources option to control which
//! settings are loaded, including custom slash commands, agents, and other
//! configurations.
//!
//! Setting sources determine where Claude Code loads configurations from:
//! - "user": Global user settings (~/.claude/)
//! - "project": Project-level settings (.claude/ in project)
//! - "local": Local gitignored settings (.claude-local/)
//!
//! IMPORTANT: When setting_sources is not provided (None), NO settings are loaded
//! by default. This creates an isolated environment.
//!
//! Run with: cargo run --example setting_sources [example_name]

use claude_agents_sdk::{ClaudeAgentOptions, ClaudeClient, Message, SettingSource};
use std::env;
use std::future::Future;
use std::pin::Pin;
use tokio_stream::StreamExt;

/// Type alias for async example functions.
type ExampleFn =
    fn() -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send>>;

/// Extract slash command names from system message.
fn extract_slash_commands(msg: &claude_agents_sdk::SystemMessage) -> Vec<String> {
    if msg.subtype == "init" {
        if let Some(commands) = msg.data.get("slash_commands") {
            if let Some(arr) = commands.as_array() {
                return arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }
    }
    Vec::new()
}

/// Default behavior - no settings loaded.
async fn example_default() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Default Behavior Example ===");
    println!("Setting sources: None (default)");
    println!("Expected: No custom slash commands will be available\n");

    let sdk_dir = env::current_dir()?;

    let mut options = ClaudeAgentOptions::new();
    options.cwd = Some(sdk_dir);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await?;

    client.query("What is 2 + 2?").await?;

    while let Some(msg) = client.receive_messages().next().await {
        let msg = msg?;
        if let Message::System(sys) = &msg {
            if sys.subtype == "init" {
                let commands = extract_slash_commands(sys);
                println!("Available slash commands: {:?}", commands);
                if commands.contains(&"commit".to_string()) {
                    println!("❌ /commit is available (unexpected)");
                } else {
                    println!("✓ /commit is NOT available (expected - no settings loaded)");
                }
                break;
            }
        }
    }

    client.disconnect().await?;
    println!();

    Ok(())
}

/// Load only user-level settings, excluding project settings.
async fn example_user_only() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== User Settings Only Example ===");
    println!("Setting sources: ['user']");
    println!("Expected: Project slash commands (like /commit) will NOT be available\n");

    let sdk_dir = env::current_dir()?;

    let mut options = ClaudeAgentOptions::new();
    options.setting_sources = Some(vec![SettingSource::User]);
    options.cwd = Some(sdk_dir);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await?;

    client.query("What is 2 + 2?").await?;

    while let Some(msg) = client.receive_messages().next().await {
        let msg = msg?;
        if let Message::System(sys) = &msg {
            if sys.subtype == "init" {
                let commands = extract_slash_commands(sys);
                println!("Available slash commands: {:?}", commands);
                if commands.contains(&"commit".to_string()) {
                    println!("❌ /commit is available (unexpected)");
                } else {
                    println!("✓ /commit is NOT available (expected)");
                }
                break;
            }
        }
    }

    client.disconnect().await?;
    println!();

    Ok(())
}

/// Load both project and user settings.
async fn example_project_and_user() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Project + User Settings Example ===");
    println!("Setting sources: ['user', 'project']");
    println!("Expected: Project slash commands (like /commit) WILL be available\n");

    let sdk_dir = env::current_dir()?;

    let mut options = ClaudeAgentOptions::new();
    options.setting_sources = Some(vec![SettingSource::User, SettingSource::Project]);
    options.cwd = Some(sdk_dir);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await?;

    client.query("What is 2 + 2?").await?;

    while let Some(msg) = client.receive_messages().next().await {
        let msg = msg?;
        if let Message::System(sys) = &msg {
            if sys.subtype == "init" {
                let commands = extract_slash_commands(sys);
                println!("Available slash commands: {:?}", commands);
                if commands.contains(&"commit".to_string()) {
                    println!("✓ /commit is available (expected)");
                } else {
                    println!("❌ /commit is NOT available (unexpected)");
                }
                break;
            }
        }
    }

    client.disconnect().await?;
    println!();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let examples: Vec<(&str, ExampleFn)> = vec![
        ("default", || Box::pin(example_default())),
        ("user_only", || Box::pin(example_user_only())),
        ("project_and_user", || Box::pin(example_project_and_user())),
    ];

    println!("Starting Claude SDK Setting Sources Examples...");
    println!("{}\n", "=".repeat(50));

    if args.len() < 2 {
        println!("Usage: cargo run --example setting_sources <example_name>");
        println!("\nAvailable examples:");
        println!("  all - Run all examples");
        for (name, _) in &examples {
            println!("  {}", name);
        }
        return Ok(());
    }

    let example_name = &args[1];

    if example_name == "all" {
        for (_, func) in &examples {
            func().await?;
            println!("{}\n", "-".repeat(50));
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
