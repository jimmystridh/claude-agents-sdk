//! Tests for Claude SDK transport layer.

use claude_agents_sdk::{ClaudeAgentOptions, PermissionMode, SettingSource, SystemPromptConfig};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_options_for_basic_command() {
    let options = ClaudeAgentOptions::new();

    // Verify default state
    assert!(options.model.is_none());
    assert!(options.system_prompt.is_none());
    assert!(options.permission_mode.is_none());
    assert!(options.max_turns.is_none());
}

#[test]
fn test_options_with_system_prompt_string() {
    let options = ClaudeAgentOptions::new().with_system_prompt("Be helpful");

    match options.system_prompt {
        Some(SystemPromptConfig::Text(text)) => {
            assert_eq!(text, "Be helpful");
        }
        _ => panic!("Expected text system prompt"),
    }
}

#[test]
fn test_options_with_system_prompt_preset() {
    use claude_agents_sdk::SystemPromptPreset;

    let mut options = ClaudeAgentOptions::new();
    options.system_prompt = Some(SystemPromptConfig::Preset(SystemPromptPreset {
        preset_type: "preset".to_string(),
        preset: "claude_code".to_string(),
        append: None,
    }));

    match options.system_prompt {
        Some(SystemPromptConfig::Preset(preset)) => {
            assert_eq!(preset.preset, "claude_code");
            assert!(preset.append.is_none());
        }
        _ => panic!("Expected preset system prompt"),
    }
}

#[test]
fn test_options_with_system_prompt_preset_and_append() {
    use claude_agents_sdk::SystemPromptPreset;

    let mut options = ClaudeAgentOptions::new();
    options.system_prompt = Some(SystemPromptConfig::Preset(SystemPromptPreset {
        preset_type: "preset".to_string(),
        preset: "claude_code".to_string(),
        append: Some("Be concise.".to_string()),
    }));

    match options.system_prompt {
        Some(SystemPromptConfig::Preset(preset)) => {
            assert_eq!(preset.preset, "claude_code");
            assert_eq!(preset.append, Some("Be concise.".to_string()));
        }
        _ => panic!("Expected preset system prompt"),
    }
}

#[test]
fn test_options_with_model_and_permission_mode() {
    let options = ClaudeAgentOptions::new()
        .with_model("claude-sonnet-4-5")
        .with_permission_mode(PermissionMode::AcceptEdits)
        .with_max_turns(5);

    assert_eq!(options.model, Some("claude-sonnet-4-5".to_string()));
    assert_eq!(options.permission_mode, Some(PermissionMode::AcceptEdits));
    assert_eq!(options.max_turns, Some(5));
}

