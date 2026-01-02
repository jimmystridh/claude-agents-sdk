//! Example of using the "include_partial_messages" option to stream partial messages
//! (Rust port of include_partial_messages.py).
//!
//! This feature allows you to receive stream events that contain incremental
//! updates as Claude generates responses. This is useful for:
//! - Building real-time UIs that show text as it's being generated
//! - Monitoring tool use progress
//! - Getting early results before the full response is complete
//!
//! Run with: cargo run --example include_partial_messages

use claude_agents_sdk::{ClaudeClient, ClaudeAgentOptions, Message, UserMessageContent};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Partial Message Streaming Example");
    println!("{}", "=".repeat(50));

    // Enable partial message streaming
    let options = ClaudeAgentOptions::new()
        .with_model("claude-sonnet-4-5")
        .with_max_turns(2)
        .with_partial_messages();

    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await?;

    // Send a prompt that will generate a streaming response
    let prompt = "Think of three jokes, then tell one";
    println!("Prompt: {}\n", prompt);
    println!("{}", "=".repeat(50));

    client.query(prompt).await?;

    while let Some(msg) = client.receive_messages().next().await {
        let msg = msg?;

        match &msg {
            Message::StreamEvent(event) => {
                // Stream events contain partial data
                println!("StreamEvent: {:?}", event);
            }
            Message::Assistant(asst) => {
                println!("AssistantMessage: {} blocks", asst.content.len());
                for block in &asst.content {
                    println!("  Block: {:?}", block);
                }
            }
            Message::User(user) => {
                match &user.content {
                    UserMessageContent::Text(text) => {
                        println!("UserMessage (text): {:?}", text);
                    }
                    UserMessageContent::Blocks(blocks) => {
                        println!("UserMessage: {} blocks", blocks.len());
                    }
                }
            }
            Message::System(sys) => {
                println!("SystemMessage: subtype={:?}", sys.subtype);
            }
            Message::Result(result) => {
                println!("ResultMessage: subtype={:?}, cost={:?}", result.subtype, result.total_cost_usd);
                break;
            }
        }
    }

    client.disconnect().await?;

    Ok(())
}
