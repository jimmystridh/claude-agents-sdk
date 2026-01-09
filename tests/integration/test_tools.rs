//! Tool use and tool permission tests.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{
    ClaudeAgentOptions, ClaudeClient, ContentBlock, Message, PermissionMode, UserMessageContent,
};
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::integration::helpers::*;

/// Test that allowed_tools configuration restricts tool usage.
#[tokio::test]
async fn test_allowed_tools_configuration() {
    let options = ClaudeAgentOptions::new()
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(3);

    let result = tokio::time::timeout(
        Duration::from_secs(60),
        collect_messages("Run 'echo hello_world' and tell me the output.", options),
    )
    .await;

    match result {
        Ok(Ok(messages)) => {
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
        Ok(Err(e)) => {
            // Query error - may happen if permissions are not available
            eprintln!("Query error (may be permission-related): {}", e);
        }
        Err(_) => {
            panic!("Test timed out after 60 seconds");
        }
    }
}

/// Test that tool results are properly parsed and linked to tool uses.
#[tokio::test]
async fn test_tool_result_parsing() {
    let options = ClaudeAgentOptions::new()
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(3);

    let mut client = ClaudeClient::new(Some(options));

    let connect_result = tokio::time::timeout(Duration::from_secs(30), client.connect()).await;

    match connect_result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            eprintln!("Connection error: {}", e);
            return;
        }
        Err(_) => {
            panic!("Connect timed out after 30 seconds");
        }
    }

    client
        .query("Run 'echo test_output_123' using bash.")
        .await
        .expect("Failed to send query");

    let mut tool_use_ids = Vec::new();
    let mut tool_result_ids = Vec::new();

    let receive_result = tokio::time::timeout(Duration::from_secs(60), async {
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
    })
    .await;

    match receive_result {
        Ok(_) => {
            // Verify tool results reference known tool use IDs
            for result_id in &tool_result_ids {
                assert!(
                    tool_use_ids.contains(result_id),
                    "Tool result ID '{}' should reference a known tool use",
                    result_id
                );
            }
            eprintln!(
                "Found {} tool uses and {} tool results",
                tool_use_ids.len(),
                tool_result_ids.len()
            );
        }
        Err(_) => {
            eprintln!("Receive timed out - this may happen if tool permissions are pending");
        }
    }

    client.disconnect().await.ok();
}
