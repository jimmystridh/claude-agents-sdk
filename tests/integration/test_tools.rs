//! Tool use and tool permission tests.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{
    ClaudeAgentOptions, ClaudeClient, ContentBlock, Message, PermissionMode,
    UserMessageContent,
};
use tokio_stream::StreamExt;

use crate::integration::helpers::*;

/// Test that allowed_tools configuration restricts tool usage.
#[tokio::test]
async fn test_allowed_tools_configuration() {
    let options = ClaudeAgentOptions::new()
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_permission_mode(PermissionMode::BypassPermissions)
        .with_max_turns(3);

    let messages = collect_messages("Run 'echo hello_world' and tell me the output.", options)
        .await
        .expect("Query failed");

    // Check that if any tool was used, it was Bash
    for msg in &messages {
        if let Message::Assistant(asst) = msg {
            for block in &asst.content {
                if let ContentBlock::ToolUse(tool) = block {
                    assert_eq!(
                        tool.name, "Bash",
                        "Should only use allowed Bash tool, got: {}",
                        tool.name
                    );
                }
            }
        }
    }

    let result = get_result(&messages).expect("Should receive result");
    assert!(!result.is_error);
}

/// Test that tool results are properly parsed and linked to tool uses.
#[tokio::test]
async fn test_tool_result_parsing() {
    let options = ClaudeAgentOptions::new()
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_permission_mode(PermissionMode::BypassPermissions)
        .with_max_turns(3);

    let mut client = ClaudeClient::new(Some(options), None);
    client.connect().await.expect("Failed to connect");

    client
        .query("Run 'echo test_output_123' using bash.")
        .await
        .expect("Failed to send query");

    let mut tool_use_ids = Vec::new();
    let mut tool_result_ids = Vec::new();

    while let Some(msg) = client.receive_messages().next().await {
        match msg.expect("Error in stream") {
            Message::Assistant(asst) => {
                for block in &asst.content {
                    if let ContentBlock::ToolUse(tool) = block {
                        tool_use_ids.push(tool.id.clone());
                    }
                }
            }
            Message::User(user) => {
                if let UserMessageContent::Blocks(blocks) = &user.content {
                    for block in blocks {
                        if let ContentBlock::ToolResult(result) = block {
                            tool_result_ids.push(result.tool_use_id.clone());
                        }
                    }
                }
            }
            Message::Result(_) => break,
            _ => {}
        }
    }

    // Verify tool results reference known tool use IDs
    for result_id in &tool_result_ids {
        assert!(
            tool_use_ids.contains(result_id),
            "Tool result ID '{}' should reference a known tool use",
            result_id
        );
    }

    client.disconnect().await.expect("Failed to disconnect");
}
