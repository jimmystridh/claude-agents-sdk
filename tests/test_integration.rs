//! Integration tests for Claude SDK.
//!
//! These tests verify end-to-end functionality with mocked transport responses.

use claude_agents_sdk::{
    AssistantMessage, ClaudeAgentOptions, ContentBlock, Message, ResultMessage, TextBlock,
    ToolUseBlock,
};

/// Create a mock assistant message for testing.
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

/// Create a mock result message for testing.
fn create_result_message(session_id: &str, cost: f64) -> ResultMessage {
    ResultMessage {
        subtype: "success".to_string(),
        duration_ms: 1000,
        duration_api_ms: 800,
        is_error: false,
        num_turns: 1,
        session_id: session_id.to_string(),
        total_cost_usd: Some(cost),
        usage: None,
        result: None,
        structured_output: None,
    }
}

#[test]
fn test_simple_query_response_parsing() {
    // Test parsing a simple text response
    let assistant = create_assistant_message("2 + 2 equals 4");

    assert_eq!(assistant.content.len(), 1);
    assert_eq!(assistant.text(), "2 + 2 equals 4");
    assert_eq!(assistant.model, "claude-opus-4-1-20250805");
}

#[test]
fn test_result_message_parsing() {
    // Test parsing a result message
    let result = create_result_message("test-session", 0.001);

    assert_eq!(result.subtype, "success");
    assert_eq!(result.session_id, "test-session");
    assert_eq!(result.total_cost_usd, Some(0.001));
    assert!(!result.is_error);
}

#[test]
fn test_query_with_tool_use() {
    // Test parsing a response with tool use
    let assistant = AssistantMessage {
        content: vec![
            ContentBlock::Text(TextBlock {
                text: "Let me read that file for you.".to_string(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "tool-123".to_string(),
                name: "Read".to_string(),
                input: serde_json::json!({"file_path": "/test.txt"}),
            }),
        ],
        model: "claude-opus-4-1-20250805".to_string(),
        parent_tool_use_id: None,
        error: None,
    };

    assert_eq!(assistant.content.len(), 2);
    assert_eq!(assistant.text(), "Let me read that file for you.");

    let tool_uses = assistant.tool_uses();
    assert_eq!(tool_uses.len(), 1);
    assert_eq!(tool_uses[0].name, "Read");
    assert_eq!(
        tool_uses[0].input["file_path"].as_str(),
        Some("/test.txt")
    );
}

#[test]
fn test_options_with_continuation() {
    // Test creating options with continuation
    let options = ClaudeAgentOptions::new();
    assert!(!options.continue_conversation);

    let mut options = ClaudeAgentOptions::new();
    options.continue_conversation = true;
    options.resume = Some("session-123".to_string());

    assert!(options.continue_conversation);
    assert_eq!(options.resume, Some("session-123".to_string()));
}

#[test]
fn test_options_with_max_budget() {
    // Test creating options with max budget
    let mut options = ClaudeAgentOptions::new();
    options.max_budget_usd = Some(0.0001);

    assert_eq!(options.max_budget_usd, Some(0.0001));
}

#[test]
fn test_budget_exceeded_result() {
    // Test parsing a budget exceeded result
    let result = ResultMessage {
        subtype: "error_max_budget_usd".to_string(),
        duration_ms: 500,
        duration_api_ms: 400,
        is_error: false,
        num_turns: 1,
        session_id: "test-session-budget".to_string(),
        total_cost_usd: Some(0.0002),
        usage: Some(serde_json::json!({
            "input_tokens": 100,
            "output_tokens": 50,
        })),
        result: None,
        structured_output: None,
    };

    assert_eq!(result.subtype, "error_max_budget_usd");
    assert!(!result.is_error); // Budget exceeded is not considered an error
    assert!(result.total_cost_usd.is_some());
    assert!(result.total_cost_usd.unwrap() > 0.0);
}

#[test]
fn test_message_json_parsing() {
    // Test parsing messages from JSON (as they would come from CLI)
    let json = r#"{
        "type": "assistant",
        "content": [{"type": "text", "text": "Hello, world!"}],
        "model": "claude-3"
    }"#;

    let msg: Message = serde_json::from_str(json).unwrap();
    assert!(msg.is_assistant());

    if let Message::Assistant(asst) = msg {
        assert_eq!(asst.text(), "Hello, world!");
    } else {
        panic!("Expected assistant message");
    }
}

#[test]
fn test_result_json_parsing() {
    let json = r#"{
        "type": "result",
        "subtype": "success",
        "duration_ms": 1000,
        "duration_api_ms": 800,
        "is_error": false,
        "num_turns": 1,
        "session_id": "test-session",
        "total_cost_usd": 0.001
    }"#;

    let msg: Message = serde_json::from_str(json).unwrap();
    assert!(msg.is_result());

    if let Message::Result(result) = msg {
        assert_eq!(result.subtype, "success");
        assert_eq!(result.session_id, "test-session");
        assert_eq!(result.total_cost_usd, Some(0.001));
    } else {
        panic!("Expected result message");
    }
}

#[test]
fn test_options_with_allowed_tools() {
    let options = ClaudeAgentOptions::new().with_allowed_tools(vec![
        "Read".to_string(),
        "Write".to_string(),
    ]);

    assert_eq!(options.allowed_tools, vec!["Read", "Write"]);
}

#[test]
fn test_options_with_cwd() {
    let options = ClaudeAgentOptions::new().with_cwd("/custom/path");

    assert_eq!(
        options.cwd,
        Some(std::path::PathBuf::from("/custom/path"))
    );
}

#[test]
fn test_options_with_max_turns() {
    let options = ClaudeAgentOptions::new().with_max_turns(5);

    assert_eq!(options.max_turns, Some(5));
}

#[test]
fn test_options_with_permission_mode() {
    use claude_agents_sdk::PermissionMode;

    let options = ClaudeAgentOptions::new().with_permission_mode(PermissionMode::AcceptEdits);

    assert_eq!(options.permission_mode, Some(PermissionMode::AcceptEdits));
}

#[test]
fn test_options_with_system_prompt() {
    let options = ClaudeAgentOptions::new().with_system_prompt("Be helpful");

    match options.system_prompt {
        Some(claude_agents_sdk::SystemPromptConfig::Text(text)) => {
            assert_eq!(text, "Be helpful");
        }
        _ => panic!("Expected text system prompt"),
    }
}

#[test]
fn test_tool_use_block_serialization() {
    let tool_use = ToolUseBlock {
        id: "tool-123".to_string(),
        name: "Bash".to_string(),
        input: serde_json::json!({"command": "ls -la"}),
    };

    let json = serde_json::to_string(&tool_use).unwrap();
    assert!(json.contains("tool-123"));
    assert!(json.contains("Bash"));
    assert!(json.contains("ls -la"));
}

#[test]
fn test_assistant_message_multiple_content_blocks() {
    let assistant = AssistantMessage {
        content: vec![
            ContentBlock::Text(TextBlock {
                text: "First ".to_string(),
            }),
            ContentBlock::Text(TextBlock {
                text: "Second".to_string(),
            }),
        ],
        model: "claude-3".to_string(),
        parent_tool_use_id: None,
        error: None,
    };

    // text() should concatenate all text blocks
    assert_eq!(assistant.text(), "First Second");
}
