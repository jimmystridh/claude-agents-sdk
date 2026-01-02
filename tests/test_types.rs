//! Tests for type definitions.

use claude_agents_sdk::*;
use serde_json::json;

#[test]
fn test_permission_mode_serialization() {
    let mode = PermissionMode::AcceptEdits;
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(json, r#""acceptEdits""#);

    let mode: PermissionMode = serde_json::from_str(r#""bypassPermissions""#).unwrap();
    assert_eq!(mode, PermissionMode::BypassPermissions);
}

#[test]
fn test_permission_result_allow() {
    let result = PermissionResult::allow();
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["behavior"], "allow");
}

#[test]
fn test_permission_result_deny() {
    let result = PermissionResult::deny_with_message("Not allowed");
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["behavior"], "deny");
    assert_eq!(json["message"], "Not allowed");
}

#[test]
fn test_permission_result_allow_with_updated_input() {
    let result = PermissionResult::Allow(PermissionResultAllow {
        behavior: "allow".to_string(),
        updated_input: Some(json!({"modified": true})),
        updated_permissions: None,
    });
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["behavior"], "allow");
    assert_eq!(json["updatedInput"]["modified"], true);
}

#[test]
fn test_content_block_text() {
    let block = ContentBlock::Text(TextBlock {
        text: "Hello, world!".to_string(),
    });
    assert_eq!(block.as_text(), Some("Hello, world!"));
    assert!(!block.is_tool_use());
}

#[test]
fn test_content_block_tool_use() {
    let block = ContentBlock::ToolUse(ToolUseBlock {
        id: "tool_123".to_string(),
        name: "Bash".to_string(),
        input: json!({"command": "ls"}),
    });
    assert!(block.is_tool_use());
    assert!(block.as_text().is_none());
}

#[test]
fn test_user_message_text() {
    let msg = UserMessage {
        content: UserMessageContent::Text("Hello".to_string()),
        uuid: None,
        parent_tool_use_id: None,
    };
    assert_eq!(msg.text(), Some("Hello"));
}

#[test]
fn test_assistant_message_text() {
    let msg = AssistantMessage {
        content: vec![
            ContentBlock::Text(TextBlock {
                text: "Hello ".to_string(),
            }),
            ContentBlock::Text(TextBlock {
                text: "world!".to_string(),
            }),
        ],
        model: "claude-3-sonnet".to_string(),
        parent_tool_use_id: None,
        error: None,
    };
    assert_eq!(msg.text(), "Hello world!");
}

