//! Tests for Claude SDK client functionality.

use claude_agents_sdk::{
    AssistantMessage, ClaudeAgentOptions, ContentBlock, Message, PermissionMode, SystemPromptConfig,
    TextBlock, ToolsConfig,
};
use std::path::PathBuf;

#[test]
fn test_options_default() {
    let options = ClaudeAgentOptions::new();

    assert!(options.tools.is_none());
    assert!(options.allowed_tools.is_empty());
    assert!(options.system_prompt.is_none());
    assert!(options.permission_mode.is_none());
    assert!(!options.continue_conversation);
    assert!(options.resume.is_none());
    assert!(options.max_turns.is_none());
    assert!(options.max_budget_usd.is_none());
    assert!(options.disallowed_tools.is_empty());
    assert!(options.model.is_none());
    assert!(options.can_use_tool.is_none());
    assert!(options.hooks.is_none());
    assert!(!options.include_partial_messages);
}

#[test]
fn test_options_builder_chain() {
    let options = ClaudeAgentOptions::new()
        .with_model("claude-sonnet-4-5")
        .with_max_turns(10)
        .with_permission_mode(PermissionMode::AcceptEdits)
        .with_system_prompt("Be helpful")
        .with_cwd("/test/path")
        .with_allowed_tools(vec!["Read".to_string(), "Write".to_string()]);

    assert_eq!(options.model, Some("claude-sonnet-4-5".to_string()));
    assert_eq!(options.max_turns, Some(10));
    assert_eq!(options.permission_mode, Some(PermissionMode::AcceptEdits));
    assert_eq!(options.cwd, Some(PathBuf::from("/test/path")));
    assert_eq!(options.allowed_tools, vec!["Read", "Write"]);

    match options.system_prompt {
        Some(SystemPromptConfig::Text(text)) => assert_eq!(text, "Be helpful"),
        _ => panic!("Expected text system prompt"),
    }
}

#[test]
fn test_options_with_tools_list() {
    let mut options = ClaudeAgentOptions::new();
    options.tools = Some(ToolsConfig::List(vec![
        "Read".to_string(),
        "Write".to_string(),
        "Bash".to_string(),
    ]));

    match options.tools {
        Some(ToolsConfig::List(tools)) => {
            assert_eq!(tools.len(), 3);
            assert!(tools.contains(&"Read".to_string()));
            assert!(tools.contains(&"Write".to_string()));
            assert!(tools.contains(&"Bash".to_string()));
        }
        _ => panic!("Expected tools list"),
    }
}

#[test]
fn test_options_with_empty_tools_list() {
    let mut options = ClaudeAgentOptions::new();
    options.tools = Some(ToolsConfig::List(vec![]));

    match options.tools {
        Some(ToolsConfig::List(tools)) => {
            assert!(tools.is_empty());
        }
        _ => panic!("Expected empty tools list"),
    }
}

#[test]
fn test_options_with_tools_preset() {
    use claude_agents_sdk::ToolsPreset;

    let mut options = ClaudeAgentOptions::new();
    options.tools = Some(ToolsConfig::Preset(ToolsPreset {
        preset_type: "preset".to_string(),
        preset: "claude_code".to_string(),
    }));

    match options.tools {
        Some(ToolsConfig::Preset(preset)) => {
            assert_eq!(preset.preset, "claude_code");
        }
        _ => panic!("Expected tools preset"),
    }
}

#[test]
fn test_options_with_disallowed_tools() {
    let mut options = ClaudeAgentOptions::new();
    options.disallowed_tools = vec!["Bash".to_string(), "Write".to_string()];

    assert_eq!(options.disallowed_tools.len(), 2);
    assert!(options.disallowed_tools.contains(&"Bash".to_string()));
    assert!(options.disallowed_tools.contains(&"Write".to_string()));
}

#[test]
fn test_options_with_model() {
    let options = ClaudeAgentOptions::new().with_model("claude-opus-4-5");

    assert_eq!(options.model, Some("claude-opus-4-5".to_string()));
}

#[test]
fn test_options_with_fallback_model() {
    let mut options = ClaudeAgentOptions::new();
    options.model = Some("opus".to_string());
    options.fallback_model = Some("sonnet".to_string());

    assert_eq!(options.model, Some("opus".to_string()));
    assert_eq!(options.fallback_model, Some("sonnet".to_string()));
}

#[test]
fn test_options_with_max_thinking_tokens() {
    let mut options = ClaudeAgentOptions::new();
    options.max_thinking_tokens = Some(5000);

    assert_eq!(options.max_thinking_tokens, Some(5000));
}

