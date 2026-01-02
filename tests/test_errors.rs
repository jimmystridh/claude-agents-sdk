//! Tests for error handling.

use claude_agents_sdk::*;

#[test]
fn test_cli_not_found_error() {
    let err = ClaudeSDKError::cli_not_found("Claude not in PATH");
    assert!(err.is_cli_not_found());
    assert!(!err.is_recoverable());
    assert!(err.to_string().contains("Claude not in PATH"));
}

#[test]
fn test_cli_connection_error() {
    let err = ClaudeSDKError::cli_connection("Connection refused");
    assert!(err.is_recoverable());
    assert!(err.to_string().contains("Connection refused"));
}

#[test]
fn test_process_error() {
    let err = ClaudeSDKError::process(Some(1), "Command failed");
    assert!(!err.is_recoverable());
    assert!(err.to_string().contains("exit code"));
    assert!(err.to_string().contains("1"));
}

#[test]
fn test_process_error_with_stderr() {
    let err = ClaudeSDKError::process_with_stderr(
        Some(2),
        "Command failed",
        "Error: invalid argument",
    );
    match &err {
        ClaudeSDKError::Process { stderr, .. } => {
            assert_eq!(stderr.as_deref(), Some("Error: invalid argument"));
        }
        _ => panic!("Expected Process error"),
    }
}

#[test]
fn test_json_decode_error() {
    let err = ClaudeSDKError::json_decode("Invalid JSON");
    assert!(!err.is_recoverable());
    assert!(err.to_string().contains("Invalid JSON"));
}

#[test]
fn test_message_parse_error() {
    let err = ClaudeSDKError::message_parse("Unknown message type");
    assert!(err.to_string().contains("Unknown message type"));
}

#[test]
fn test_message_parse_error_with_raw() {
    let raw = serde_json::json!({"type": "unknown"});
    let err = ClaudeSDKError::message_parse_with_raw("Unknown type", raw.clone());
    match &err {
        ClaudeSDKError::MessageParse { raw_message, .. } => {
            assert_eq!(raw_message.as_ref().unwrap()["type"], "unknown");
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_configuration_error() {
    let err = ClaudeSDKError::configuration("Invalid configuration");
    assert!(err.to_string().contains("Invalid configuration"));
}

#[test]
fn test_control_protocol_error() {
    let err = ClaudeSDKError::control_protocol("Protocol error");
    assert!(err.to_string().contains("Protocol error"));
}

#[test]
fn test_control_protocol_error_with_id() {
    let err = ClaudeSDKError::control_protocol_with_id("Timeout", "req_123");
    match &err {
        ClaudeSDKError::ControlProtocol { request_id, .. } => {
            assert_eq!(request_id.as_deref(), Some("req_123"));
        }
        _ => panic!("Expected ControlProtocol error"),
    }
}

#[test]
fn test_timeout_error() {
    let err = ClaudeSDKError::timeout(5000);
    assert!(err.is_recoverable());
    assert!(err.to_string().contains("5000ms"));
}

#[test]
fn test_version_mismatch_error() {
    let err = ClaudeSDKError::version_mismatch("1.0.0", "2.0.0");
    assert!(err.to_string().contains("1.0.0"));
    assert!(err.to_string().contains("2.0.0"));
}

#[test]
fn test_channel_error() {
    let err = ClaudeSDKError::channel("Channel closed");
    assert!(err.is_recoverable());
}

#[test]
fn test_internal_error() {
    let err = ClaudeSDKError::internal("Unexpected state");
    assert!(!err.is_recoverable());
}

#[test]
fn test_error_display() {
    let errors = vec![
        ClaudeSDKError::cli_not_found("not found"),
        ClaudeSDKError::cli_connection("failed"),
        ClaudeSDKError::process(Some(1), "exit"),
        ClaudeSDKError::json_decode("parse"),
        ClaudeSDKError::message_parse("type"),
        ClaudeSDKError::configuration("config"),
        ClaudeSDKError::control_protocol("control"),
        ClaudeSDKError::Interrupted,
        ClaudeSDKError::timeout(1000),
        ClaudeSDKError::version_mismatch("1", "2"),
        ClaudeSDKError::channel("chan"),
        ClaudeSDKError::internal("bug"),
    ];

    for err in errors {
        // Just verify Display doesn't panic
        let _ = err.to_string();
    }
}

#[test]
fn test_io_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let sdk_err: ClaudeSDKError = io_err.into();
    assert!(matches!(sdk_err, ClaudeSDKError::Io(_)));
}
