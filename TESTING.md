# Testing Guide

## Quick Start

```bash
# Unit tests (no auth required)
make test

# Integration tests (requires auth)
claude setup-token          # One-time: generate OAuth token
cp .env.example .env        # One-time: create env file, paste token
make integration-test       # Run all integration tests
```

## Unit Tests

Unit tests run locally without authentication:

```bash
cargo test
# or
make test
```

These test message parsing, option building, and SDK internals.

## Integration Tests

Integration tests run against the real Claude API inside Docker containers for isolation.

### Authentication Setup

1. Generate an OAuth token (requires Claude Pro/Max):
   ```bash
   claude setup-token
   ```

2. Create `.env` file:
   ```bash
   cp .env.example .env
   ```

3. Edit `.env` and add your token:
   ```
   CLAUDE_CODE_OAUTH_TOKEN=your-token-here
   ```

### Running Tests

```bash
# All integration tests
make integration-test

# With verbose output
make integration-test-verbose

# Specific test pattern
./scripts/run-integration-tests.sh test_oneshot

# Interactive shell for debugging
make integration-shell
```

### Test Categories

| Category | File | Description |
|----------|------|-------------|
| Core | `test_core.rs` | One-shot queries, streaming, message format |
| Client | `test_client.rs` | Builder API, multi-turn, model selection |
| Context | `test_context.rs` | Conversation memory across turns |
| Callbacks | `test_callbacks.rs` | Tool permission callbacks |
| Hooks | `test_hooks.rs` | Lifecycle hooks (PreToolUse, PostToolUse) |
| Cancellation | `test_cancellation.rs` | Timeouts, disconnects, task abort |
| Resources | `test_resources.rs` | Process cleanup, no leaks |
| Errors | `test_errors.rs` | Error handling, edge cases |
| Tools | `test_tools.rs` | Tool configuration, result parsing |
| System Prompt | `test_system_prompt.rs` | Custom and preset prompts |
| Results | `test_results.rs` | Cost tracking, metadata |
| Edge Cases | `test_edge_cases.rs` | Unicode, special chars, long prompts |

### Ignored Tests

Some tests are marked `#[ignore]` and require special conditions:

- `test_authentication_failure` - Requires invalid credentials
- `stress_test_*` - Long-running stress tests

Run ignored tests using the shell:
```bash
make integration-shell
# Inside container:
cargo test --features integration-tests -- --ignored --test-threads=1
```

## Test Infrastructure

- **Docker**: Tests run in containers for isolation
- **Single-threaded**: `--test-threads=1` prevents race conditions
- **Timeouts**: All tests have timeouts to prevent hangs
- **Helpers**: Common utilities in `tests/integration/helpers.rs`

## Writing New Tests

1. Add to appropriate file in `tests/integration/`
2. Use `#![cfg(feature = "integration-tests")]` gate
3. Use helpers: `default_options()`, `collect_messages()`, etc.
4. Add timeouts for operations that could hang
5. Handle both success and acceptable failure cases

Example:
```rust
#[tokio::test]
async fn test_my_feature() {
    let options = default_options();
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        collect_messages("test prompt", options),
    ).await;

    match result {
        Ok(Ok(messages)) => {
            assert!(get_result(&messages).is_some());
        }
        Ok(Err(e)) => {
            // Handle acceptable errors
            eprintln!("Error (may be acceptable): {}", e);
        }
        Err(_) => panic!("Test timed out"),
    }
}
```