#[test]
fn test_options_with_add_dirs() {
    let mut options = ClaudeAgentOptions::new();
    options.add_dirs = vec![
        PathBuf::from("/path/to/dir1"),
        PathBuf::from("/path/to/dir2"),
    ];

    assert_eq!(options.add_dirs.len(), 2);
    assert!(options.add_dirs.contains(&PathBuf::from("/path/to/dir1")));
    assert!(options.add_dirs.contains(&PathBuf::from("/path/to/dir2")));
}

#[test]
fn test_options_with_session_continuation() {
    let mut options = ClaudeAgentOptions::new();
    options.continue_conversation = true;
    options.resume = Some("session-123".to_string());

    assert!(options.continue_conversation);
    assert_eq!(options.resume, Some("session-123".to_string()));
}

#[test]
fn test_options_with_fork_session() {
    let mut options = ClaudeAgentOptions::new();
    options.resume = Some("session-123".to_string());
    options.fork_session = true;

    assert!(options.fork_session);
    assert_eq!(options.resume, Some("session-123".to_string()));
}

#[test]
fn test_options_with_env_vars() {
    use std::collections::HashMap;

    let mut options = ClaudeAgentOptions::new();
    let mut env = HashMap::new();
    env.insert("MY_VAR".to_string(), "my_value".to_string());
    env.insert("ANOTHER_VAR".to_string(), "another_value".to_string());
    options.env = env;

    assert_eq!(options.env.len(), 2);
    assert_eq!(options.env.get("MY_VAR"), Some(&"my_value".to_string()));
}

#[test]
fn test_options_with_extra_args() {
    use std::collections::HashMap;

    let mut options = ClaudeAgentOptions::new();
    let mut extra_args = HashMap::new();
    extra_args.insert("new-flag".to_string(), Some("value".to_string()));
    extra_args.insert("boolean-flag".to_string(), None);
    options.extra_args = extra_args;

    assert_eq!(options.extra_args.len(), 2);
    assert_eq!(
        options.extra_args.get("new-flag"),
        Some(&Some("value".to_string()))
    );
    assert_eq!(options.extra_args.get("boolean-flag"), Some(&None));
}

#[test]
fn test_options_with_settings() {
    let mut options = ClaudeAgentOptions::new();
    options.settings = Some(r#"{"permissions": {"allow": ["Bash(ls:*)"]}}"#.to_string());

    assert!(options.settings.is_some());
    let settings = options.settings.unwrap();
    assert!(settings.contains("permissions"));
}

#[test]
fn test_options_with_partial_messages() {
    let options = ClaudeAgentOptions::new().with_partial_messages();

    assert!(options.include_partial_messages);
}

#[test]
fn test_message_discriminant() {
    let assistant = Message::Assistant(AssistantMessage {
        content: vec![ContentBlock::Text(TextBlock {
            text: "Hello".to_string(),
        })],
        model: "claude-3".to_string(),
        parent_tool_use_id: None,
        error: None,
    });

    assert!(assistant.is_assistant());
    assert!(!assistant.is_result());

    let result = Message::Result(claude_agents_sdk::ResultMessage {
        subtype: "success".to_string(),
        duration_ms: 100,
        duration_api_ms: 80,
        is_error: false,
        num_turns: 1,
        session_id: "test".to_string(),
        total_cost_usd: None,
        usage: None,
        result: None,
        structured_output: None,
    });

    assert!(result.is_result());
    assert!(!result.is_assistant());
}

#[test]
fn test_message_as_assistant() {
    let msg = Message::Assistant(AssistantMessage {
        content: vec![ContentBlock::Text(TextBlock {
            text: "Hello".to_string(),
        })],
        model: "claude-3".to_string(),
        parent_tool_use_id: None,
        error: None,
    });

    let asst = msg.as_assistant();
    assert!(asst.is_some());
    assert_eq!(asst.unwrap().text(), "Hello");
}

#[test]
fn test_message_as_result() {
    let msg = Message::Result(claude_agents_sdk::ResultMessage {
        subtype: "success".to_string(),
        duration_ms: 100,
        duration_api_ms: 80,
        is_error: false,
        num_turns: 1,
        session_id: "test".to_string(),
        total_cost_usd: Some(0.001),
        usage: None,
        result: None,
        structured_output: None,
    });

    let result = msg.as_result();
    assert!(result.is_some());
    assert_eq!(result.unwrap().total_cost_usd, Some(0.001));
}
