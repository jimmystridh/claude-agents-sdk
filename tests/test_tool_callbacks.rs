//! Tests for tool permission callbacks and hook callbacks.

use claude_agents_sdk::{
    ClaudeAgentOptions, HookCallback, HookContext, HookEvent, HookInput, HookMatcher, HookOutput,
    PermissionResult, PermissionResultAllow, PermissionResultDeny, PermissionUpdate,
    PreToolUseHookInput, SyncHookOutput, ToolPermissionContext,
};
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn test_permission_result_allow() {
    let result = PermissionResult::allow();

    match result {
        PermissionResult::Allow(allow) => {
            assert_eq!(allow.behavior, "allow");
            assert!(allow.updated_input.is_none());
            assert!(allow.updated_permissions.is_none());
        }
        _ => panic!("Expected allow result"),
    }
}

#[test]
fn test_permission_result_deny() {
    let result = PermissionResult::deny();

    match result {
        PermissionResult::Deny(deny) => {
            assert_eq!(deny.behavior, "deny");
            assert!(deny.message.is_empty());
            assert!(!deny.interrupt);
        }
        _ => panic!("Expected deny result"),
    }
}

#[test]
fn test_permission_result_deny_with_message() {
    let result = PermissionResult::deny_with_message("Security policy violation");

    match result {
        PermissionResult::Deny(deny) => {
            assert_eq!(deny.behavior, "deny");
            assert_eq!(deny.message, "Security policy violation");
            assert!(!deny.interrupt);
        }
        _ => panic!("Expected deny result"),
    }
}

#[test]
fn test_permission_result_allow_with_updated_input() {
    let updated_input = serde_json::json!({"safe_mode": true, "param": "value"});
    let result =
        PermissionResult::Allow(PermissionResultAllow::with_updated_input(updated_input.clone()));

    match result {
        PermissionResult::Allow(allow) => {
            assert_eq!(allow.behavior, "allow");
            assert!(allow.updated_input.is_some());
            let input = allow.updated_input.unwrap();
            assert_eq!(input["safe_mode"], true);
            assert_eq!(input["param"], "value");
        }
        _ => panic!("Expected allow result"),
    }
}

#[test]
fn test_permission_result_deny_with_interrupt() {
    let result = PermissionResultDeny::with_interrupt("Critical security violation");

    assert_eq!(result.behavior, "deny");
    assert_eq!(result.message, "Critical security violation");
    assert!(result.interrupt);
}

#[test]
fn test_tool_permission_context() {
    let context = ToolPermissionContext {
        suggestions: vec![],
    };

    assert!(context.suggestions.is_empty());
}

#[test]
fn test_tool_permission_context_with_suggestions() {
    use claude_agents_sdk::PermissionUpdateType;

    let context = ToolPermissionContext {
        suggestions: vec![PermissionUpdate {
            update_type: PermissionUpdateType::AddRules,
            rules: None,
            behavior: None,
            mode: None,
            directories: None,
            destination: None,
        }],
    };

    assert_eq!(context.suggestions.len(), 1);
}

#[test]
fn test_hook_matcher_creation() {
    let callback: HookCallback = Arc::new(|_input, _tool_use_id, _context| {
        Box::pin(async move { HookOutput::Sync(SyncHookOutput::default()) })
    });

    let matcher = HookMatcher {
        matcher: Some("Bash".to_string()),
        hooks: vec![callback],
        timeout: Some(30.0),
    };

    assert_eq!(matcher.matcher, Some("Bash".to_string()));
    assert_eq!(matcher.hooks.len(), 1);
    assert_eq!(matcher.timeout, Some(30.0));
}

#[test]
fn test_hook_matcher_without_matcher() {
    let callback: HookCallback = Arc::new(|_input, _tool_use_id, _context| {
        Box::pin(async move { HookOutput::Sync(SyncHookOutput::default()) })
    });

    let matcher = HookMatcher {
        matcher: None, // Match all tools
        hooks: vec![callback],
        timeout: None,
    };

    assert!(matcher.matcher.is_none());
    assert_eq!(matcher.hooks.len(), 1);
    assert!(matcher.timeout.is_none());
}

#[test]
fn test_sync_hook_output_default() {
    let output = SyncHookOutput::default();

    assert!(output.continue_.is_none());
    assert!(output.suppress_output.is_none());
    assert!(output.stop_reason.is_none());
    assert!(output.decision.is_none());
    assert!(output.system_message.is_none());
    assert!(output.reason.is_none());
    assert!(output.hook_specific_output.is_none());
}

#[test]
fn test_sync_hook_output_with_block() {
    let output = SyncHookOutput {
        continue_: Some(false),
        suppress_output: None,
        stop_reason: None,
        decision: Some("block".to_string()),
        system_message: None,
        reason: Some("Security policy violation".to_string()),
        hook_specific_output: None,
    };

    assert_eq!(output.continue_, Some(false));
    assert_eq!(output.decision, Some("block".to_string()));
    assert_eq!(output.reason, Some("Security policy violation".to_string()));
}

