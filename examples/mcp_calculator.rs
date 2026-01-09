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
    mcp::{create_sdk_mcp_server, SdkMcpTool, ToolInputSchema, ToolResult},
    ClaudeAgentOptions, ClaudeClient, ContentBlock, Message, UserMessageContent,
};
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
        Message::User(user) => match &user.content {
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
                            println!("Tool Result: {}", content_preview);
                        }
                        _ => {}
                    }
                }
            }
        },
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
    // Define calculator tools using the proper API
    let add_tool = SdkMcpTool::new(
        "add",
        "Add two numbers",
        ToolInputSchema::object()
            .number_property("a", "First number")
            .number_property("b", "Second number")
            .required_property("a")
            .required_property("b"),
        |input| async move {
            let a = input.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = input.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let result = a + b;
            ToolResult::text(format!("{} + {} = {}", a, b, result))
        },
    );

    let subtract_tool = SdkMcpTool::new(
        "subtract",
        "Subtract one number from another",
        ToolInputSchema::object()
            .number_property("a", "First number")
            .number_property("b", "Number to subtract")
            .required_property("a")
            .required_property("b"),
        |input| async move {
            let a = input.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = input.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let result = a - b;
            ToolResult::text(format!("{} - {} = {}", a, b, result))
        },
    );

    let multiply_tool = SdkMcpTool::new(
        "multiply",
        "Multiply two numbers",
        ToolInputSchema::object()
            .number_property("a", "First number")
            .number_property("b", "Second number")
            .required_property("a")
            .required_property("b"),
        |input| async move {
            let a = input.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = input.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let result = a * b;
            ToolResult::text(format!("{} × {} = {}", a, b, result))
        },
    );

    let divide_tool = SdkMcpTool::new(
        "divide",
        "Divide one number by another",
        ToolInputSchema::object()
            .number_property("a", "Dividend")
            .number_property("b", "Divisor")
            .required_property("a")
            .required_property("b"),
        |input| async move {
            let a = input.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = input.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if b == 0.0 {
                return ToolResult::error("Division by zero is not allowed");
            }
            let result = a / b;
            ToolResult::text(format!("{} ÷ {} = {}", a, b, result))
        },
    );

    let sqrt_tool = SdkMcpTool::new(
        "sqrt",
        "Calculate square root",
        ToolInputSchema::object()
            .number_property("n", "Number to find square root of")
            .required_property("n"),
        |input| async move {
            let n = input.get("n").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if n < 0.0 {
                return ToolResult::error(format!(
                    "Cannot calculate square root of negative number {}",
                    n
                ));
            }
            let result = n.sqrt();
            ToolResult::text(format!("√{} = {}", n, result))
        },
    );

    let power_tool = SdkMcpTool::new(
        "power",
        "Raise a number to a power",
        ToolInputSchema::object()
            .number_property("base", "Base number")
            .number_property("exponent", "Exponent")
            .required_property("base")
            .required_property("exponent"),
        |input| async move {
            let base = input.get("base").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let exponent = input
                .get("exponent")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let result = base.powf(exponent);
            ToolResult::text(format!("{}^{} = {}", base, exponent, result))
        },
    );

    // Create the calculator server with all tools
    let (_config, _tools) = create_sdk_mcp_server(
        "calculator",
        "2.0.0",
        vec![
            add_tool,
            subtract_tool,
            multiply_tool,
            divide_tool,
            sqrt_tool,
            power_tool,
        ],
    );

    // Configure Claude to use the calculator server with allowed tools
    // Note: SDK MCP servers are not yet fully implemented in the control protocol.
    // This example demonstrates the API but may not work end-to-end.
    let options = ClaudeAgentOptions::new().with_allowed_tools(vec![
        "mcp__calc__add".to_string(),
        "mcp__calc__subtract".to_string(),
        "mcp__calc__multiply".to_string(),
        "mcp__calc__divide".to_string(),
        "mcp__calc__sqrt".to_string(),
        "mcp__calc__power".to_string(),
    ]);

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

        let mut client = ClaudeClient::new(Some(options.clone()));
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
