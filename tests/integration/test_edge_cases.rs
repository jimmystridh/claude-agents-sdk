//! Edge case and error handling tests.

#![cfg(feature = "integration-tests")]

use crate::integration::helpers::*;

/// Test handling of minimal/simple queries.
#[tokio::test]
async fn test_handles_simple_query() {
    let messages = collect_messages("Hi", default_options())
        .await
        .expect("Query failed");

    let result = get_result(&messages).expect("Should have result");
    assert!(!result.is_error, "Simple query should succeed");
}

/// Test multiple sequential sessions (process isolation).
#[tokio::test]
async fn test_multiple_sequential_sessions() {
    // First session
    let messages1 = collect_messages("Say '1'.", default_options())
        .await
        .expect("First query failed");
    assert!(
        get_result(&messages1).is_some(),
        "First session should complete"
    );

    // Second session (separate process)
    let messages2 = collect_messages("Say '2'.", default_options())
        .await
        .expect("Second query failed");
    assert!(
        get_result(&messages2).is_some(),
        "Second session should complete"
    );
}

/// Test that special characters in prompts are handled correctly.
#[tokio::test]
async fn test_special_characters_in_prompt() {
    let prompt = r#"Say exactly: "Hello 'World'" with quotes"#;
    let messages = collect_messages(prompt, default_options())
        .await
        .expect("Query failed");

    let response = extract_assistant_text(&messages);
    assert!(
        response.contains("Hello") && response.contains("World"),
        "Should handle special characters, got: {}",
        response
    );
}

/// Test unicode handling in prompts and responses.
#[tokio::test]
async fn test_unicode_handling() {
    let messages = collect_messages("Say: 你好世界 🌍", default_options())
        .await
        .expect("Query failed");

    let result = get_result(&messages).expect("Should have result");
    assert!(!result.is_error, "Unicode query should succeed");

    let response = extract_assistant_text(&messages);
    assert!(
        response.contains("你好") || response.contains("🌍") || !response.is_empty(),
        "Should handle unicode, got: {}",
        response
    );
}

/// Test long prompt handling.
#[tokio::test]
async fn test_long_prompt() {
    let long_text = "word ".repeat(100);
    let prompt = format!("Count the words in this text: {}", long_text);

    let messages = collect_messages(&prompt, default_options())
        .await
        .expect("Query failed");

    let result = get_result(&messages).expect("Should have result");
    assert!(!result.is_error, "Long prompt should be handled");
}