#[test]
fn test_hook_output_sync() {
    let output = HookOutput::Sync(SyncHookOutput::default());

    match output {
        HookOutput::Sync(sync) => {
            assert!(sync.continue_.is_none());
        }
        _ => panic!("Expected sync output"),
    }
}

#[test]
fn test_hook_output_async() {
    use claude_agents_sdk::AsyncHookOutput;

    let output = HookOutput::Async(AsyncHookOutput {
        async_: true,
        async_timeout: Some(5000),
    });

    match output {
        HookOutput::Async(async_out) => {
            assert!(async_out.async_);
            assert_eq!(async_out.async_timeout, Some(5000));
        }
        _ => panic!("Expected async output"),
    }
}

#[test]
fn test_options_with_hooks() {
    let callback: HookCallback = Arc::new(|_input, _tool_use_id, _context| {
        Box::pin(async move { HookOutput::Sync(SyncHookOutput::default()) })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![HookMatcher {
            matcher: Some("Bash".to_string()),
            hooks: vec![callback],
            timeout: None,
        }],
    );

    let mut options = ClaudeAgentOptions::new();
    options.hooks = Some(hooks);

    assert!(options.hooks.is_some());
    let hooks = options.hooks.unwrap();
    assert!(hooks.contains_key(&HookEvent::PreToolUse));
    assert_eq!(hooks.get(&HookEvent::PreToolUse).unwrap().len(), 1);
}

#[test]
fn test_options_with_can_use_tool() {
    use std::future::Future;
    use std::pin::Pin;

    let callback: Arc<
        dyn Fn(String, serde_json::Value, ToolPermissionContext) -> Pin<Box<dyn Future<Output = PermissionResult> + Send>>
            + Send
            + Sync,
    > = Arc::new(|tool_name, _input, _context| {
        Box::pin(async move {
            if tool_name == "Bash" {
                PermissionResult::deny_with_message("Bash not allowed")
            } else {
                PermissionResult::allow()
            }
        })
    });

    let mut options = ClaudeAgentOptions::new();
    options.can_use_tool = Some(callback);

    assert!(options.can_use_tool.is_some());
}

#[test]
fn test_hook_event_variants() {
    // Test all hook event variants
    let events = vec![
        HookEvent::PreToolUse,
        HookEvent::PostToolUse,
        HookEvent::UserPromptSubmit,
        HookEvent::Stop,
        HookEvent::SubagentStop,
        HookEvent::PreCompact,
    ];

    // Verify they can be used as hash keys
    let mut map: HashMap<HookEvent, String> = HashMap::new();
    for event in events {
        map.insert(event, format!("{:?}", event));
    }

    assert_eq!(map.len(), 6);
    assert!(map.contains_key(&HookEvent::PreToolUse));
    assert!(map.contains_key(&HookEvent::PostToolUse));
}

#[test]
fn test_hook_context_default() {
    let context = HookContext::default();
    // HookContext is currently empty but reserved for future use
    let _ = context; // Just verify it can be created
}

#[test]
fn test_permission_result_serialization() {
    let allow = PermissionResult::allow();
    let json = serde_json::to_string(&allow).unwrap();
    assert!(json.contains("allow"));

    let deny = PermissionResult::deny_with_message("Not allowed");
    let json = serde_json::to_string(&deny).unwrap();
    assert!(json.contains("deny"));
    assert!(json.contains("Not allowed"));
}

#[test]
fn test_sync_hook_output_serialization() {
    let output = SyncHookOutput {
        continue_: Some(true),
        suppress_output: Some(false),
        stop_reason: Some("Test reason".to_string()),
        decision: Some("allow".to_string()),
        system_message: Some("Test message".to_string()),
        reason: Some("Test reason detail".to_string()),
        hook_specific_output: None,
    };

    let json = serde_json::to_string(&output).unwrap();

    // Verify the serialized field name is "continue" not "continue_"
    assert!(json.contains("\"continue\""));
    assert!(!json.contains("\"continue_\""));
    assert!(json.contains("\"stopReason\""));
    assert!(json.contains("\"systemMessage\""));
}

#[tokio::test]
async fn test_hook_callback_execution() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();

    let callback: HookCallback = Arc::new(move |input, _tool_use_id, _context| {
        let called = called_clone.clone();
        Box::pin(async move {
            called.store(true, Ordering::SeqCst);

            // Verify we received the expected input type
            match input {
                HookInput::PreToolUse(pre) => {
                    assert_eq!(pre.tool_name, "TestTool");
                }
                _ => panic!("Expected PreToolUse input"),
            }

            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    // Create a mock PreToolUse input
    let input = HookInput::PreToolUse(PreToolUseHookInput {
        base: claude_agents_sdk::BaseHookInput {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/test".to_string(),
            permission_mode: None,
        },
        hook_event_name: "PreToolUse".to_string(),
        tool_name: "TestTool".to_string(),
        tool_input: serde_json::json!({"param": "value"}),
    });

    // Call the callback
    let result = callback(input, None, HookContext::default()).await;

    assert!(called.load(Ordering::SeqCst));
    match result {
        HookOutput::Sync(_) => {}
        _ => panic!("Expected sync output"),
    }
}
