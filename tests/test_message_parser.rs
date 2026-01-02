//! Tests for message parsing.

use claude_agents_sdk::_internal::message_parser::*;
use claude_agents_sdk::*;
use serde_json::json;

#[test]
fn test_parse_user_message_text() {
    let raw = json!({
        "type": "user",
        "message": {
            "content": "Hello, Claude!"
        }
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::User(user) => {
            assert_eq!(user.text(), Some("Hello, Claude!"));
        }
        _ => panic!("Expected user message"),
    }
}

#[test]
fn test_parse_user_message_with_uuid() {
    let raw = json!({
        "type": "user",
        "message": {
            "content": "Hello"
        },
        "uuid": "user_123"
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::User(user) => {
            assert_eq!(user.uuid, Some("user_123".to_string()));
        }
        _ => panic!("Expected user message"),
    }
}

#[test]
fn test_parse_user_message_with_content_blocks() {
    let raw = json!({
        "type": "user",
        "message": {
            "content": [
                {"type": "text", "text": "Hello"},
                {"type": "text", "text": " world"}
            ]
        }
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::User(user) => {
            match &user.content {
                UserMessageContent::Blocks(blocks) => {
                    assert_eq!(blocks.len(), 2);
                }
                _ => panic!("Expected blocks"),
            }
        }
        _ => panic!("Expected user message"),
    }
}

#[test]
fn test_parse_assistant_message_text() {
    let raw = json!({
        "type": "assistant",
        "message": {
            "content": [
                {"type": "text", "text": "Hello, I'm Claude!"}
            ],
            "model": "claude-3-sonnet-20240229"
        }
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::Assistant(asst) => {
            assert_eq!(asst.text(), "Hello, I'm Claude!");
            assert_eq!(asst.model, "claude-3-sonnet-20240229");
        }
        _ => panic!("Expected assistant message"),
    }
}

#[test]
fn test_parse_assistant_message_with_tool_use() {
    let raw = json!({
        "type": "assistant",
        "message": {
            "content": [
                {"type": "text", "text": "Let me check that for you."},
                {
                    "type": "tool_use",
                    "id": "toolu_01234",
                    "name": "Bash",
                    "input": {"command": "ls -la"}
                }
            ],
            "model": "claude-3"
        }
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::Assistant(asst) => {
            assert_eq!(asst.content.len(), 2);
            let tools = asst.tool_uses();
            assert_eq!(tools.len(), 1);
            assert_eq!(tools[0].name, "Bash");
            assert_eq!(tools[0].input["command"], "ls -la");
        }
        _ => panic!("Expected assistant message"),
    }
}

#[test]
fn test_parse_assistant_message_with_thinking() {
    let raw = json!({
        "type": "assistant",
        "message": {
            "content": [
                {
                    "type": "thinking",
                    "thinking": "Let me think about this...",
                    "signature": "sig123"
                },
                {"type": "text", "text": "Here's my answer."}
            ],
            "model": "claude-3-opus"
        }
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::Assistant(asst) => {
            assert_eq!(asst.content.len(), 2);
            match &asst.content[0] {
                ContentBlock::Thinking(thinking) => {
                    assert_eq!(thinking.thinking, "Let me think about this...");
                    assert_eq!(thinking.signature, "sig123");
                }
                _ => panic!("Expected thinking block"),
            }
        }
        _ => panic!("Expected assistant message"),
    }
}

#[test]
fn test_parse_assistant_message_with_error() {
    let raw = json!({
        "type": "assistant",
        "message": {
            "content": [{"type": "text", "text": "Error occurred"}],
            "model": "claude-3",
            "error": "rate_limit"
        }
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::Assistant(asst) => {
            assert_eq!(asst.error, Some(AssistantMessageError::RateLimit));
        }
        _ => panic!("Expected assistant message"),
    }
}

#[test]
fn test_parse_system_message() {
    let raw = json!({
        "type": "system",
        "subtype": "init",
        "data": {"session_id": "sess_123"}
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::System(sys) => {
            assert_eq!(sys.subtype, "init");
            assert_eq!(sys.data["session_id"], "sess_123");
        }
        _ => panic!("Expected system message"),
    }
}

#[test]
fn test_parse_result_message() {
    let raw = json!({
        "type": "result",
        "subtype": "success",
        "duration_ms": 1500,
        "duration_api_ms": 800,
        "is_error": false,
        "num_turns": 5,
        "session_id": "sess_abc123",
        "total_cost_usd": 0.0123,
        "usage": {
            "input_tokens": 100,
            "output_tokens": 200
        },
        "result": "Task completed successfully"
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::Result(result) => {
            assert_eq!(result.subtype, "success");
            assert_eq!(result.duration_ms, 1500);
            assert_eq!(result.duration_api_ms, 800);
            assert!(!result.is_error);
            assert_eq!(result.num_turns, 5);
            assert_eq!(result.session_id, "sess_abc123");
            assert_eq!(result.total_cost_usd, Some(0.0123));
            assert!(result.usage.is_some());
            assert_eq!(result.result, Some("Task completed successfully".to_string()));
        }
        _ => panic!("Expected result message"),
    }
}

#[test]
fn test_parse_result_message_camel_case() {
    // Test that camelCase field names also work
    let raw = json!({
        "type": "result",
        "subtype": "success",
        "durationMs": 1000,
        "durationApiMs": 500,
        "isError": false,
        "numTurns": 2,
        "sessionId": "sess_xyz",
        "totalCostUsd": 0.01
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::Result(result) => {
            assert_eq!(result.duration_ms, 1000);
            assert_eq!(result.duration_api_ms, 500);
            assert!(!result.is_error);
            assert_eq!(result.num_turns, 2);
            assert_eq!(result.session_id, "sess_xyz");
            assert_eq!(result.total_cost_usd, Some(0.01));
        }
        _ => panic!("Expected result message"),
    }
}

#[test]
fn test_parse_stream_event() {
    let raw = json!({
        "type": "stream_event",
        "uuid": "evt_123",
        "session_id": "sess_456",
        "event": {
            "type": "content_block_delta",
            "delta": {"type": "text_delta", "text": "Hello"}
        }
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::StreamEvent(evt) => {
            assert_eq!(evt.uuid, "evt_123");
            assert_eq!(evt.session_id, "sess_456");
            assert_eq!(evt.event["type"], "content_block_delta");
        }
        _ => panic!("Expected stream event"),
    }
}

#[test]
fn test_parse_tool_result_block() {
    let raw = json!({
        "type": "assistant",
        "message": {
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": "toolu_123",
                    "content": "Command output here",
                    "is_error": false
                }
            ],
            "model": "claude-3"
        }
    });

    let msg = parse_message(raw).unwrap();
    match msg {
        Message::Assistant(asst) => {
            match &asst.content[0] {
                ContentBlock::ToolResult(result) => {
                    assert_eq!(result.tool_use_id, "toolu_123");
                    assert_eq!(result.is_error, Some(false));
                }
                _ => panic!("Expected tool result block"),
            }
        }
        _ => panic!("Expected assistant message"),
    }
}

#[test]
fn test_parse_unknown_message_type() {
    let raw = json!({
        "type": "unknown_type",
        "data": {}
    });

    let result = parse_message(raw);
    assert!(result.is_err());
}

#[test]
fn test_parse_missing_type() {
    let raw = json!({
        "content": "Hello"
    });

    let result = parse_message(raw);
    assert!(result.is_err());
}

#[test]
fn test_is_control_request() {
    let raw = json!({
        "type": "control_request",
        "request_id": "req_123",
        "request": {"subtype": "interrupt"}
    });
    assert!(is_control_request(&raw));

    let raw = json!({
        "type": "assistant",
        "content": []
    });
    assert!(!is_control_request(&raw));
}

#[test]
fn test_is_control_response() {
    let raw = json!({
        "type": "control_response",
        "response": {
            "subtype": "success",
            "request_id": "req_123"
        }
    });
    assert!(is_control_response(&raw));

    let raw = json!({
        "type": "result",
        "subtype": "success"
    });
    assert!(!is_control_response(&raw));
}

#[test]
fn test_parse_control_request_can_use_tool() {
    let raw = json!({
        "type": "control_request",
        "request_id": "req_001",
        "request": {
            "subtype": "can_use_tool",
            "tool_name": "Bash",
            "input": {"command": "rm -rf /"},
            "permission_suggestions": null,
            "blocked_path": null
        }
    });

    let request = parse_control_request(raw).unwrap();
    assert_eq!(request.request_id, "req_001");
    match request.request {
        ControlRequestPayload::CanUseTool { tool_name, input, .. } => {
            assert_eq!(tool_name, "Bash");
            assert_eq!(input["command"], "rm -rf /");
        }
        _ => panic!("Expected CanUseTool request"),
    }
}

#[test]
fn test_parse_control_response_success() {
    let raw = json!({
        "type": "control_response",
        "response": {
            "subtype": "success",
            "request_id": "req_123",
            "response": {"initialized": true}
        }
    });

    let response = parse_control_response(raw).unwrap();
    assert!(response.is_success());
    assert_eq!(response.request_id(), "req_123");
    assert!(response.data().is_some());
}

#[test]
fn test_parse_control_response_error() {
    let raw = json!({
        "type": "control_response",
        "response": {
            "subtype": "error",
            "request_id": "req_456",
            "error": "Tool not found"
        }
    });

    let response = parse_control_response(raw).unwrap();
    assert!(!response.is_success());
    assert_eq!(response.error(), Some("Tool not found"));
}