#[test]
fn test_options_with_allowed_and_disallowed_tools() {
    let mut options = ClaudeAgentOptions::new();
    options.allowed_tools = vec!["Read".to_string(), "Write".to_string()];
    options.disallowed_tools = vec!["Bash".to_string()];

    assert_eq!(options.allowed_tools, vec!["Read", "Write"]);
    assert_eq!(options.disallowed_tools, vec!["Bash"]);
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
fn test_options_with_settings_string() {
    let mut options = ClaudeAgentOptions::new();
    options.settings = Some(r#"{"permissions": {"allow": ["Bash(ls:*)"]}}"#.to_string());

    assert!(options.settings.is_some());
}

#[test]
fn test_options_with_extra_args() {
    let mut options = ClaudeAgentOptions::new();
    let mut extra_args = HashMap::new();
    extra_args.insert("new-flag".to_string(), Some("value".to_string()));
    extra_args.insert("boolean-flag".to_string(), None);
    extra_args.insert("another-option".to_string(), Some("test-value".to_string()));
    options.extra_args = extra_args;

    assert_eq!(options.extra_args.len(), 3);
    assert_eq!(
        options.extra_args.get("new-flag"),
        Some(&Some("value".to_string()))
    );
    assert_eq!(options.extra_args.get("boolean-flag"), Some(&None));
}

#[test]
fn test_options_with_mcp_servers_map() {
    use claude_agents_sdk::{McpServerConfig, McpServersConfig, McpStdioServerConfig};

    let mut mcp_servers = HashMap::new();
    mcp_servers.insert(
        "test-server".to_string(),
        McpServerConfig::Stdio(McpStdioServerConfig {
            server_type: "stdio".to_string(),
            command: "/path/to/server".to_string(),
            args: vec!["--option".to_string(), "value".to_string()],
            env: HashMap::new(),
        }),
    );

    let mut options = ClaudeAgentOptions::new();
    options.mcp_servers = McpServersConfig::Map(mcp_servers);

    match options.mcp_servers {
        McpServersConfig::Map(servers) => {
            assert!(servers.contains_key("test-server"));
        }
        _ => panic!("Expected map config"),
    }
}

#[test]
fn test_options_with_mcp_servers_path() {
    use claude_agents_sdk::McpServersConfig;

    let mut options = ClaudeAgentOptions::new();
    options.mcp_servers = McpServersConfig::Path(PathBuf::from("/path/to/mcp-config.json"));

    match options.mcp_servers {
        McpServersConfig::Path(path) => {
            assert_eq!(path, PathBuf::from("/path/to/mcp-config.json"));
        }
        _ => panic!("Expected path config"),
    }
}

#[test]
fn test_options_with_env_vars() {
    let mut options = ClaudeAgentOptions::new();
    let mut env = HashMap::new();
    env.insert("MY_TEST_VAR".to_string(), "test-value".to_string());
    env.insert("ANOTHER_VAR".to_string(), "another-value".to_string());
    options.env = env;

    assert_eq!(options.env.len(), 2);
    assert_eq!(options.env.get("MY_TEST_VAR"), Some(&"test-value".to_string()));
}

#[test]
fn test_options_with_user() {
    let mut options = ClaudeAgentOptions::new();
    options.user = Some("claude".to_string());

    assert_eq!(options.user, Some("claude".to_string()));
}

#[test]
fn test_options_with_sandbox() {
    use claude_agents_sdk::{SandboxNetworkConfig, SandboxSettings};

    let mut options = ClaudeAgentOptions::new();
    options.sandbox = Some(SandboxSettings {
        enabled: true,
        auto_allow_bash_if_sandboxed: true,
        excluded_commands: vec![],
        allow_unsandboxed_commands: true,
        network: Some(SandboxNetworkConfig {
            allow_unix_sockets: vec!["/var/run/docker.sock".to_string()],
            allow_all_unix_sockets: false,
            allow_local_binding: true,
            http_proxy_port: None,
            socks_proxy_port: None,
        }),
        ignore_violations: None,
        enable_weaker_nested_sandbox: false,
    });

    assert!(options.sandbox.is_some());
    let sandbox = options.sandbox.unwrap();
    assert!(sandbox.enabled);
    assert!(sandbox.auto_allow_bash_if_sandboxed);
    assert!(sandbox.network.is_some());
}

#[test]
fn test_options_with_tools_array() {
    use claude_agents_sdk::ToolsConfig;

    let mut options = ClaudeAgentOptions::new();
    options.tools = Some(ToolsConfig::List(vec![
        "Read".to_string(),
        "Edit".to_string(),
        "Bash".to_string(),
    ]));

    match options.tools {
        Some(ToolsConfig::List(tools)) => {
            assert_eq!(tools.len(), 3);
            assert!(tools.contains(&"Read".to_string()));
        }
        _ => panic!("Expected tools list"),
    }
}

#[test]
fn test_options_with_tools_empty_array() {
    use claude_agents_sdk::ToolsConfig;

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
    use claude_agents_sdk::{ToolsConfig, ToolsPreset};

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
fn test_options_with_setting_sources() {
    let mut options = ClaudeAgentOptions::new();
    options.setting_sources = Some(vec![SettingSource::User, SettingSource::Project]);

    assert!(options.setting_sources.is_some());
    let sources = options.setting_sources.unwrap();
    assert_eq!(sources.len(), 2);
    assert!(sources.contains(&SettingSource::User));
    assert!(sources.contains(&SettingSource::Project));
}

#[test]
fn test_options_with_agents() {
    use claude_agents_sdk::AgentDefinition;

    let mut agents = HashMap::new();
    agents.insert(
        "test-agent".to_string(),
        AgentDefinition {
            description: "A test agent".to_string(),
            prompt: "You are a test agent".to_string(),
            tools: Some(vec!["Read".to_string()]),
            model: None,
        },
    );

    let mut options = ClaudeAgentOptions::new();
    options.agents = Some(agents);

    assert!(options.agents.is_some());
    let agents = options.agents.unwrap();
    assert!(agents.contains_key("test-agent"));
}

#[test]
fn test_permission_mode_serialization() {
    let mode = PermissionMode::AcceptEdits;
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(json, "\"acceptEdits\"");

    let mode = PermissionMode::BypassPermissions;
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(json, "\"bypassPermissions\"");

    let mode = PermissionMode::Plan;
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(json, "\"plan\"");

    let mode = PermissionMode::Default;
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(json, "\"default\"");
}

#[test]
fn test_setting_source_serialization() {
    let source = SettingSource::User;
    let json = serde_json::to_string(&source).unwrap();
    assert_eq!(json, "\"user\"");

    let source = SettingSource::Project;
    let json = serde_json::to_string(&source).unwrap();
    assert_eq!(json, "\"project\"");

    let source = SettingSource::Local;
    let json = serde_json::to_string(&source).unwrap();
    assert_eq!(json, "\"local\"");
}

#[test]
fn test_cli_path_option() {
    let mut options = ClaudeAgentOptions::new();
    options.cli_path = Some(PathBuf::from("/usr/local/bin/claude"));

    assert_eq!(
        options.cli_path,
        Some(PathBuf::from("/usr/local/bin/claude"))
    );
}

#[test]
fn test_cwd_option() {
    let options = ClaudeAgentOptions::new().with_cwd("/custom/path");

    assert_eq!(options.cwd, Some(PathBuf::from("/custom/path")));
}

#[test]
fn test_max_buffer_size_option() {
    let mut options = ClaudeAgentOptions::new();
    options.max_buffer_size = Some(1024 * 1024); // 1MB

    assert_eq!(options.max_buffer_size, Some(1024 * 1024));
}
