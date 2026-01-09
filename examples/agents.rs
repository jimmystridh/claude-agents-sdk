//! Example of using custom agents with Claude Agents SDK (Rust port of agents.py).
//!
//! This example demonstrates how to define and use custom agents with specific
//! tools, prompts, and models.
//!
//! Run with: cargo run --example agents

use claude_agents_sdk::{
    query, AgentDefinition, AgentModel, ClaudeAgentOptions, ContentBlock, Message,
};
use std::collections::HashMap;
use tokio_stream::StreamExt;

/// Example using a custom code reviewer agent.
async fn code_reviewer_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Code Reviewer Agent Example ===");

    let mut agents = HashMap::new();
    agents.insert(
        "code-reviewer".to_string(),
        AgentDefinition {
            description: "Reviews code for best practices and potential issues".to_string(),
            prompt: "You are a code reviewer. Analyze code for bugs, performance issues, \
                     security vulnerabilities, and adherence to best practices. \
                     Provide constructive feedback."
                .to_string(),
            tools: Some(vec!["Read".to_string(), "Grep".to_string()]),
            model: Some(AgentModel::Sonnet),
        },
    );

    let mut options = ClaudeAgentOptions::new();
    options.agents = Some(agents);

    let mut stream = query(
        "Use the code-reviewer agent to review the code in src/lib.rs",
        Some(options),
    )
    .await?;

    while let Some(message) = stream.next().await {
        match message? {
            Message::Assistant(msg) => {
                for block in &msg.content {
                    if let ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result(result) => {
                if let Some(cost) = result.total_cost_usd {
                    if cost > 0.0 {
                        println!("\nCost: ${:.4}", cost);
                    }
                }
            }
            _ => {}
        }
    }
    println!();

    Ok(())
}

/// Example using a documentation writer agent.
async fn documentation_writer_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Documentation Writer Agent Example ===");

    let mut agents = HashMap::new();
    agents.insert(
        "doc-writer".to_string(),
        AgentDefinition {
            description: "Writes comprehensive documentation".to_string(),
            prompt: "You are a technical documentation expert. Write clear, comprehensive \
                     documentation with examples. Focus on clarity and completeness."
                .to_string(),
            tools: Some(vec![
                "Read".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
            ]),
            model: Some(AgentModel::Sonnet),
        },
    );

    let mut options = ClaudeAgentOptions::new();
    options.agents = Some(agents);

    let mut stream = query(
        "Use the doc-writer agent to explain what AgentDefinition is used for",
        Some(options),
    )
    .await?;

    while let Some(message) = stream.next().await {
        match message? {
            Message::Assistant(msg) => {
                for block in &msg.content {
                    if let ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result(result) => {
                if let Some(cost) = result.total_cost_usd {
                    if cost > 0.0 {
                        println!("\nCost: ${:.4}", cost);
                    }
                }
            }
            _ => {}
        }
    }
    println!();

    Ok(())
}

/// Example with multiple custom agents.
async fn multiple_agents_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Multiple Agents Example ===");

    let mut agents = HashMap::new();
    agents.insert(
        "analyzer".to_string(),
        AgentDefinition {
            description: "Analyzes code structure and patterns".to_string(),
            prompt: "You are a code analyzer. Examine code structure, patterns, and architecture."
                .to_string(),
            tools: Some(vec![
                "Read".to_string(),
                "Grep".to_string(),
                "Glob".to_string(),
            ]),
            model: None, // Use default model
        },
    );
    agents.insert(
        "tester".to_string(),
        AgentDefinition {
            description: "Creates and runs tests".to_string(),
            prompt: "You are a testing expert. Write comprehensive tests and ensure code quality."
                .to_string(),
            tools: Some(vec![
                "Read".to_string(),
                "Write".to_string(),
                "Bash".to_string(),
            ]),
            model: Some(AgentModel::Sonnet),
        },
    );

    let mut options = ClaudeAgentOptions::new();
    options.agents = Some(agents);
    options.setting_sources = Some(vec![
        claude_agents_sdk::SettingSource::User,
        claude_agents_sdk::SettingSource::Project,
    ]);

    let mut stream = query(
        "Use the analyzer agent to find all Rust files in the examples/ directory",
        Some(options),
    )
    .await?;

    while let Some(message) = stream.next().await {
        match message? {
            Message::Assistant(msg) => {
                for block in &msg.content {
                    if let ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result(result) => {
                if let Some(cost) = result.total_cost_usd {
                    if cost > 0.0 {
                        println!("\nCost: ${:.4}", cost);
                    }
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
    code_reviewer_example().await?;
    documentation_writer_example().await?;
    multiple_agents_example().await?;

    Ok(())
}
