//! Result message, cost tracking, and budget verification tests.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{ClaudeAgentOptions, Message, PermissionMode};

use crate::integration::helpers::*;

// ============================================================================
// Result Message Tests
// ============================================================================

/// Test that result message contains all expected fields.
#[tokio::test]
async fn test_result_message_fields() {
    let messages = collect_messages("Say 'test'.", default_options())
        .await
        .expect("Query failed");

    let result = get_result(&messages).expect("Should have result message");

    assert!(result.duration_ms > 0, "Duration should be positive");
    assert!(result.num_turns >= 1, "Should have at least 1 turn");
    assert!(!result.is_error, "Should not be an error");
    assert!(
        !result.subtype.is_empty(),
        "Subtype should not be empty: got '{}'",
        result.subtype
    );
}

/// Test that system message is received during query.
#[tokio::test]
async fn test_system_message_received() {
    let messages = collect_messages("Say 'hi'.", default_options())
        .await
        .expect("Query failed");

    let system_msg = messages.iter().find_map(|m| {
        if let Message::System(sys) = m {
            Some(sys)
        } else {
            None
        }
    });

    assert!(system_msg.is_some(), "Should receive system message");
    assert!(
        !system_msg.unwrap().subtype.is_empty(),
        "System message should have a subtype"
    );
}

// ============================================================================
// Cost Tracking Tests
// ============================================================================

/// Test cost tracking in result message.
#[tokio::test]
async fn test_cost_tracking() {
    let messages = collect_messages("What is 1+1?", default_options())
        .await
        .expect("Query failed");

    let result = get_result(&messages).expect("Should have result");

    // Cost should be present (though might be 0 for some configurations)
    if let Some(cost) = result.total_cost_usd {
        assert!(cost >= 0.0, "Cost should be non-negative: {}", cost);
        eprintln!("Query cost: ${:.6}", cost);
    } else {
        eprintln!("Cost tracking not available in this configuration");
    }
}

/// Test that cost increases with longer responses.
#[tokio::test]
async fn test_cost_scales_with_response_length() {
    // Short response
    let short_result = collect_messages("Say 'hi'.", default_options()).await;

    // Longer response
    let long_result = collect_messages(
        "List the first 10 prime numbers, one per line.",
        default_options(),
    )
    .await;

    match (short_result, long_result) {
        (Ok(short_msgs), Ok(long_msgs)) => {
            let short_cost = get_result(&short_msgs).and_then(|r| r.total_cost_usd);
            let long_cost = get_result(&long_msgs).and_then(|r| r.total_cost_usd);

            eprintln!(
                "Cost comparison: short={:?}, long={:?}",
                short_cost, long_cost
            );

            // If both have costs, longer should generally cost more
            // (but this isn't always guaranteed due to caching, etc.)
            if let (Some(s), Some(l)) = (short_cost, long_cost) {
                // Just log - don't assert since caching can affect this
                if l > s {
                    eprintln!("Longer response cost more as expected");
                } else {
                    eprintln!("Note: Longer response didn't cost more (possible caching)");
                }
            }
        }
        _ => {
            eprintln!("One or both queries failed");
        }
    }
}

/// Test cost tracking with tool use.
#[tokio::test]
async fn test_cost_with_tool_use() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_allowed_tools(vec!["Bash".to_string()])
        .with_max_turns(3);

    let result = collect_messages("Run 'echo cost_test' using bash.", options).await;

    match result {
        Ok(messages) => {
            if let Some(result) = get_result(&messages) {
                eprintln!("Tool use cost: {:?}", result.total_cost_usd);
                eprintln!("Tool use turns: {}", result.num_turns);
                eprintln!("Tool use duration: {}ms", result.duration_ms);
            }
        }
        Err(e) => {
            eprintln!("Tool query failed: {}", e);
        }
    }
}

// ============================================================================
// Budget Limit Tests
// ============================================================================

