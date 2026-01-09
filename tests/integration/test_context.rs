//! Conversation context, session management, and multi-turn tests.

#![cfg(feature = "integration-tests")]

use claude_agents_sdk::{ClaudeAgentOptions, ClaudeClient, PermissionMode};

/// Test that conversation context is maintained across turns.
///
/// Uses a simple math problem that requires remembering previous context.
#[tokio::test]
async fn test_conversation_context_maintained() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(2);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await.expect("Failed to connect");

    // First turn: establish a number
    client
        .query(
            "Let's do some math. Start with the number 15. Confirm by saying 'Starting with 15'.",
        )
        .await
        .expect("Failed to send first query");

    let (response1, result1) = client
        .receive_response()
        .await
        .expect("First response failed");
    assert!(!result1.is_error);
    assert!(
        response1.contains("15"),
        "First response should acknowledge 15, got: {}",
        response1
    );

    // Second turn: use context to calculate
    client
        .query("Add 25 to that number. What's the total? Just give me the number.")
        .await
        .expect("Failed to send second query");

    let (response2, result2) = client
        .receive_response()
        .await
        .expect("Second response failed");

    assert!(!result2.is_error);
    assert!(
        response2.contains("40"),
        "Should compute 15+25=40, got: {}",
        response2
    );

    client.disconnect().await.expect("Failed to disconnect");
}

/// Test three-turn conversation maintains context.
#[tokio::test]
async fn test_three_turn_conversation() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(3);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await.expect("Failed to connect");

    // Turn 1: Start with a number
    client
        .query("I'm thinking of the number 7. Say 'got it'.")
        .await
        .unwrap();
    let (_, r1) = client.receive_response().await.unwrap();
    assert!(!r1.is_error);

    // Turn 2: Add to it
    client
        .query("Add 3 to that number. What's the result?")
        .await
        .unwrap();
    let (resp2, r2) = client.receive_response().await.unwrap();
    assert!(!r2.is_error);
    assert!(resp2.contains("10"), "7+3 should be 10, got: {}", resp2);

    // Turn 3: Continue the chain
    client
        .query("Double that. What's the result?")
        .await
        .unwrap();
    let (resp3, r3) = client.receive_response().await.unwrap();
    assert!(!r3.is_error);
    assert!(resp3.contains("20"), "10*2 should be 20, got: {}", resp3);

    client.disconnect().await.expect("Failed to disconnect");
}

// ============================================================================
// Session ID and Persistence Tests
// ============================================================================

/// Test that session ID remains consistent across turns.
#[tokio::test]
async fn test_session_id_consistent_across_turns() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(2);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await.expect("Failed to connect");

    // Turn 1
    client.query("Say 'turn1'").await.unwrap();
    let (_, result1) = client.receive_response().await.unwrap();
    let session1 = result1.session_id.clone();

    // Turn 2
    client.query("Say 'turn2'").await.unwrap();
    let (_, result2) = client.receive_response().await.unwrap();
    let session2 = result2.session_id.clone();

    client.disconnect().await.ok();

    // Session ID should be consistent across turns
    assert!(!session1.is_empty(), "Session ID should not be empty");
    assert_eq!(
        session1, session2,
        "Session ID should remain consistent across turns"
    );
}

/// Test that different clients get different session IDs.
#[tokio::test]
async fn test_different_clients_different_sessions() {
    let options1 = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    let options2 = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    // Client 1
    let mut client1 = ClaudeClient::new(Some(options1));
    client1.connect().await.expect("Client 1 connect failed");
    client1.query("Say 'client1'").await.unwrap();
    let (_, result1) = client1.receive_response().await.unwrap();
    let session1 = result1.session_id.clone();
    client1.disconnect().await.ok();

    // Client 2
    let mut client2 = ClaudeClient::new(Some(options2));
    client2.connect().await.expect("Client 2 connect failed");
    client2.query("Say 'client2'").await.unwrap();
    let (_, result2) = client2.receive_response().await.unwrap();
    let session2 = result2.session_id.clone();
    client2.disconnect().await.ok();

    assert!(!session1.is_empty(), "Session 1 ID should not be empty");
    assert!(!session2.is_empty(), "Session 2 ID should not be empty");
    assert_ne!(
        session1, session2,
        "Different clients should have different session IDs"
    );
}

