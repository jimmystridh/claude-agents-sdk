//! Example: Calculator MCP Server (Rust port of mcp_calculator.py).
//!
//! This example demonstrates how to create an in-process MCP server with
//! calculator tools using the Claude Agents SDK.
//!
//! Unlike external MCP servers that require separate processes, this server
//! runs directly within your Rust application, providing better performance
//! and simpler deployment.
//!
//! Run with: cargo run --example mcp_calculator --features mcp

#[cfg(feature = "mcp")]
use claude_agents_sdk::{
    mcp::{create_sdk_mcp_server, SdkMcpTool, McpSdkServerConfig},
    ClaudeClient, ClaudeAgentOptions, ContentBlock, Message,
};
#[cfg(feature = "mcp")]
use std::sync::Arc;
#[cfg(feature = "mcp")]
use tokio_stream::StreamExt;

#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("This example requires the 'mcp' feature.");
    eprintln!("Run with: cargo run --example mcp_calculator --features mcp");
    std::process::exit(1);
}

#[cfg(feature = "mcp")]
fn display_message(msg: &Message) {
    match msg {
        Message::User(user) => {
            for block in &user.content {
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
                        println!("Tool Result: {}", content_preview);
                    }
                    _ => {}
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
                        println!("Using tool: {}", tool.name);
                        if !tool.input.is_null() {
                            println!("  Input: {}", tool.input);
                        }
                    }
                    _ => {}
                }
            }
        }
        Message::System(_) => {
            // Ignore system messages
        }
        Message::Result(result) => {
            println!("Result ended");
            if let Some(cost) = result.total_cost_usd {
                println!("Cost: ${:.6}", cost);
            }
        }
        Message::StreamEvent(_) => {}
    }
}

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define calculator tools
    let add_tool = SdkMcpTool::new(
        "add",
        "Add two numbers",
        serde_json::json!({
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "First number"},
                "b": {"type": "number", "description": "Second number"}
            },
            "required": ["a", "b"]
        }),
        Arc::new(|args| {
            Box::pin(async move {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let result = a + b;
                Ok(serde_json::json!({
                    "content": [{"type": "text", "text": format!("{} + {} = {}", a, b, result)}]
                }))
            })
        }),
    );

    let subtract_tool = SdkMcpTool::new(
        "subtract",
        "Subtract one number from another",
        serde_json::json!({
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "First number"},
                "b": {"type": "number", "description": "Number to subtract"}
            },
            "required": ["a", "b"]
        }),
        Arc::new(|args| {
            Box::pin(async move {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let result = a - b;
                Ok(serde_json::json!({
                    "content": [{"type": "text", "text": format!("{} - {} = {}", a, b, result)}]
                }))
            })
        }),
    );

    let multiply_tool = SdkMcpTool::new(
        "multiply",
        "Multiply two numbers",
        serde_json::json!({
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "First number"},
                "b": {"type": "number", "description": "Second number"}
            },
            "required": ["a", "b"]
        }),
        Arc::new(|args| {
            Box::pin(async move {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let result = a * b;
                Ok(serde_json::json!({
                    "content": [{"type": "text", "text": format!("{} × {} = {}", a, b, result)}]
                }))
            })
        }),
    );

    let divide_tool = SdkMcpTool::new(
        "divide",
        "Divide one number by another",
        serde_json::json!({
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "Dividend"},
                "b": {"type": "number", "description": "Divisor"}
            },
            "required": ["a", "b"]
        }),
        Arc::new(|args| {
            Box::pin(async move {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                if b == 0.0 {
                    return Ok(serde_json::json!({
                        "content": [{"type": "text", "text": "Error: Division by zero is not allowed"}],
                        "is_error": true
                    }));
                }
                let result = a / b;
                Ok(serde_json::json!({
                    "content": [{"type": "text", "text": format!("{} ÷ {} = {}", a, b, result)}]
                }))
            })
        }),
    );

    let sqrt_tool = SdkMcpTool::new(
        "sqrt",
        "Calculate square root",
        serde_json::json!({
            "type": "object",
            "properties": {
                "n": {"type": "number", "description": "Number to find square root of"}
            },
            "required": ["n"]
        }),
        Arc::new(|args| {
            Box::pin(async move {
                let n = args.get("n").and_then(|v| v.as_f64()).unwrap_or(0.0);
                if n < 0.0 {
                    return Ok(serde_json::json!({
                        "content": [{"type": "text", "text": format!("Error: Cannot calculate square root of negative number {}", n)}],
                        "is_error": true
                    }));
                }
                let result = n.sqrt();
                Ok(serde_json::json!({
                    "content": [{"type": "text", "text": format!("√{} = {}", n, result)}]
                }))
            })
        }),
    );

    let power_tool = SdkMcpTool::new(
        "power",
        "Raise a number to a power",
        serde_json::json!({
            "type": "object",
            "properties": {
                "base": {"type": "number", "description": "Base number"},
                "exponent": {"type": "number", "description": "Exponent"}
            },
            "required": ["base", "exponent"]
        }),
        Arc::new(|args| {
            Box::pin(async move {
                let base = args.get("base").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let exponent = args.get("exponent").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let result = base.powf(exponent);
                Ok(serde_json::json!({
                    "content": [{"type": "text", "text": format!("{}^{} = {}", base, exponent, result)}]
                }))
            })
        }),
    );

    // Create the calculator server with all tools
    let calculator_config = McpSdkServerConfig {
        name: "calculator".to_string(),
        version: "2.0.0".to_string(),
        tools: vec![
            add_tool,
            subtract_tool,
            multiply_tool,
            divide_tool,
            sqrt_tool,
            power_tool,
        ],
    };

    let calculator_server = create_sdk_mcp_server(calculator_config);

    // Configure Claude to use the calculator server with allowed tools
    let mut options = ClaudeAgentOptions::new();

    // Add the MCP server
    if let claude_agents_sdk::McpServersConfig::Map(ref mut servers) = options.mcp_servers {
        servers.insert("calc".to_string(), calculator_server);
    }

    // Pre-approve all calculator MCP tools
    options.allowed_tools = vec![
        "mcp__calc__add".to_string(),
        "mcp__calc__subtract".to_string(),
        "mcp__calc__multiply".to_string(),
        "mcp__calc__divide".to_string(),
        "mcp__calc__sqrt".to_string(),
        "mcp__calc__power".to_string(),
    ];

    // Example prompts to demonstrate calculator usage
    let prompts = [
        "List your tools",
        "Calculate 15 + 27",
        "What is 100 divided by 7?",
        "Calculate the square root of 144",
        "What is 2 raised to the power of 8?",
        "Calculate (12 + 8) * 3 - 10", // Complex calculation
    ];

    for prompt in prompts {
        println!("\n{}", "=".repeat(50));
        println!("Prompt: {}", prompt);
        println!("{}", "=".repeat(50));

        let mut client = ClaudeClient::new(Some(options.clone()), None);
        client.connect().await?;

        client.query(prompt).await?;

        while let Some(msg) = client.receive_messages().next().await {
            let msg = msg?;
            display_message(&msg);
            if matches!(msg, Message::Result(_)) {
                break;
            }
        }

        client.disconnect().await?;
    }

    Ok(())
}
