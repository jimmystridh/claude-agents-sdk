# Integration Testing Tasks

Actionable task breakdown for implementing the integration testing plan.

---

## Legend

- `[ ]` Not started
- `[~]` In progress
- `[x]` Complete
- `[!]` Blocked
- `[-]` Skipped/Not applicable

**Effort**: S (< 1 hour), M (1-4 hours), L (4-8 hours), XL (> 8 hours)

---

## Phase 1: Error Path Testing

**Priority**: High | **Total Effort**: L

### 1.1 CLI Executable Errors

- [x] **Create `tests/integration/test_errors.rs`** [S]
  - [x] Add feature gate and imports
  - [x] Add module to `tests/integration/mod.rs`

- [-] **Test: CLI not found** [M] *(SDK doesn't support custom CLI path yet)*
  - [ ] Verify SDK supports custom CLI path (check API)
  - [ ] If supported: test with `/nonexistent/path/to/claude`
  - [ ] If not supported: add `with_cli_path()` to `ClaudeAgentOptions` first
  - [ ] Assert returns `ClaudeSDKError::CliNotFound` or similar
  - [ ] Verify error message contains useful information

- [-] **Test: CLI not executable** [S] *(Requires custom CLI path support)*
  - [ ] Create temp file (not executable)
  - [ ] Attempt to use as CLI path
  - [ ] Verify graceful error (not panic)

- [-] **Test: CLI version mismatch** [M] *(Requires custom CLI path support)*
  - [ ] Research how version mismatch manifests
  - [ ] Create test that triggers version check failure
  - [ ] Verify `ClaudeSDKError::VersionMismatch` handling

### 1.2 Connection Failures

- [x] **Test: Connection timeout** [M]
  - [x] Use very short timeout (1 second)
  - [x] Verify timeout error is returned
  - [x] Verify timing is approximately correct

- [x] **Test: CLI unexpected exit** [M]
  - [x] Test error handling when process terminates
  - [x] Verify no hang/deadlock occurs

- [-] **Test: Stdin/stdout pipe failures** [S]
  - [-] Test behavior when pipes are closed unexpectedly
  - [-] May require mock transport for unit test

### 1.3 Malformed Response Handling

- [x] **Test: Special characters in prompts** [S]
  - [x] Null bytes (`\x00`)
  - [x] JSON-like strings
  - [x] Only newlines
  - [x] Markdown code blocks
  - [x] Verify no panics, graceful errors acceptable

- [x] **Test: Very long prompts** [S]
  - [x] Test prompt > 10KB
  - [x] Verify handling or clear error

### 1.4 API Error Responses

- [x] **Test: Authentication failure** [M] `[ignore]`
  - [x] Document how to run with invalid credentials
  - [x] Verify auth error is reported clearly
  - [x] Mark as `#[ignore]` for manual runs

- [-] **Test: Rate limiting** [S] `[ignore]`
  - [-] Difficult to trigger reliably
  - [-] Deferred

---

## Phase 2: Resource Management Testing

**Priority**: High | **Total Effort**: L

### 2.1 Setup

- [x] **Create `tests/integration/test_resources.rs`** [S]
  - [x] Add feature gate and imports
  - [x] Add module to `tests/integration/mod.rs`

- [x] **Implement `count_claude_processes()` helper** [M]
  - [x] Unix implementation using `pgrep`
  - [x] Windows implementation using `tasklist`
  - [x] Handle cases where command not available
  - [x] Add to helpers.rs

### 2.2 Process Cleanup Tests

- [x] **Test: Cleanup after normal disconnect** [M]
  - [x] Record initial process count
  - [x] Run 3+ sessions with proper connect/query/disconnect
  - [x] Verify process count returns to baseline
  - [x] Allow small tolerance for timing

- [x] **Test: Cleanup on stream drop** [M]
  - [x] Start query, read partial messages
  - [x] Drop stream without exhausting
  - [x] Verify process cleanup
  - [x] Repeat 3+ times

- [x] **Test: Cleanup on client drop without disconnect** [M]
  - [x] Connect and query
  - [x] Drop client without calling disconnect()
  - [x] Verify process cleanup via Drop impl

### 2.3 Handle/Resource Leak Tests

- [x] **Test: No leaks across many sessions** [M]
  - [x] Run 10+ sequential sessions
  - [x] Verify stable resource usage
  - [x] No OOM or handle exhaustion

- [x] **Test: No leaks with concurrent sessions** [M]
  - [x] Run 3 batches of 3 concurrent sessions
  - [x] Verify stable resource usage
  - [x] Brief pause between batches

---

## Phase 3: Cancellation Testing

**Priority**: Medium | **Total Effort**: M

### 3.1 Setup

- [x] **Create `tests/integration/test_cancellation.rs`** [S]
  - [x] Add feature gate and imports
  - [x] Add module to `tests/integration/mod.rs`

### 3.2 Stream Cancellation

- [x] **Test: Drop stream mid-query** [M]
  - [x] Start long query (multi-turn or long response)
  - [x] Read 1-2 messages
  - [x] Drop stream
  - [x] Verify new query works afterward

- [x] **Test: Disconnect during active query** [M]
  - [x] Start query
  - [x] Read partial messages
  - [x] Call disconnect() immediately
  - [x] Verify no panic, no hang
  - [x] Verify new client can connect

### 3.3 Tokio Integration

- [x] **Test: tokio::select! cancellation** [M]
  - [x] Race receive loop against timeout
  - [x] Trigger timeout branch
  - [x] Cleanup client
  - [x] Verify system still functional

- [x] **Test: Task abort** [S]
  - [x] Spawn query task
  - [x] Abort task handle
  - [x] Verify cleanup occurs

### 3.4 Timeout Behavior

- [x] **Test: SDK timeout respected** [M]
  - [x] Set short timeout (5s)
  - [x] Run query that exceeds timeout
  - [x] Verify timeout fires approximately on time
  - [x] Verify error type is timeout-related

---

## Phase 4: Concurrent Session Testing

**Priority**: Medium | **Total Effort**: M

### 4.1 Setup

- [ ] **Create `tests/integration/test_concurrent_sessions.rs`** [S]
  - [ ] Add feature gate and imports
  - [ ] Add module to `tests/integration/mod.rs`

### 4.2 Parallel Query Tests

- [ ] **Test: Concurrent query() calls** [M]
  - [ ] Spawn 3 concurrent query tasks
  - [ ] Collect results
  - [ ] Verify at least 2/3 succeed
  - [ ] Document expected success rate

- [ ] **Test: Concurrent ClaudeClient instances** [M]
  - [ ] Create 3 clients
  - [ ] Run queries in parallel
  - [ ] Verify responses match expected clients
  - [ ] Proper cleanup of all clients

### 4.3 Interleaved Operations

- [ ] **Test: Interleaved client operations** [M]
  - [ ] Create 2 clients
  - [ ] Connect both
  - [ ] Send queries to both
  - [ ] Receive in opposite order
  - [ ] Verify correct responses for each

- [ ] **Test: Shared options with concurrent use** [S]
  - [ ] Create Arc<ClaudeAgentOptions>
  - [ ] Clone and use from multiple tasks
  - [ ] Verify thread-safety in practice

---

## Phase 5: Callback Integration Testing

**Priority**: Medium | **Total Effort**: L

### 5.1 Setup

- [x] **Create `tests/integration/test_callbacks.rs`** [S]
  - [x] Add feature gate and imports
  - [x] Add module to `tests/integration/mod.rs`

### 5.2 Tool Permission Callback

- [x] **Test: Callback actually invoked** [L]
  - [x] Set up callback with AtomicBool flag
  - [x] Query that triggers tool use (e.g., "Run echo using bash")
  - [x] Verify flag is set after query
  - [x] Verify tool name is captured

- [x] **Test: Deny prevents tool use** [M]
  - [x] Set up callback that always denies
  - [x] Query requiring tool use
  - [x] Verify tool result contains denial/error
  - [x] Verify callback was called

- [x] **Test: Callback receives correct data** [M]
  - [x] Capture tool_name and input in callback
  - [x] Query with specific tool request
  - [x] Verify captured data matches expected
  - [x] Check input structure (e.g., Bash has "command")

- [ ] **Test: Callback context contains expected data** [S]
  - [ ] Capture ToolPermissionContext in callback
  - [ ] Verify session_id present
  - [ ] Verify cwd present
  - [ ] Verify other context fields

### 5.3 Callback Error Handling

- [x] **Test: Callback panic handling** [M]
  - [x] Test callback returning error result
  - [x] Verify SDK handles gracefully

- [ ] **Test: Callback timeout** [M]
  - [ ] Create slow callback (sleep 10s)
  - [ ] Verify timeout mechanism (if any)
  - [ ] Or document no timeout exists

---

## Phase 6: Hook Integration Testing

**Priority**: Medium | **Total Effort**: L

### 6.1 Setup

- [ ] **Create `tests/integration/test_hooks.rs`** [S]
  - [ ] Add feature gate and imports
  - [ ] Add module to `tests/integration/mod.rs`

### 6.2 Pre-Tool-Use Hooks

- [ ] **Test: PreToolUse hook invoked** [M]
  - [ ] Register PreToolUse hook with counter
  - [ ] Query triggering tool use
  - [ ] Verify counter incremented
  - [ ] Log count for visibility

- [ ] **Test: PreToolUse receives correct input** [M]
  - [ ] Capture HookInput in hook
  - [ ] Verify tool_name present
  - [ ] Verify tool_input structure
  - [ ] Verify session context

### 6.3 Post-Tool-Use Hooks

- [ ] **Test: PostToolUse hook invoked** [M]
  - [ ] Register PostToolUse hook with counter
  - [ ] Query triggering tool use
  - [ ] Verify counter incremented
  - [ ] Verify called after tool execution

- [ ] **Test: PostToolUse receives tool result** [M]
  - [ ] Capture post-hook input
  - [ ] Verify tool result/output present
  - [ ] Verify tool_use_id linkage

### 6.4 Hook Behavior

- [ ] **Test: Hook can modify output** [M]
  - [ ] Return HookOutput with modifications
  - [ ] Verify modifications are applied
  - [ ] Or document limitations

- [ ] **Test: Hook matcher filters correctly** [M]
  - [ ] Register hooks for "Bash" and "Read"
  - [ ] Query using Bash tool only
  - [ ] Verify only Bash hook called
  - [ ] Read hook counter stays zero

- [ ] **Test: Multiple hooks same event** [S]
  - [ ] Register 2+ hooks for same event
  - [ ] Verify all are called
  - [ ] Verify order (if deterministic)

---

## Phase 7: Budget/Cost Verification

**Priority**: Medium | **Total Effort**: M

### 7.1 Setup

- [ ] **Create `tests/integration/test_budget.rs`** [S]
  - [ ] Add feature gate and imports
  - [ ] Add module to `tests/integration/mod.rs`

### 7.2 Budget Enforcement

- [ ] **Test: max_budget_usd enforced** [M]
  - [ ] Set very low budget (0.0001)
  - [ ] Run expensive query (long response)
  - [ ] Verify early termination or budget error
  - [ ] Verify cost near limit

- [ ] **Test: Budget error type** [S]
  - [ ] Trigger budget exceeded
  - [ ] Verify result.subtype == "error_max_budget_usd"
  - [ ] Or verify appropriate error variant

### 7.3 Cost Reporting

- [ ] **Test: Cost fields present** [S]
  - [ ] Run simple query
  - [ ] Verify total_cost_usd populated
  - [ ] Verify cost is reasonable (> 0, < $1)

- [ ] **Test: Duration fields accurate** [S]
  - [ ] Run query
  - [ ] Verify duration_ms > 0
  - [ ] Verify duration_api_ms >= 0
  - [ ] Verify duration_ms >= duration_api_ms

- [ ] **Test: Usage statistics** [M]
  - [ ] Run query
  - [ ] Check result.usage field
  - [ ] Verify input_tokens present
  - [ ] Verify output_tokens present
  - [ ] Log values for reference

---

## Phase 8: Context and Session Management

**Priority**: Medium | **Total Effort**: M

### 8.1 Setup

- [ ] **Create `tests/integration/test_context_extended.rs`** [S]
  - [ ] Add feature gate and imports
  - [ ] Add module to `tests/integration/mod.rs`

### 8.2 Session Resume

- [ ] **Test: Resume by session_id** [M]
  - [ ] Run initial query, capture session_id
  - [ ] Create new options with resume(session_id)
  - [ ] Query referencing previous context
  - [ ] Verify context is accessible

- [ ] **Test: Invalid session_id handling** [S]
  - [ ] Use fake/invalid session_id
  - [ ] Verify graceful error or fresh session

### 8.3 Continue Conversation

- [ ] **Test: continue_conversation flag** [M]
  - [ ] Multi-turn with ClaudeClient
  - [ ] Set flag between queries
  - [ ] Verify context maintained
  - [ ] Test with number memory ("I'm thinking of 7")

- [ ] **Test: Context isolation without flag** [S]
  - [ ] Query without continue flag
  - [ ] Verify no context bleed

### 8.4 Working Directory

- [ ] **Test: Custom cwd respected** [M]
  - [ ] Create temp directory with test file
  - [ ] Set cwd to temp directory
  - [ ] Query to read relative file
  - [ ] Verify correct file read

- [ ] **Test: Invalid cwd handling** [S]
  - [ ] Set cwd to nonexistent path
  - [ ] Verify error or fallback behavior

---

## Phase 9: Property-Based Testing

**Priority**: Low | **Total Effort**: L

### 9.1 Setup

- [ ] **Add proptest dependency** [S]
  - [ ] Add to Cargo.toml as optional dev-dependency
  - [ ] Gate behind `property-tests` feature
  - [ ] Create `tests/property_tests.rs`

### 9.2 Strategies

- [ ] **Implement arb_text() strategy** [S]
  - [ ] Generate arbitrary valid text

- [ ] **Implement arb_tool_use() strategy** [S]
  - [ ] Generate valid ToolUseBlock

- [ ] **Implement arb_content_block() strategy** [S]
  - [ ] Combine text and tool_use strategies

- [ ] **Implement arb_assistant_message() strategy** [M]
  - [ ] Use content_block strategy
  - [ ] Add model string

- [ ] **Implement arb_result_message() strategy** [S]
  - [ ] Generate all numeric fields
  - [ ] Generate session_id

### 9.3 Property Tests

- [ ] **Test: Message serialization roundtrip** [M]
  - [ ] Serialize to JSON
  - [ ] Deserialize back
  - [ ] Verify equality

- [ ] **Test: text() never panics** [S]
  - [ ] Generate arbitrary AssistantMessage
  - [ ] Call text()
  - [ ] Verify no panic

- [ ] **Test: tool_uses() correct count** [S]
  - [ ] Generate arbitrary content
  - [ ] Count ToolUse blocks manually
  - [ ] Compare to tool_uses().len()

- [ ] **Test: ResultMessage roundtrip** [M]
  - [ ] Generate with arbitrary values
  - [ ] Serialize/deserialize
  - [ ] Verify field equality

---

## Phase 10: Test Infrastructure

**Priority**: Low | **Total Effort**: M

### 10.1 Helper Improvements

- [x] **Add collect_messages_verbose()** [S]
  - [x] Detailed error reporting
  - [x] Log message count on error

- [x] **Add assert_response_contains()** [S]
  - [x] Case-insensitive comparison
  - [x] Detailed assertion message

- [x] **Add extract_tool_uses()** [S]
  - [x] Collect all ToolUseBlock from messages
  - [x] Return as Vec<&ToolUseBlock>

- [x] **Add with_retry() helper** [M]
  - [x] Retry async function with backoff
  - [x] Configurable max attempts
  - [x] Log retry attempts

### 10.2 Test Organization

- [x] **Add stress-tests feature flag** [S]
  - [x] Update Cargo.toml
  - [x] Gate expensive tests

- [x] **Add property-tests feature flag** [S]
  - [x] Update Cargo.toml
  - [x] Make proptest optional

- [x] **Update mod.rs for new modules** [S]
  - [x] test_errors
  - [x] test_resources
  - [x] test_cancellation
  - [-] test_concurrent_sessions (not created yet)
  - [x] test_callbacks
  - [-] test_hooks (not created yet)
  - [-] test_budget (not created yet)
  - [-] test_context_extended (not created yet)

### 10.3 CI Updates

- [x] **Add Docker-based integration test setup** [M]
  - [x] Dockerfile.integration-tests
  - [x] docker-compose.integration-tests.yml
  - [x] scripts/run-integration-tests.sh
  - [x] .env.example with authentication options
  - [x] Support for both ANTHROPIC_API_KEY and CLAUDE_CODE_OAUTH_TOKEN
  - [x] Added .env to .gitignore

- [ ] **Add GitHub Actions integration test job** [M]
  - [ ] Trigger on schedule or commit message
  - [ ] Install Claude CLI
  - [ ] Set API key from secrets
  - [ ] Run with --test-threads=1

- [ ] **Add stress test job** [S]
  - [ ] Schedule only (nightly/weekly)
  - [ ] Run ignored tests

- [ ] **Add property test job** [S]
  - [ ] Run on every PR
  - [ ] Fast, no external deps

---

## Dependency Graph

```
Phase 10.2 (features) ─┬─► Phase 9 (property tests)
                       │
Phase 10.1 (helpers) ──┼─► Phase 1-8 (all tests)
                       │
                       └─► Phase 10.3 (CI)

Phase 1 (errors) ──────► Independent
Phase 2 (resources) ───► Independent
Phase 3 (cancellation) ► Depends on Phase 2 concepts
Phase 4 (concurrent) ──► Independent
Phase 5 (callbacks) ───► Independent
Phase 6 (hooks) ───────► Similar to Phase 5
Phase 7 (budget) ──────► Independent
Phase 8 (context) ─────► Independent
```

---

## Quick Start Order

For fastest value delivery:

1. **Phase 10.1** - Helpers (enables better tests)
2. **Phase 1.1** - CLI errors (high impact, low effort)
3. **Phase 2.2** - Process cleanup (critical for stability)
4. **Phase 5.2** - Tool callback (validates key feature)
5. **Phase 3.2** - Stream cancellation (common failure mode)
6. **Phase 4.2** - Parallel queries (real-world usage)

---

## Tracking Summary

| Phase | Tasks | Subtasks | Status |
|-------|-------|----------|--------|
| 1. Error Paths | 4 | 14 | [x] Complete (some deferred) |
| 2. Resources | 3 | 10 | [x] Complete |
| 3. Cancellation | 4 | 8 | [x] Complete |
| 4. Concurrent | 3 | 6 | [ ] Not started |
| 5. Callbacks | 4 | 9 | [~] Mostly complete |
| 6. Hooks | 4 | 10 | [ ] Not started |
| 7. Budget | 3 | 7 | [ ] Not started |
| 8. Context | 4 | 7 | [ ] Not started |
| 9. Property | 3 | 10 | [ ] Not started |
| 10. Infra | 3 | 11 | [~] Mostly complete |
| **Total** | **35** | **92** | |

### Progress Notes

- **2024-01-XX**: Created initial test files for Phases 1, 2, 3, 5, 10
- Docker-based testing infrastructure added
- Helpers extended with `count_claude_processes()`, `with_retry()`, `extract_tool_uses()`

---

## Notes

- Tasks marked `[ignore]` require manual runs with specific setup
- Some tasks may reveal SDK limitations requiring feature additions
- Update this file as tasks complete
- Reference `INTEGRATION_TESTING_PLAN.md` for implementation details
