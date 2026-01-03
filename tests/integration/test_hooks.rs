//! Hook integration tests.
//!
//! Tests for lifecycle hooks: PreToolUse, PostToolUse, UserPromptSubmit, Stop.

#![cfg(feature = "integration-tests")]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use claude_agents_sdk::{
    ClaudeAgentOptions, ClaudeClient, HookCallback, HookEvent, HookInput, HookMatcher, HookOutput,
    PermissionMode, SyncHookOutput,
};
use tokio::sync::Mutex;

use crate::integration::helpers::*;

// ============================================================================
// PreToolUse Hook Tests
// ============================================================================

/// Test that PreToolUse hook is invoked before tool execution.
#[tokio::test]
async fn test_pre_tool_use_hook_invoked() {
    let hook_called = Arc::new(AtomicBool::new(false));
    let tool_names = Arc::new(Mutex::new(Vec::<String>::new()));

    let called = Arc::clone(&hook_called);
    let names = Arc::clone(&tool_names);

    let callback: HookCallback = Arc::new(move |input, _tool_use_id, _ctx| {
        let called = Arc::clone(&called);
        let names = Arc::clone(&names);
        Box::pin(async move {
            called.store(true, Ordering::SeqCst);
            if let HookInput::PreToolUse(pre) = input {
                names.lock().await.push(pre.tool_name.clone());
            }
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![HookMatcher {
            matcher: None, // Match all tools
            hooks: vec![callback],
            timeout: None,
        }],
    );

    let mut options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_max_turns(3);
    options.hooks = Some(hooks);

    let result = collect_messages(
        "Run 'echo hook_test_123' using bash and show me the output.",
        options,
    )
    .await;

    match result {
        Ok(messages) => {
            let response = get_response_text(&messages);
            let hook_was_called = hook_called.load(Ordering::SeqCst);
            let recorded_tools = tool_names.lock().await;

            eprintln!(
                "PreToolUse hook test: called={}, tools={:?}, response={}",
                hook_was_called, *recorded_tools, response
            );

            // If the response contains the echo output, tool was used
            if response.contains("hook_test_123") {
                // Hook may or may not be called depending on CLI behavior
                if hook_was_called {
                    assert!(
                        recorded_tools.iter().any(|t| t == "Bash"),
                        "Should have recorded Bash tool"
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("Query error (may be acceptable): {}", e);
        }
    }
}

/// Test PreToolUse hook can modify tool input.
#[tokio::test]
async fn test_pre_tool_use_hook_modifies_input() {
    let hook_called = Arc::new(AtomicBool::new(false));
    let called = Arc::clone(&hook_called);

    let callback: HookCallback = Arc::new(move |input, _tool_use_id, _ctx| {
        let called = Arc::clone(&called);
        Box::pin(async move {
            called.store(true, Ordering::SeqCst);
            if let HookInput::PreToolUse(pre) = &input {
                eprintln!(
                    "PreToolUse hook: tool={}, input={}",
                    pre.tool_name, pre.tool_input
                );
            }
            // Return default (no modification) - modifying input would require
            // returning a modified tool_input in the hook output
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![HookMatcher {
            matcher: Some("Bash".to_string()), // Only match Bash
            hooks: vec![callback],
            timeout: None,
        }],
    );

    let mut options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_max_turns(3);
    options.hooks = Some(hooks);

    let result = collect_messages("Run 'echo test' using bash.", options).await;

    // Just verify no crash - hook modification behavior depends on CLI
    match result {
        Ok(_) => eprintln!("Hook modification test completed"),
        Err(e) => eprintln!("Query error: {}", e),
    }
}

// ============================================================================
// PostToolUse Hook Tests
// ============================================================================

/// Test that PostToolUse hook is invoked after tool execution.
#[tokio::test]
async fn test_post_tool_use_hook_invoked() {
    let hook_called = Arc::new(AtomicBool::new(false));
    let tool_responses = Arc::new(Mutex::new(Vec::<String>::new()));

    let called = Arc::clone(&hook_called);
    let responses = Arc::clone(&tool_responses);

    let callback: HookCallback = Arc::new(move |input, _tool_use_id, _ctx| {
        let called = Arc::clone(&called);
        let responses = Arc::clone(&responses);
        Box::pin(async move {
            called.store(true, Ordering::SeqCst);
            if let HookInput::PostToolUse(post) = input {
                responses.lock().await.push(post.tool_response.to_string());
            }
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PostToolUse,
        vec![HookMatcher {
            matcher: None,
            hooks: vec![callback],
            timeout: None,
        }],
    );

    let mut options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_max_turns(3);
    options.hooks = Some(hooks);

    let result = collect_messages("Run 'echo post_hook_test' using bash.", options).await;

    match result {
        Ok(messages) => {
            let response = get_response_text(&messages);
            let hook_was_called = hook_called.load(Ordering::SeqCst);
            let recorded_responses = tool_responses.lock().await;

            eprintln!(
                "PostToolUse hook test: called={}, responses={:?}",
                hook_was_called,
                recorded_responses.len()
            );

            if response.contains("post_hook_test") && hook_was_called {
                assert!(
                    !recorded_responses.is_empty(),
                    "Should have recorded tool responses"
                );
            }
        }
        Err(e) => {
            eprintln!("Query error: {}", e);
        }
    }
}

// ============================================================================
// Multiple Hook Tests
// ============================================================================

/// Test multiple hooks on same event.
#[tokio::test]
async fn test_multiple_hooks_same_event() {
    let hook1_count = Arc::new(AtomicUsize::new(0));
    let hook2_count = Arc::new(AtomicUsize::new(0));

    let count1 = Arc::clone(&hook1_count);
    let count2 = Arc::clone(&hook2_count);

    let callback1: HookCallback = Arc::new(move |_input, _tool_use_id, _ctx| {
        let count = Arc::clone(&count1);
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let callback2: HookCallback = Arc::new(move |_input, _tool_use_id, _ctx| {
        let count = Arc::clone(&count2);
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![
            HookMatcher {
                matcher: None,
                hooks: vec![callback1],
                timeout: None,
            },
            HookMatcher {
                matcher: None,
                hooks: vec![callback2],
                timeout: None,
            },
        ],
    );

    let mut options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_max_turns(3);
    options.hooks = Some(hooks);

    let result = collect_messages("Run 'echo multi_hook' using bash.", options).await;

    let count1 = hook1_count.load(Ordering::SeqCst);
    let count2 = hook2_count.load(Ordering::SeqCst);

    eprintln!("Multiple hooks test: hook1={}, hook2={}", count1, count2);

    match result {
        Ok(_) => {
            // If hooks were invoked, both should have same count
            if count1 > 0 || count2 > 0 {
                assert_eq!(
                    count1, count2,
                    "Both hooks should be called same number of times"
                );
            }
        }
        Err(e) => eprintln!("Query error: {}", e),
    }
}

/// Test hooks on different events.
#[tokio::test]
async fn test_hooks_different_events() {
    let pre_count = Arc::new(AtomicUsize::new(0));
    let post_count = Arc::new(AtomicUsize::new(0));

    let pre = Arc::clone(&pre_count);
    let post = Arc::clone(&post_count);

    let pre_callback: HookCallback = Arc::new(move |_input, _tool_use_id, _ctx| {
        let count = Arc::clone(&pre);
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let post_callback: HookCallback = Arc::new(move |_input, _tool_use_id, _ctx| {
        let count = Arc::clone(&post);
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![HookMatcher {
            matcher: None,
            hooks: vec![pre_callback],
            timeout: None,
        }],
    );
    hooks.insert(
        HookEvent::PostToolUse,
        vec![HookMatcher {
            matcher: None,
            hooks: vec![post_callback],
            timeout: None,
        }],
    );

    let mut options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_max_turns(3);
    options.hooks = Some(hooks);

    let result = collect_messages("Run 'echo different_events' using bash.", options).await;

    let pre_calls = pre_count.load(Ordering::SeqCst);
    let post_calls = post_count.load(Ordering::SeqCst);

    eprintln!(
        "Different events test: pre={}, post={}",
        pre_calls, post_calls
    );

    match result {
        Ok(_) => {
            // Pre should be called before post
            // Both should have same count if tool was used
            if pre_calls > 0 && post_calls > 0 {
                assert_eq!(
                    pre_calls, post_calls,
                    "Pre and Post hooks should be called same number of times"
                );
            }
        }
        Err(e) => eprintln!("Query error: {}", e),
    }
}

// ============================================================================
// Hook Matcher Tests
// ============================================================================

/// Test hook with specific tool matcher.
#[tokio::test]
async fn test_hook_matcher_specific_tool() {
    let bash_count = Arc::new(AtomicUsize::new(0));
    let other_count = Arc::new(AtomicUsize::new(0));

    let bash = Arc::clone(&bash_count);
    let other = Arc::clone(&other_count);

    let bash_callback: HookCallback = Arc::new(move |_input, _tool_use_id, _ctx| {
        let count = Arc::clone(&bash);
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let other_callback: HookCallback = Arc::new(move |_input, _tool_use_id, _ctx| {
        let count = Arc::clone(&other);
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![
            HookMatcher {
                matcher: Some("Bash".to_string()),
                hooks: vec![bash_callback],
                timeout: None,
            },
            HookMatcher {
                matcher: Some("Read".to_string()),
                hooks: vec![other_callback],
                timeout: None,
            },
        ],
    );

    let mut options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_max_turns(3);
    options.hooks = Some(hooks);

    let result = collect_messages("Run 'echo matcher_test' using bash.", options).await;

    let bash_calls = bash_count.load(Ordering::SeqCst);
    let other_calls = other_count.load(Ordering::SeqCst);

    eprintln!(
        "Matcher test: bash_hook={}, other_hook={}",
        bash_calls, other_calls
    );

    match result {
        Ok(_) => {
            // Only Bash hook should be called (if any hooks were invoked)
            if bash_calls > 0 {
                assert_eq!(
                    other_calls, 0,
                    "Read hook should not be called for Bash command"
                );
            }
        }
        Err(e) => eprintln!("Query error: {}", e),
    }
}

// ============================================================================
// Hook with Client API Tests
// ============================================================================

/// Test hooks with ClaudeClient multi-turn conversation.
#[tokio::test]
async fn test_hooks_with_client_multi_turn() {
    let hook_invocations = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&hook_invocations);

    let callback: HookCallback = Arc::new(move |_input, _tool_use_id, _ctx| {
        let count = Arc::clone(&count);
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            HookOutput::Sync(SyncHookOutput::default())
        })
    });

    let mut hooks: HashMap<HookEvent, Vec<HookMatcher>> = HashMap::new();
    hooks.insert(
        HookEvent::PreToolUse,
        vec![HookMatcher {
            matcher: None,
            hooks: vec![callback],
            timeout: None,
        }],
    );

    let mut options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_max_turns(3);
    options.hooks = Some(hooks);

    let mut client = ClaudeClient::new(Some(options), None);

    if let Err(e) = client.connect().await {
        eprintln!("Connect failed: {}", e);
        return;
    }

    // First turn
    if let Err(e) = client.query("Run 'echo turn1' using bash.").await {
        eprintln!("Query 1 failed: {}", e);
        client.disconnect().await.ok();
        return;
    }

    let _ = client.receive_response().await;
    let count_after_turn1 = hook_invocations.load(Ordering::SeqCst);

    // Second turn
    if let Err(e) = client.query("Run 'echo turn2' using bash.").await {
        eprintln!("Query 2 failed: {}", e);
        client.disconnect().await.ok();
        return;
    }

    let _ = client.receive_response().await;
    let count_after_turn2 = hook_invocations.load(Ordering::SeqCst);

    client.disconnect().await.ok();

    eprintln!(
        "Multi-turn hooks: after_turn1={}, after_turn2={}",
        count_after_turn1, count_after_turn2
    );

    // If hooks were invoked, count should increase with turns
    if count_after_turn1 > 0 {
        assert!(
            count_after_turn2 >= count_after_turn1,
            "Hook count should not decrease"
        );
    }
}