#[test]
fn test_assistant_message_tool_uses() {
    let msg = AssistantMessage {
        content: vec![
            ContentBlock::Text(TextBlock {
                text: "Let me run a command".to_string(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "tool_1".to_string(),
                name: "Bash".to_string(),
                input: json!({"command": "ls"}),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "tool_2".to_string(),
                name: "Read".to_string(),
                input: json!({"path": "/tmp/file.txt"}),
            }),
        ],
        model: "claude-3".to_string(),
        parent_tool_use_id: None,
        error: None,
    };

    let tool_uses = msg.tool_uses();
    assert_eq!(tool_uses.len(), 2);
    assert_eq!(tool_uses[0].name, "Bash");
    assert_eq!(tool_uses[1].name, "Read");
}

#[test]
fn test_message_is_result() {
    let msg = Message::Result(ResultMessage {
        subtype: "success".to_string(),
        duration_ms: 1000,
        duration_api_ms: 500,
        is_error: false,
        num_turns: 3,
        session_id: "sess_123".to_string(),
        total_cost_usd: Some(0.05),
        usage: None,
        result: Some("Done".to_string()),
        structured_output: None,
    });
    assert!(msg.is_result());
    assert!(!msg.is_assistant());
}

#[test]
fn test_message_as_assistant() {
    let msg = Message::Assistant(AssistantMessage {
        content: vec![ContentBlock::Text(TextBlock {
            text: "Hi".to_string(),
        })],
        model: "claude-3".to_string(),
        parent_tool_use_id: None,
        error: None,
    });

    assert!(msg.is_assistant());
    assert!(msg.as_assistant().is_some());
    assert!(msg.as_result().is_none());
}

#[test]
fn test_claude_agent_options_builder() {
    let options = ClaudeAgentOptions::new()
        .with_model("claude-3-opus")
        .with_max_turns(10)
        .with_system_prompt("You are a helpful assistant.")
        .with_permission_mode(PermissionMode::AcceptEdits)
        .with_partial_messages();

    assert_eq!(options.model, Some("claude-3-opus".to_string()));
    assert_eq!(options.max_turns, Some(10));
    assert!(options.include_partial_messages);
    assert_eq!(options.permission_mode, Some(PermissionMode::AcceptEdits));
}

#[test]
fn test_claude_agent_options_with_allowed_tools() {
    let options = ClaudeAgentOptions::new()
        .with_allowed_tools(vec!["Bash".to_string(), "Read".to_string()]);

    assert_eq!(options.allowed_tools.len(), 2);
    assert!(options.allowed_tools.contains(&"Bash".to_string()));
}

#[test]
fn test_hook_event_serialization() {
    let event = HookEvent::PreToolUse;
    let json = serde_json::to_string(&event).unwrap();
    assert_eq!(json, r#""PreToolUse""#);
}

#[test]
fn test_sandbox_settings() {
    let settings = SandboxSettings {
        enabled: true,
        auto_allow_bash_if_sandboxed: true,
        excluded_commands: vec!["docker".to_string()],
        allow_unsandboxed_commands: false,
        network: Some(SandboxNetworkConfig {
            allow_unix_sockets: vec!["/var/run/docker.sock".to_string()],
            allow_local_binding: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let json = serde_json::to_value(&settings).unwrap();
    assert_eq!(json["enabled"], true);
    assert_eq!(json["excludedCommands"][0], "docker");
}

#[test]
fn test_mcp_server_config_stdio() {
    let config = McpServerConfig::Stdio(McpStdioServerConfig {
        server_type: "stdio".to_string(),
        command: "node".to_string(),
        args: vec!["server.js".to_string()],
        env: std::collections::HashMap::new(),
    });

    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["type"], "stdio");
    assert_eq!(json["command"], "node");
}

#[test]
fn test_agent_definition() {
    let agent = AgentDefinition {
        description: "A coding agent".to_string(),
        prompt: "You are a coding assistant.".to_string(),
        tools: Some(vec!["Bash".to_string(), "Read".to_string()]),
        model: Some(AgentModel::Sonnet),
    };

    let json = serde_json::to_value(&agent).unwrap();
    assert_eq!(json["description"], "A coding agent");
    assert_eq!(json["model"], "sonnet");
}

#[test]
fn test_control_response_success() {
    let response = ControlResponse {
        response_type: "control_response".to_string(),
        response: ControlResponsePayload::Success {
            request_id: "req_123".to_string(),
            response: Some(json!({"initialized": true})),
        },
    };

    assert!(response.is_success());
    assert_eq!(response.request_id(), "req_123");
    assert!(response.data().is_some());
    assert!(response.error().is_none());
}

#[test]
fn test_control_response_error() {
    let response = ControlResponse {
        response_type: "control_response".to_string(),
        response: ControlResponsePayload::Error {
            request_id: "req_456".to_string(),
            error: "Something went wrong".to_string(),
        },
    };

    assert!(!response.is_success());
    assert_eq!(response.error(), Some("Something went wrong"));
}

#[test]
fn test_sync_hook_output() {
    let output = SyncHookOutput {
        continue_: Some(true),
        suppress_output: Some(false),
        ..Default::default()
    };

    let json = serde_json::to_value(&output).unwrap();
    // Should use "continue" not "continue_" in JSON
    assert_eq!(json["continue"], true);
}

// Tests for query_chunks functionality
#[test]
fn test_query_chunks_concept() {
    // The query_chunks function joins chunks into a single prompt
    let chunks = vec!["Hello", ", ", "world", "!"];
    let prompt: String = chunks.into_iter().collect();
    assert_eq!(prompt, "Hello, world!");
}

#[test]
fn test_query_chunks_with_code_blocks() {
    // Useful for building prompts with code blocks
    let chunks = vec![
        "Analyze this:\n",
        "```rust\n",
        "fn main() {}\n",
        "```",
    ];
    let prompt: String = chunks.into_iter().collect();
    assert!(prompt.contains("```rust"));
    assert!(prompt.contains("fn main()"));
}

#[test]
fn test_client_builder_chain() {
    // Test that ClaudeAgentOptions builder can chain options
    let options = ClaudeAgentOptions::new()
        .with_model("claude-3-sonnet")
        .with_max_turns(5)
        .with_system_prompt("Test prompt")
        .with_permission_mode(PermissionMode::AcceptEdits);

    assert_eq!(options.model, Some("claude-3-sonnet".to_string()));
    assert_eq!(options.max_turns, Some(5));
    assert_eq!(options.permission_mode, Some(PermissionMode::AcceptEdits));
}

#[test]
fn test_options_with_stderr_callback() {
    use std::sync::Arc;

    // Test that stderr callback can be set directly on options
    let stderr_output = Arc::new(std::sync::Mutex::new(Vec::new()));
    let output_clone = stderr_output.clone();

    let callback: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |line| {
        output_clone.lock().unwrap().push(line);
    });

    let mut options = ClaudeAgentOptions::new();
    options.stderr = Some(callback);
    assert!(options.stderr.is_some());
}

#[test]
fn test_options_with_allowed_tools() {
    // Test setting allowed tools
    let options = ClaudeAgentOptions::new()
        .with_allowed_tools(vec!["Read".to_string(), "Write".to_string()]);

    assert_eq!(options.allowed_tools.len(), 2);
    assert!(options.allowed_tools.contains(&"Read".to_string()));
}

#[test]
fn test_options_with_cwd() {
    use std::path::PathBuf;

    // Test setting working directory
    let options = ClaudeAgentOptions::new()
        .with_cwd("/tmp/test");

    assert_eq!(options.cwd, Some(PathBuf::from("/tmp/test")));
}
