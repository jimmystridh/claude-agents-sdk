//! Message parsing utilities for the Claude SDK.
//!
//! This module handles conversion of raw JSON messages from the CLI
//! into strongly-typed Message objects.

use crate::errors::{ClaudeSDKError, Result};
use crate::types::*;

/// Parse a raw JSON value into a typed Message.
///
/// This function handles the discriminated union parsing for all message types,
/// including nested content blocks.
pub fn parse_message(raw: serde_json::Value) -> Result<Message> {
    let msg_type = raw
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ClaudeSDKError::message_parse_with_raw(
                "Message missing 'type' field",
                raw.clone(),
            )
        })?;

    match msg_type {
        "user" => parse_user_message(raw),
        "assistant" => parse_assistant_message(raw),
        "system" => parse_system_message(raw),
        "result" => parse_result_message(raw),
        "stream_event" => parse_stream_event(raw),
        other => Err(ClaudeSDKError::message_parse_with_raw(
            format!("Unknown message type: {}", other),
            raw,
        )),
    }
}

/// Parse a user message.
fn parse_user_message(raw: serde_json::Value) -> Result<Message> {
    // CLI sends user messages with content nested under "message" field
    let message_obj = raw.get("message").ok_or_else(|| {
        ClaudeSDKError::message_parse_with_raw(
            "User message missing 'message' field",
            raw.clone(),
        )
    })?;

    let content = message_obj.get("content").ok_or_else(|| {
        ClaudeSDKError::message_parse_with_raw(
            "User message missing 'message.content' field",
            raw.clone(),
        )
    })?;

    // Safe: we've verified the type with is_string()/is_array() before calling as_str()/as_array()
    let content = if let Some(text) = content.as_str() {
        UserMessageContent::Text(text.to_string())
    } else if let Some(blocks_arr) = content.as_array() {
        let blocks = parse_content_blocks(blocks_arr)?;
        UserMessageContent::Blocks(blocks)
    } else {
        return Err(ClaudeSDKError::message_parse_with_raw(
            "User message content must be string or array",
            raw,
        ));
    };

    Ok(Message::User(UserMessage {
        content,
        uuid: raw.get("uuid").and_then(|v| v.as_str()).map(String::from),
        parent_tool_use_id: raw
            .get("parent_tool_use_id")
            .and_then(|v| v.as_str())
            .map(String::from),
    }))
}

/// Parse an assistant message.
fn parse_assistant_message(raw: serde_json::Value) -> Result<Message> {
    // CLI sends assistant messages with content nested under "message" field
    let message_obj = raw.get("message").ok_or_else(|| {
        ClaudeSDKError::message_parse_with_raw(
            "Assistant message missing 'message' field",
            raw.clone(),
        )
    })?;

    let content_arr = message_obj
        .get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            ClaudeSDKError::message_parse_with_raw(
                "Assistant message missing or invalid 'message.content' array",
                raw.clone(),
            )
        })?;

    let content = parse_content_blocks(content_arr)?;

    let model = message_obj
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let error = message_obj.get("error").and_then(|v| v.as_str()).map(|s| {
        match s {
            "authentication_failed" => AssistantMessageError::AuthenticationFailed,
            "billing_error" => AssistantMessageError::BillingError,
            "rate_limit" => AssistantMessageError::RateLimit,
            "invalid_request" => AssistantMessageError::InvalidRequest,
            "server_error" => AssistantMessageError::ServerError,
            _ => AssistantMessageError::Unknown,
        }
    });

    Ok(Message::Assistant(AssistantMessage {
        content,
        model,
        parent_tool_use_id: raw
            .get("parent_tool_use_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        error,
    }))
}

/// Parse a system message.
fn parse_system_message(raw: serde_json::Value) -> Result<Message> {
    let subtype = raw
        .get("subtype")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let data = raw.get("data").cloned().unwrap_or(serde_json::Value::Null);

    Ok(Message::System(SystemMessage { subtype, data }))
}