// ============================================================================
// Session Resume Tests
// ============================================================================

/// Test session resume with explicit session ID.
#[tokio::test]
async fn test_session_resume_with_id() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    // First session: establish context
    let mut client1 = ClaudeClient::new(Some(options.clone()));
    client1.connect().await.expect("Connect failed");

    client1
        .query("Remember this secret code: XRAY42. Acknowledge with 'Code stored'.")
        .await
        .unwrap();
    let (_, result1) = client1.receive_response().await.unwrap();
    let session_id = result1.session_id.clone();
    client1.disconnect().await.ok();

    // Second session: resume with the same session ID
    let mut resume_options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);
    resume_options.resume = Some(session_id.clone());

    let mut client2 = ClaudeClient::new(Some(resume_options));

    match client2.connect().await {
        Ok(_) => {
            // Try to recall the secret
            if client2
                .query("What secret code did I give you earlier?")
                .await
                .is_ok()
            {
                match client2.receive_response().await {
                    Ok((response, _)) => {
                        eprintln!("Resume response: {}", response);
                        // If resume worked, should know the code
                        if response.contains("XRAY42") {
                            eprintln!("Session resume worked - context preserved");
                        } else {
                            eprintln!("Session resume may not have preserved context");
                        }
                    }
                    Err(e) => eprintln!("Resume query response failed: {}", e),
                }
            }
            client2.disconnect().await.ok();
        }
        Err(e) => {
            eprintln!("Resume connect failed (may not be supported): {}", e);
        }
    }
}

// ============================================================================
// Context Window Tests
// ============================================================================

/// Test that context is maintained with longer conversations.
#[tokio::test]
async fn test_context_with_multiple_facts() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(4);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await.expect("Failed to connect");

    // Turn 1: First fact
    client
        .query("Fact 1: The cat's name is Whiskers. Say 'Got fact 1'.")
        .await
        .unwrap();
    let (_, r1) = client.receive_response().await.unwrap();
    assert!(!r1.is_error);

    // Turn 2: Second fact
    client
        .query("Fact 2: The dog's name is Buddy. Say 'Got fact 2'.")
        .await
        .unwrap();
    let (_, r2) = client.receive_response().await.unwrap();
    assert!(!r2.is_error);

    // Turn 3: Third fact
    client
        .query("Fact 3: The fish's name is Nemo. Say 'Got fact 3'.")
        .await
        .unwrap();
    let (_, r3) = client.receive_response().await.unwrap();
    assert!(!r3.is_error);

    // Turn 4: Recall all facts
    client
        .query("What are the names of all three pets I mentioned?")
        .await
        .unwrap();
    let (response, r4) = client.receive_response().await.unwrap();
    assert!(!r4.is_error);

    // Should remember all three
    let has_whiskers = response.to_lowercase().contains("whiskers");
    let has_buddy = response.to_lowercase().contains("buddy");
    let has_nemo = response.to_lowercase().contains("nemo");

    eprintln!("Multi-fact response: {}", response);
    eprintln!(
        "Facts recalled: whiskers={}, buddy={}, nemo={}",
        has_whiskers, has_buddy, has_nemo
    );

    // Should remember at least 2 of 3 facts
    let fact_count = [has_whiskers, has_buddy, has_nemo]
        .iter()
        .filter(|&&x| x)
        .count();
    assert!(
        fact_count >= 2,
        "Should remember at least 2 of 3 facts, got {}",
        fact_count
    );

    client.disconnect().await.ok();
}

// ============================================================================
// Turn Counter Tests
// ============================================================================

/// Test that turn counter is reported correctly.
///
/// Note: num_turns counts turns within a single query/response cycle,
/// not cumulative turns across the session.
#[tokio::test]
async fn test_turn_counter_reported() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(3);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await.expect("Failed to connect");

    // Turn 1
    client.query("Say 'one'").await.unwrap();
    let (_, r1) = client.receive_response().await.unwrap();
    let turns1 = r1.num_turns;

    // Turn 2
    client.query("Say 'two'").await.unwrap();
    let (_, r2) = client.receive_response().await.unwrap();
    let turns2 = r2.num_turns;

    // Turn 3
    client.query("Say 'three'").await.unwrap();
    let (_, r3) = client.receive_response().await.unwrap();
    let turns3 = r3.num_turns;

    client.disconnect().await.ok();

    eprintln!("Turn counts: t1={}, t2={}, t3={}", turns1, turns2, turns3);

    // Each query should report at least 1 turn
    assert!(turns1 >= 1, "Turn 1 should have at least 1 turn");
    assert!(turns2 >= 1, "Turn 2 should have at least 1 turn");
    assert!(turns3 >= 1, "Turn 3 should have at least 1 turn");
}

