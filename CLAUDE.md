# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

```bash
cargo build                          # Build the project
cargo test                           # Run all tests
cargo test -- --nocapture            # Run tests with output
cargo test test_parse_user_message   # Run a specific test
cargo fmt --check                    # Check formatting
cargo fmt                            # Fix formatting
cargo clippy                         # Run linter
cargo doc --open                     # Build documentation

# Run examples
cargo run --example simple_query
cargo run --example streaming_client
cargo run --example with_tools
```

## Architecture

This SDK provides a Rust client for the Claude Code CLI, communicating via JSON-RPC over stdin/stdout with a subprocess.

```
┌─────────────────┐     stdin/stdout      ┌──────────────┐
│  Rust SDK       │ ◄──── JSON-RPC ────► │  Claude CLI  │
│  (your code)    │                       │  (subprocess)│
└─────────────────┘                       └──────────────┘
```

### Two Entry Points

1. **`query()` function** (`src/query.rs`): One-shot, unidirectional queries returning an async stream
2. **`ClaudeClient`** (`src/client.rs`): Bidirectional client with persistent connection, supports multiple queries, callbacks, and runtime config changes

### Internal Layering

- `src/client.rs` / `src/query.rs` → Public API facades
- `src/_internal/client.rs` → `InternalClient` core implementation
- `src/_internal/query.rs` → Control protocol handler (request/response management)
- `src/_internal/transport/` → Abstract `Transport` trait + `SubprocessTransport` implementation
- `src/_internal/message_parser.rs` → JSON message parsing from CLI output

### Type System

All types are in `src/types.rs`:
- **Messages**: `Message` enum (User/Assistant/System/Result/StreamEvent)
- **Content**: `ContentBlock` enum (Text/Thinking/ToolUse/ToolResult)
- **Options**: `ClaudeAgentOptions` with builder pattern
- **Permissions**: `PermissionResult`, `CanUseTool` callback type
- **Hooks**: `HookEvent`, `HookInput`, `HookOutput`, `HookCallback`
- **Control Protocol**: `ControlRequest`, `ControlResponse` for CLI communication

### Callback Pattern

Tool permission and hook callbacks use this signature pattern:
```rust
Arc<dyn Fn(Input, Context) -> Pin<Box<dyn Future<Output = Result> + Send>> + Send + Sync>
```

## Porting from Python SDK

When implementing features from `claude-agent-sdk-python/`:

1. Check Python implementation first
2. Add types to `src/types.rs` with appropriate serde attributes
3. Implement logic in the corresponding module
4. Add tests mirroring Python tests in `tests/`

Key Rust adaptations:
- `serde` for serialization (not dataclasses)
- `tokio` for async (not asyncio)
- `thiserror` for errors
- Serde rename for reserved words: `async_` → `async`, `continue_` → `continue`
- Trait objects (`Box<dyn ...>`) for callbacks (not protocols)

## Feature Flags

- `default`: No extra features
- `mcp`: Enables MCP tool support via `mcp-core` crate
