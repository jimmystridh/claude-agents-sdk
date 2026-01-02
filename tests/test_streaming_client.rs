//! Tests for ClaudeClient streaming functionality.

use claude_agents_sdk::{
    AssistantMessage, ClaudeAgentOptions, ContentBlock, Message, ResultMessage, SystemMessage,
    TextBlock, UserMessage, UserMessageContent,
};

/// Helper to create an assistant message with text.
fn create_assistant_message(text: &str) -> AssistantMessage {
    AssistantMessage {
        content: vec![ContentBlock::Text(TextBlock {
            text: text.to_string(),
        })],
        model: "claude-opus-4-1-20250805".to_string(),
        parent_tool_use_id: None,
        error: None,
    }
}

/// Helper to create a result message.
fn create_result_message(success: bool) -> ResultMessage {
    ResultMessage {
        subtype: if success {
            "success".to_string()
        } else {
            "error".to_string()
        },
        duration_ms: 1000,
        duration_api_ms: 800,
        is_error: !success,
        num_turns: 1,
        session_id: "test-session".to_string(),
        total_cost_usd: Some(0.001),
        usage: None,
        result: None,
        structured_output: None,
    }
}

/// Helper to create a system init message.
fn create_system_init() -> SystemMessage {
    SystemMessage {
        subtype: "init".to_string(),
        data: serde_json::json!({
            "tools": ["Read", "Write", "Bash"],
            "session_id": "test-session",
        }),
    }
}

#[test]
fn test_message_type_discrimination() {
    let assistant = Message::Assistant(create_assistant_message("Hello"));
    assert!(assistant.is_assistant());
    assert!(!assistant.is_result());

    let result = Message::Result(create_result_message(true));
    assert!(result.is_result());
    assert!(!result.is_assistant());
}

#[test]
fn test_assistant_message_text() {
    let msg = create_assistant_message("Test response");
    assert_eq!(msg.text(), "Test response");
}

#[test]
fn test_assistant_message_multiple_text_blocks() {
    let msg = AssistantMessage {
        content: vec![
            ContentBlock::Text(TextBlock {
                text: "First ".to_string(),
            }),
            ContentBlock::Text(TextBlock {
                text: "Second ".to_string(),
            }),
            ContentBlock::Text(TextBlock {
                text: "Third".to_string(),
            }),
        ],
        model: "claude-3".to_string(),
        parent_tool_use_id: None,
        error: None,
    };

    assert_eq!(msg.text(), "First Second Third");
}

#[test]
fn test_result_message_success() {
    let result = create_result_message(true);
    assert_eq!(result.subtype, "success");
    assert!(!result.is_error);
}

#[test]
fn test_result_message_error() {
    let result = create_result_message(false);
    assert_eq!(result.subtype, "error");
    assert!(result.is_error);
}

#[test]
fn test_system_message_init() {
    let sys = create_system_init();
    assert_eq!(sys.subtype, "init");
    assert!(sys.data.get("tools").is_some());
}

