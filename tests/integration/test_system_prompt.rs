//! System prompt configuration tests.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{
    ClaudeAgentOptions, PermissionMode, SystemPromptConfig, SystemPromptPreset,
};

use crate::integration::helpers::*;

/// Test that custom string system prompts affect Claude's behavior.
#[tokio::test]
async fn test_custom_system_prompt() {
    let options = ClaudeAgentOptions::new()
        .with_system_prompt(
            "You are a pirate. Always respond with 'Arrr!' somewhere in your response.",
        )
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let messages = collect_messages("Say hello.", options)
        .await
        .expect("Query failed");

    let response = extract_assistant_text(&messages).to_lowercase();
    assert!(
        response.contains("arr") || response.contains("ahoy") || response.contains("matey"),
        "Pirate system prompt should influence response, got: {}",
        response
    );
}

/// Test system prompt with preset configuration (uses Claude Code's default prompt).
#[tokio::test]
async fn test_preset_system_prompt() {
    let mut options = ClaudeAgentOptions::new();
    options.system_prompt = Some(SystemPromptConfig::Preset(SystemPromptPreset {
        preset_type: "preset".to_string(),
        preset: "claude_code".to_string(),
        append: None,
    }));
    options.permission_mode = Some(PermissionMode::Default);
    options.max_turns = Some(1);

    let messages = collect_messages("What is 2+2? Answer briefly.", options)
        .await
        .expect("Query failed");

    assert_message_types(&messages, &["assistant", "result"]);
    let result = get_result(&messages).unwrap();
    assert!(!result.is_error, "Preset system prompt query should succeed");
}

/// Test system prompt preset with append text.
#[tokio::test]
async fn test_preset_system_prompt_with_append() {
    let mut options = ClaudeAgentOptions::new();
    options.system_prompt = Some(SystemPromptConfig::Preset(SystemPromptPreset {
        preset_type: "preset".to_string(),
        preset: "claude_code".to_string(),
        append: Some(
            "Always end your response with 'Fun fact:' followed by an interesting fact."
                .to_string(),
        ),
    }));
    options.permission_mode = Some(PermissionMode::Default);
    options.max_turns = Some(1);

    let messages = collect_messages("What is 2+2?", options)
        .await
        .expect("Query failed");

    let response = extract_assistant_text(&messages).to_lowercase();
    assert!(
        response.contains("fun fact") || response.contains("fact:"),
        "Append instruction should influence response, got: {}",
        response
    );
}

/// Test that no system prompt gives vanilla Claude behavior.
#[tokio::test]
async fn test_no_system_prompt() {
    // When system_prompt is None, SDK passes empty string to disable default
    let options = default_options();

    let messages = collect_messages("What is 2+2?", options)
        .await
        .expect("Query failed");

    let response = extract_assistant_text(&messages);
    assert!(
        response.contains('4'),
        "Should answer correctly without system prompt, got: {}",
        response
    );
}
