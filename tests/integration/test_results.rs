//! Result message and system message tests.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::Message;

use crate::integration::helpers::*;

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
    }
    // Note: total_cost_usd might be None in some configurations
}