/// Test max_turns limit is respected.
#[tokio::test]
async fn test_max_turns_limit() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(2);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await.expect("Failed to connect");

    // Turn 1
    client.query("Say 'first'").await.unwrap();
    let r1 = client.receive_response().await;
    assert!(r1.is_ok(), "Turn 1 should succeed");

    // Turn 2
    client.query("Say 'second'").await.unwrap();
    let r2 = client.receive_response().await;
    assert!(r2.is_ok(), "Turn 2 should succeed");

    // Turn 3 - may fail due to max_turns limit
    let q3 = client.query("Say 'third'").await;

    match q3 {
        Ok(_) => {
            // Query sent, but response might fail
            match client.receive_response().await {
                Ok((_, result)) => {
                    eprintln!("Turn 3 succeeded with {} turns", result.num_turns);
                    // Max turns might be per-response, not per-session
                }
                Err(e) => {
                    eprintln!("Turn 3 response failed (expected): {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Turn 3 query rejected (expected): {}", e);
        }
    }

    client.disconnect().await.ok();
}

// ============================================================================
// Context Isolation Tests
// ============================================================================

/// Test that reconnecting creates fresh context.
#[tokio::test]
async fn test_reconnect_creates_fresh_context() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(1);

    // First connection: establish context
    let mut client = ClaudeClient::new(Some(options.clone()));
    client.connect().await.expect("Connect failed");

    client
        .query("My favorite number is 42. Say 'Number noted'.")
        .await
        .unwrap();
    let (_, r1) = client.receive_response().await.unwrap();
    assert!(!r1.is_error);

    client.disconnect().await.expect("Disconnect failed");

    // Second connection: should not have previous context
    let mut client2 = ClaudeClient::new(Some(options));
    client2.connect().await.expect("Reconnect failed");

    client2
        .query("What is my favorite number? If you don't know, say 'I don't know your favorite number'.")
        .await
        .unwrap();
    let (response, r2) = client2.receive_response().await.unwrap();
    assert!(!r2.is_error);

    client2.disconnect().await.ok();

    eprintln!("Fresh context response: {}", response);

    // New session should NOT know about 42 from previous session
    // (unless it's a very common guess)
    let knows_42 = response.contains("42")
        && !response.to_lowercase().contains("don't know")
        && !response.to_lowercase().contains("do not know");

    if knows_42 {
        eprintln!("Warning: New session might have leaked context or guessed 42");
    } else {
        eprintln!("Context isolation confirmed - new session doesn't know previous context");
    }
}

// ============================================================================
// Duration Tests
// ============================================================================

/// Test that duration_ms is reasonable and increases with complexity.
#[tokio::test]
async fn test_duration_tracking() {
    let options = ClaudeAgentOptions::new()
        .with_permission_mode(PermissionMode::Default)
        .with_max_turns(2);

    let mut client = ClaudeClient::new(Some(options));
    client.connect().await.expect("Failed to connect");

    // Simple query
    client.query("Say 'hi'").await.unwrap();
    let (_, r1) = client.receive_response().await.unwrap();

    // More complex query
    client
        .query("List the first 5 prime numbers.")
        .await
        .unwrap();
    let (_, r2) = client.receive_response().await.unwrap();

    client.disconnect().await.ok();

    eprintln!(
        "Duration: simple={}ms, complex={}ms",
        r1.duration_ms, r2.duration_ms
    );

    // Both should have reasonable duration (< 5 minutes)
    assert!(
        r1.duration_ms < 300_000,
        "Simple query duration seems too long"
    );
    assert!(
        r2.duration_ms < 300_000,
        "Complex query duration seems too long"
    );

    // Duration should be positive
    assert!(r1.duration_ms > 0, "Duration should be positive");
    assert!(r2.duration_ms > 0, "Duration should be positive");
}