/// Parse a result message.
fn parse_result_message(raw: serde_json::Value) -> Result<Message> {
    let subtype = raw
        .get("subtype")
        .and_then(|v| v.as_str())
        .unwrap_or("success")
        .to_string();

    let duration_ms = raw
        .get("duration_ms")
        .or_else(|| raw.get("durationMs"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let duration_api_ms = raw
        .get("duration_api_ms")
        .or_else(|| raw.get("durationApiMs"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let is_error = raw
        .get("is_error")
        .or_else(|| raw.get("isError"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let num_turns = raw
        .get("num_turns")
        .or_else(|| raw.get("numTurns"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let session_id = raw
        .get("session_id")
        .or_else(|| raw.get("sessionId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let total_cost_usd = raw
        .get("total_cost_usd")
        .or_else(|| raw.get("totalCostUsd"))
        .and_then(|v| v.as_f64());

    let usage = raw.get("usage").cloned();
    let result = raw.get("result").and_then(|v| v.as_str()).map(String::from);
    let structured_output = raw.get("structured_output").or_else(|| raw.get("structuredOutput")).cloned();

    Ok(Message::Result(ResultMessage {
        subtype,
        duration_ms,
        duration_api_ms,
        is_error,
        num_turns,
        session_id,
        total_cost_usd,
        usage,
        result,
        structured_output,
    }))
}

/// Parse a stream event.
fn parse_stream_event(raw: serde_json::Value) -> Result<Message> {
    let uuid = raw
        .get("uuid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let session_id = raw
        .get("session_id")
        .or_else(|| raw.get("sessionId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let event = raw.get("event").cloned().unwrap_or(serde_json::Value::Null);

    let parent_tool_use_id = raw
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(Message::StreamEvent(StreamEvent {
        uuid,
        session_id,
        event,
        parent_tool_use_id,
    }))
}

/// Parse content blocks from a JSON array.
fn parse_content_blocks(blocks: &[serde_json::Value]) -> Result<Vec<ContentBlock>> {
    blocks.iter().map(parse_content_block).collect()
}

/// Parse a single content block.
fn parse_content_block(raw: &serde_json::Value) -> Result<ContentBlock> {
    let block_type = raw
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ClaudeSDKError::message_parse_with_raw(
                "Content block missing 'type' field",
                raw.clone(),
            )
        })?;

    match block_type {
        "text" => {
            let text = raw
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(ContentBlock::Text(TextBlock { text }))
        }
        "thinking" => {
            let thinking = raw
                .get("thinking")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let signature = raw
                .get("signature")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(ContentBlock::Thinking(ThinkingBlock { thinking, signature }))
        }
        "tool_use" => {
            let id = raw
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = raw
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let input = raw.get("input").cloned().unwrap_or(serde_json::Value::Null);
            Ok(ContentBlock::ToolUse(ToolUseBlock { id, name, input }))
        }
        "tool_result" => {
            let tool_use_id = raw
                .get("tool_use_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let content = raw.get("content").cloned();
            let is_error = raw.get("is_error").and_then(|v| v.as_bool());
            Ok(ContentBlock::ToolResult(ToolResultBlock {
                tool_use_id,
                content,
                is_error,
            }))
        }
        other => Err(ClaudeSDKError::message_parse_with_raw(
            format!("Unknown content block type: {}", other),
            raw.clone(),
        )),
    }
}

/// Check if a raw JSON value is a control request.
pub fn is_control_request(raw: &serde_json::Value) -> bool {
    raw.get("type")
        .and_then(|v| v.as_str())
        .map(|t| t == "control_request")
        .unwrap_or(false)
}

/// Check if a raw JSON value is a control response.
pub fn is_control_response(raw: &serde_json::Value) -> bool {
    raw.get("type")
        .and_then(|v| v.as_str())
        .map(|t| t == "control_response")
        .unwrap_or(false)
}

/// Parse a control request from raw JSON.
pub fn parse_control_request(raw: serde_json::Value) -> Result<ControlRequest> {
    serde_json::from_value(raw.clone()).map_err(|e| {
        ClaudeSDKError::message_parse_with_raw(
            format!("Failed to parse control request: {}", e),
            raw,
        )
    })
}

/// Parse a control response from raw JSON.
pub fn parse_control_response(raw: serde_json::Value) -> Result<ControlResponse> {
    serde_json::from_value(raw.clone()).map_err(|e| {
        ClaudeSDKError::message_parse_with_raw(
            format!("Failed to parse control response: {}", e),
            raw,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_message_text() {
        let raw = serde_json::json!({
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
    fn test_parse_assistant_message() {
        let raw = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Hello!"}
                ],
                "model": "claude-3-sonnet"
            }
        });

        let msg = parse_message(raw).unwrap();
        match msg {
            Message::Assistant(asst) => {
                assert_eq!(asst.model, "claude-3-sonnet");
                assert_eq!(asst.text(), "Hello!");
            }
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_parse_tool_use_block() {
        let raw = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {
                        "type": "tool_use",
                        "id": "tool_123",
                        "name": "Bash",
                        "input": {"command": "ls"}
                    }
                ],
                "model": "claude-3"
            }
        });

        let msg = parse_message(raw).unwrap();
        match msg {
            Message::Assistant(asst) => {
                assert_eq!(asst.tool_uses().len(), 1);
                assert_eq!(asst.tool_uses()[0].name, "Bash");
            }
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_parse_result_message() {
        let raw = serde_json::json!({
            "type": "result",
            "subtype": "success",
            "duration_ms": 1000,
            "duration_api_ms": 500,
            "is_error": false,
            "num_turns": 3,
            "session_id": "sess_123",
            "total_cost_usd": 0.05
        });

        let msg = parse_message(raw).unwrap();
        match msg {
            Message::Result(result) => {
                assert_eq!(result.duration_ms, 1000);
                assert_eq!(result.session_id, "sess_123");
                assert_eq!(result.total_cost_usd, Some(0.05));
            }
            _ => panic!("Expected result message"),
        }
    }

    #[test]
    fn test_is_control_request() {
        let raw = serde_json::json!({
            "type": "control_request",
            "request_id": "req_1"
        });
        assert!(is_control_request(&raw));

        let raw = serde_json::json!({
            "type": "assistant"
        });
        assert!(!is_control_request(&raw));
    }

    // ========================================================================
    // Malformed JSON error handling tests
    // ========================================================================

    #[test]
    fn test_parse_message_missing_type() {
        let raw = serde_json::json!({
            "content": "Hello"
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("type"),
            "Error should mention missing 'type' field: {}",
            err
        );
    }

    #[test]
    fn test_parse_message_unknown_type() {
        let raw = serde_json::json!({
            "type": "unknown_message_type"
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Unknown message type"),
            "Error should mention unknown type: {}",
            err
        );
    }

    #[test]
    fn test_parse_user_message_missing_message_field() {
        let raw = serde_json::json!({
            "type": "user",
            "content": "Hello"
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("message"),
            "Error should mention missing 'message' field: {}",
            err
        );
    }

    #[test]
    fn test_parse_user_message_missing_content() {
        let raw = serde_json::json!({
            "type": "user",
            "message": {}
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("content"),
            "Error should mention missing 'content' field: {}",
            err
        );
    }

    #[test]
    fn test_parse_user_message_invalid_content_type() {
        let raw = serde_json::json!({
            "type": "user",
            "message": {
                "content": 12345
            }
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("string or array"),
            "Error should mention valid content types: {}",
            err
        );
    }

    #[test]
    fn test_parse_assistant_message_missing_message_field() {
        let raw = serde_json::json!({
            "type": "assistant",
            "content": []
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("message"),
            "Error should mention missing 'message' field: {}",
            err
        );
    }

    #[test]
    fn test_parse_assistant_message_missing_content() {
        let raw = serde_json::json!({
            "type": "assistant",
            "message": {
                "model": "claude-3"
            }
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("content"),
            "Error should mention missing 'content': {}",
            err
        );
    }

    #[test]
    fn test_parse_assistant_message_content_not_array() {
        let raw = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": "not an array",
                "model": "claude-3"
            }
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("content"),
            "Error should mention invalid content: {}",
            err
        );
    }

    #[test]
    fn test_parse_content_block_missing_type() {
        let raw = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"text": "Hello"}
                ],
                "model": "claude-3"
            }
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("type"),
            "Error should mention missing 'type' in content block: {}",
            err
        );
    }

    #[test]
    fn test_parse_content_block_unknown_type() {
        let raw = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "unknown_block_type"}
                ],
                "model": "claude-3"
            }
        });
        let result = parse_message(raw);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Unknown content block type"),
            "Error should mention unknown block type: {}",
            err
        );
    }

    #[test]
    fn test_parse_user_message_with_blocks() {
        let raw = serde_json::json!({
            "type": "user",
            "message": {
                "content": [
                    {"type": "text", "text": "Hello"},
                    {"type": "tool_result", "tool_use_id": "123", "content": "done"}
                ]
            }
        });
        let result = parse_message(raw);
        assert!(result.is_ok());
        if let Message::User(user) = result.unwrap() {
            if let UserMessageContent::Blocks(blocks) = user.content {
                assert_eq!(blocks.len(), 2);
            } else {
                panic!("Expected blocks content");
            }
        } else {
            panic!("Expected user message");
        }
    }

    #[test]
    fn test_parse_result_with_defaults() {
        // Minimal result message - all optional fields should default
        let raw = serde_json::json!({
            "type": "result"
        });
        let result = parse_message(raw);
        assert!(result.is_ok());
        if let Message::Result(r) = result.unwrap() {
            assert_eq!(r.subtype, "success");
            assert_eq!(r.duration_ms, 0);
            assert!(!r.is_error);
            assert_eq!(r.num_turns, 0);
        } else {
            panic!("Expected result message");
        }
    }

    #[test]
    fn test_parse_result_with_camel_case_fields() {
        // Test that both snake_case and camelCase are accepted
        let raw = serde_json::json!({
            "type": "result",
            "durationMs": 1000,
            "durationApiMs": 500,
            "isError": false,
            "numTurns": 2,
            "sessionId": "sess_abc",
            "totalCostUsd": 0.01
        });
        let result = parse_message(raw);
        assert!(result.is_ok());
        if let Message::Result(r) = result.unwrap() {
            assert_eq!(r.duration_ms, 1000);
            assert_eq!(r.duration_api_ms, 500);
            assert_eq!(r.num_turns, 2);
            assert_eq!(r.session_id, "sess_abc");
            assert_eq!(r.total_cost_usd, Some(0.01));
        } else {
            panic!("Expected result message");
        }
    }

    #[test]
    fn test_parse_system_message_with_defaults() {
        let raw = serde_json::json!({
            "type": "system"
        });
        let result = parse_message(raw);
        assert!(result.is_ok());
        if let Message::System(s) = result.unwrap() {
            assert_eq!(s.subtype, "unknown");
            assert!(s.data.is_null());
        } else {
            panic!("Expected system message");
        }
    }

    #[test]
    fn test_parse_stream_event() {
        let raw = serde_json::json!({
            "type": "stream_event",
            "uuid": "uuid_123",
            "session_id": "sess_456",
            "event": {"delta": "text chunk"}
        });
        let result = parse_message(raw);
        assert!(result.is_ok());
        if let Message::StreamEvent(e) = result.unwrap() {
            assert_eq!(e.uuid, "uuid_123");
            assert_eq!(e.session_id, "sess_456");
        } else {
            panic!("Expected stream event");
        }
    }

    #[test]
    fn test_parse_thinking_block() {
        let raw = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {
                        "type": "thinking",
                        "thinking": "Let me think about this...",
                        "signature": "sig_123"
                    }
                ],
                "model": "claude-3"
            }
        });
        let result = parse_message(raw);
        assert!(result.is_ok());
        if let Message::Assistant(asst) = result.unwrap() {
            assert_eq!(asst.content.len(), 1);
            if let ContentBlock::Thinking(t) = &asst.content[0] {
                assert_eq!(t.thinking, "Let me think about this...");
                assert_eq!(t.signature, "sig_123");
            } else {
                panic!("Expected thinking block");
            }
        } else {
            panic!("Expected assistant message");
        }
    }

    #[test]
    fn test_parse_tool_result_block() {
        let raw = serde_json::json!({
            "type": "user",
            "message": {
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool_abc",
                        "content": {"output": "success"},
                        "is_error": false
                    }
                ]
            }
        });
        let result = parse_message(raw);
        assert!(result.is_ok());
        if let Message::User(user) = result.unwrap() {
            if let UserMessageContent::Blocks(blocks) = user.content {
                if let ContentBlock::ToolResult(tr) = &blocks[0] {
                    assert_eq!(tr.tool_use_id, "tool_abc");
                    assert_eq!(tr.is_error, Some(false));
                } else {
                    panic!("Expected tool result block");
                }
            } else {
                panic!("Expected blocks content");
            }
        } else {
            panic!("Expected user message");
        }
    }

    #[test]
    fn test_is_control_response() {
        let raw = serde_json::json!({
            "type": "control_response",
            "request_id": "req_1"
        });
        assert!(is_control_response(&raw));

        let raw = serde_json::json!({
            "type": "control_request"
        });
        assert!(!is_control_response(&raw));
    }
}