#[test]
fn test_user_message_text_content() {
    let user = UserMessage {
        content: UserMessageContent::Text("Hello from user".to_string()),
        uuid: None,
        parent_tool_use_id: None,
    };

    match user.content {
        UserMessageContent::Text(text) => {
            assert_eq!(text, "Hello from user");
        }
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_user_message_blocks_content() {
    let user = UserMessage {
        content: UserMessageContent::Blocks(vec![ContentBlock::Text(TextBlock {
            text: "Block text".to_string(),
        })]),
        uuid: None,
        parent_tool_use_id: None,
    };

    match user.content {
        UserMessageContent::Blocks(blocks) => {
            assert_eq!(blocks.len(), 1);
        }
        _ => panic!("Expected blocks content"),
    }
}

#[test]
fn test_user_message_with_uuid() {
    let user = UserMessage {
        content: UserMessageContent::Text("Test".to_string()),
        uuid: Some("unique-id-123".to_string()),
        parent_tool_use_id: None,
    };

    assert_eq!(user.uuid, Some("unique-id-123".to_string()));
}

#[test]
fn test_options_with_partial_messages() {
    let options = ClaudeAgentOptions::new().with_partial_messages();
    assert!(options.include_partial_messages);
}

#[test]
fn test_options_default_no_partial_messages() {
    let options = ClaudeAgentOptions::new();
    assert!(!options.include_partial_messages);
}

#[test]
fn test_message_as_assistant() {
    let msg = Message::Assistant(create_assistant_message("Test"));
    let asst = msg.as_assistant();
    assert!(asst.is_some());
    assert_eq!(asst.unwrap().text(), "Test");

    let result_msg = Message::Result(create_result_message(true));
    assert!(result_msg.as_assistant().is_none());
}

#[test]
fn test_message_as_result() {
    let msg = Message::Result(create_result_message(true));
    let result = msg.as_result();
    assert!(result.is_some());
    assert_eq!(result.unwrap().subtype, "success");

    let asst_msg = Message::Assistant(create_assistant_message("Test"));
    assert!(asst_msg.as_result().is_none());
}

#[test]
fn test_message_system_match() {
    let msg = Message::System(create_system_init());

    // Use pattern matching since there's no as_system method
    if let Message::System(sys) = &msg {
        assert_eq!(sys.subtype, "init");
    } else {
        panic!("Expected system message");
    }

    let asst_msg = Message::Assistant(create_assistant_message("Test"));
    assert!(!matches!(asst_msg, Message::System(_)));
}

#[test]
fn test_message_json_serialization() {
    let msg = Message::Assistant(create_assistant_message("Hello"));
    let json = serde_json::to_string(&msg).unwrap();

    assert!(json.contains("assistant"));
    assert!(json.contains("Hello"));
}

#[test]
fn test_message_json_deserialization() {
    let json = r#"{
        "type": "assistant",
        "content": [{"type": "text", "text": "Deserialized"}],
        "model": "claude-3"
    }"#;

    let msg: Message = serde_json::from_str(json).unwrap();
    assert!(msg.is_assistant());

    if let Message::Assistant(asst) = msg {
        assert_eq!(asst.text(), "Deserialized");
    }
}

#[test]
fn test_result_message_with_usage() {
    let result = ResultMessage {
        subtype: "success".to_string(),
        duration_ms: 1000,
        duration_api_ms: 800,
        is_error: false,
        num_turns: 1,
        session_id: "test-session".to_string(),
        total_cost_usd: Some(0.002),
        usage: Some(serde_json::json!({
            "input_tokens": 150,
            "output_tokens": 75,
            "total_tokens": 225,
        })),
        result: None,
        structured_output: None,
    };

    assert!(result.usage.is_some());
    let usage = result.usage.unwrap();
    assert_eq!(usage["input_tokens"], 150);
    assert_eq!(usage["output_tokens"], 75);
}

#[test]
fn test_client_options_for_query() {
    let options = ClaudeAgentOptions::new()
        .with_model("claude-sonnet-4-5")
        .with_max_turns(5)
        .with_allowed_tools(vec!["Read".to_string(), "Write".to_string()]);

    assert_eq!(options.model, Some("claude-sonnet-4-5".to_string()));
    assert_eq!(options.max_turns, Some(5));
    assert_eq!(options.allowed_tools.len(), 2);
}

#[test]
fn test_client_options_with_custom_session() {
    let mut options = ClaudeAgentOptions::new();
    options.resume = Some("custom-session-id".to_string());

    assert_eq!(options.resume, Some("custom-session-id".to_string()));
}

#[test]
fn test_assistant_message_tool_uses() {
    use claude_agents_sdk::ToolUseBlock;

    let msg = AssistantMessage {
        content: vec![
            ContentBlock::Text(TextBlock {
                text: "Let me help you.".to_string(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "tool-1".to_string(),
                name: "Read".to_string(),
                input: serde_json::json!({"file_path": "/test.txt"}),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "tool-2".to_string(),
                name: "Write".to_string(),
                input: serde_json::json!({"file_path": "/out.txt", "content": "data"}),
            }),
        ],
        model: "claude-3".to_string(),
        parent_tool_use_id: None,
        error: None,
    };

    let tool_uses = msg.tool_uses();
    assert_eq!(tool_uses.len(), 2);
    assert_eq!(tool_uses[0].name, "Read");
    assert_eq!(tool_uses[1].name, "Write");
}

#[test]
fn test_content_block_type_checking() {
    let text_block = ContentBlock::Text(TextBlock {
        text: "Hello".to_string(),
    });
    assert!(matches!(text_block, ContentBlock::Text(_)));

    let tool_use_block = ContentBlock::ToolUse(claude_agents_sdk::ToolUseBlock {
        id: "1".to_string(),
        name: "Test".to_string(),
        input: serde_json::json!({}),
    });
    assert!(matches!(tool_use_block, ContentBlock::ToolUse(_)));

    let tool_result_block = ContentBlock::ToolResult(claude_agents_sdk::ToolResultBlock {
        tool_use_id: "1".to_string(),
        content: Some(serde_json::json!("result")),
        is_error: None,
    });
    assert!(matches!(tool_result_block, ContentBlock::ToolResult(_)));
}

#[test]
fn test_stream_event_message() {
    use claude_agents_sdk::StreamEvent;

    let event = StreamEvent {
        uuid: "test-uuid".to_string(),
        session_id: "test-session".to_string(),
        event: serde_json::json!({
            "type": "content_block_delta",
            "delta": {
                "type": "text_delta",
                "text": "Streaming..."
            }
        }),
        parent_tool_use_id: None,
    };

    assert_eq!(event.uuid, "test-uuid");
    assert!(event.event.get("delta").is_some());
}