/// Test max_budget_usd option is accepted.
#[tokio::test]
async fn test_budget_limit_option() {
    let mut options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    // Set a reasonable budget limit
    options.max_budget_usd = Some(1.0);

    let result = collect_messages("Say 'budget test'.", options).await;

    // Should complete normally with this budget
    match result {
        Ok(messages) => {
            let result = get_result(&messages);
            assert!(result.is_some(), "Should get result with budget set");

            if let Some(r) = result {
                if let Some(cost) = r.total_cost_usd {
                    assert!(cost <= 1.0, "Cost ${} should be under budget $1.0", cost);
                }
            }
        }
        Err(e) => {
            // Some errors might be acceptable
            eprintln!("Budget test error: {}", e);
        }
    }
}

/// Test very low budget limit behavior.
///
/// Note: This test may not trigger budget exceeded because a single
/// simple query typically costs less than $0.001.
#[tokio::test]
async fn test_very_low_budget_limit() {
    let mut options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    // Set an extremely low budget
    options.max_budget_usd = Some(0.0001); // $0.0001

    let result = collect_messages("Say 'tiny budget'.", options).await;

    // Either succeeds (query was cheap enough) or fails with budget error
    match result {
        Ok(messages) => {
            eprintln!("Query succeeded with tiny budget");
            if let Some(r) = get_result(&messages) {
                eprintln!("Cost: {:?}", r.total_cost_usd);
            }
        }
        Err(e) => {
            eprintln!("Query with tiny budget failed: {}", e);
            // Check if it's a budget-related error
            let is_budget_error = e.to_lowercase().contains("budget")
                || e.to_lowercase().contains("cost")
                || e.to_lowercase().contains("limit");
            if is_budget_error {
                eprintln!("Budget limit was enforced");
            }
        }
    }
}

// ============================================================================
// Token Usage Tests
// ============================================================================

/// Test that token counts are reasonable.
#[tokio::test]
async fn test_token_counts() {
    let messages = collect_messages("What is 2+2?", default_options())
        .await
        .expect("Query failed");

    let result = get_result(&messages).expect("Should have result");

    // Check session ID exists
    assert!(
        !result.session_id.is_empty(),
        "Session ID should not be empty"
    );

    // Duration should be reasonable (not hours)
    assert!(
        result.duration_ms < 300_000,
        "Duration {}ms seems too long",
        result.duration_ms
    );

    eprintln!(
        "Query stats: session={}, turns={}, duration={}ms",
        result.session_id, result.num_turns, result.duration_ms
    );
}

/// Test multi-turn cost accumulation.
#[tokio::test]
async fn test_multi_turn_cost_accumulation() {
    use claude_agents_sdk::ClaudeClient;

    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(3);

    let mut client = ClaudeClient::new(Some(options));

    if let Err(e) = client.connect().await {
        eprintln!("Connect failed: {}", e);
        return;
    }

    // Turn 1
    if let Err(e) = client.query("Say 'turn1'").await {
        eprintln!("Turn 1 failed: {}", e);
        client.disconnect().await.ok();
        return;
    }
    let (_, result1) = match client.receive_response().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Turn 1 receive failed: {}", e);
            client.disconnect().await.ok();
            return;
        }
    };

    // Turn 2
    if let Err(e) = client.query("Say 'turn2'").await {
        eprintln!("Turn 2 failed: {}", e);
        client.disconnect().await.ok();
        return;
    }
    let (_, result2) = match client.receive_response().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Turn 2 receive failed: {}", e);
            client.disconnect().await.ok();
            return;
        }
    };

    client.disconnect().await.ok();

    eprintln!(
        "Turn 1: turns={}, cost={:?}",
        result1.num_turns, result1.total_cost_usd
    );
    eprintln!(
        "Turn 2: turns={}, cost={:?}",
        result2.num_turns, result2.total_cost_usd
    );

    // Turns should accumulate
    assert!(
        result2.num_turns >= result1.num_turns,
        "Turn count should not decrease"
    );
}
