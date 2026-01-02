# claude-agents-sdk

A Rust SDK for building agents that interact with the Claude Code CLI.

Inspired by the [Python Claude Agent SDK](https://github.com/anthropics/claude-agent-sdk), providing similar functionality with idiomatic Rust APIs.

## Features

- **Simple Query API**: One-shot queries with `query()` function
- **Streaming Client**: Full bidirectional `ClaudeClient` for complex interactions
- **Tool Permissions**: Control which tools Claude can use with callbacks
- **Hooks**: Register callbacks for various lifecycle events
- **MCP Tools**: Define custom tools that run in-process
- **Type Safety**: Strongly-typed messages, content blocks, and options
- **Async/Await**: Built on Tokio for efficient async operations

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
claude-agents-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
```

For MCP tool support:

```toml
[dependencies]
claude-agents-sdk = { version = "0.1", features = ["mcp"] }
```

## Prerequisites

- [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated
- Rust 1.75 or later

## Quick Start

### Simple Query

```rust
use claude_agents_sdk::{query, ClaudeAgentOptions, Message};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::new()
        .with_max_turns(3);

    let mut stream = query("What is 2 + 2?", Some(options), None).await?;

    while let Some(message) = stream.next().await {
        match message? {
            Message::Assistant(msg) => print!("{}", msg.text()),
            Message::Result(result) => {
                println!("\nCost: ${:.4}", result.total_cost_usd.unwrap_or(0.0));
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Streaming Client

```rust
use claude_agents_sdk::{ClaudeClient, ClaudeAgentOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClaudeClient::new(None, None);
    client.connect().await?;

    // First query
    client.query("What is the capital of France?").await?;
    let (response, _) = client.receive_response().await?;
    println!("Response: {}", response);

    // Follow-up query
    client.query("What's its population?").await?;
    let (response, _) = client.receive_response().await?;
    println!("Response: {}", response);

    client.disconnect().await?;
    Ok(())
}
```

### With Tool Permissions

```rust
use claude_agents_sdk::{ClaudeClientBuilder, PermissionResult, PermissionMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClaudeClientBuilder::new()
        .permission_mode(PermissionMode::Default)
        .can_use_tool(|tool_name, input, _ctx| async move {
            println!("Tool requested: {} with {:?}", tool_name, input);

            // Allow Read, deny dangerous Bash commands
            if tool_name == "Bash" {
                if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                    if cmd.contains("rm -rf") {
                        return PermissionResult::deny_with_message("Dangerous command");
                    }
                }
            }

            PermissionResult::allow()
        })
        .build();

    client.connect().await?;
    // ... use client
    Ok(())
}
```

## API Reference

### Entry Points

- `query(prompt, options, transport)` - One-shot query returning a message stream
- `query_all(prompt, options)` - Collect all messages from a query
- `query_result(prompt, options)` - Get final response and result metadata

### ClaudeClient

```rust
let mut client = ClaudeClient::new(options, transport);
client.connect().await?;
client.query("Hello").await?;
let (response, result) = client.receive_response().await?;
client.disconnect().await?;
```

Methods:
- `connect()` - Connect to CLI
- `query(prompt)` - Send a query
- `receive_messages()` - Stream of messages
- `receive_response()` - Collect response and result
- `interrupt()` - Interrupt current operation
- `set_permission_mode(mode)` - Change permission mode
- `set_model(model)` - Change model
- `rewind_files(message_id)` - Rewind to checkpoint
- `disconnect()` - Disconnect from CLI

### ClaudeAgentOptions

```rust
let options = ClaudeAgentOptions::new()
    .with_model("claude-3-sonnet")
    .with_system_prompt("You are helpful.")
    .with_max_turns(10)
    .with_permission_mode(PermissionMode::AcceptEdits)
    .with_allowed_tools(vec!["Bash".into(), "Read".into()])
    .with_partial_messages();
```

### Message Types

```rust
enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
    Result(ResultMessage),
    StreamEvent(StreamEvent),
}

// AssistantMessage has helpful methods
let text = assistant_msg.text();
let tool_uses = assistant_msg.tool_uses();
```

### Content Blocks

```rust
enum ContentBlock {
    Text(TextBlock),
    Thinking(ThinkingBlock),
    ToolUse(ToolUseBlock),
    ToolResult(ToolResultBlock),
}
```

## Examples

Run the examples:

```bash
# Simple query
cargo run --example simple_query

# Streaming client with multiple queries
cargo run --example streaming_client

# Tool permission callback
cargo run --example with_tools
```

## Error Handling

The SDK provides `ClaudeSDKError`, a comprehensive error type for all failure modes.

### Error Types

```rust
use claude_agents_sdk::ClaudeSDKError;

match result {
    // CLI not found or not installed
    Err(ClaudeSDKError::CLINotFound { message }) => {
        eprintln!("Claude CLI not installed: {}", message);
        eprintln!("Install from: https://docs.anthropic.com/en/docs/claude-code");
    }

    // CLI process exited with error
    Err(ClaudeSDKError::Process { exit_code, stderr, .. }) => {
        eprintln!("CLI failed with exit code {:?}", exit_code);
        if let Some(err) = stderr {
            eprintln!("stderr: {}", err);
        }
    }

    // Operation timed out
    Err(ClaudeSDKError::Timeout { duration_ms }) => {
        eprintln!("Operation timed out after {}ms", duration_ms);
    }

    // Connection to CLI failed
    Err(ClaudeSDKError::CLIConnection { message }) => {
        eprintln!("Failed to connect to CLI: {}", message);
    }

    // Invalid configuration
    Err(ClaudeSDKError::Configuration { message }) => {
        eprintln!("Configuration error: {}", message);
    }

    // Message parsing failed (malformed JSON from CLI)
    Err(ClaudeSDKError::MessageParse { message, .. }) => {
        eprintln!("Failed to parse CLI message: {}", message);
    }

    // Control protocol error
    Err(ClaudeSDKError::ControlProtocol { message, .. }) => {
        eprintln!("Control protocol error: {}", message);
    }

    // All other errors
    Err(e) => eprintln!("Error: {}", e),

    Ok(_) => {}
}
```

### Recoverable Errors

Some errors can be retried:

```rust
if error.is_recoverable() {
    // Can retry: CLIConnection, Timeout, Channel errors
    tokio::time::sleep(Duration::from_secs(1)).await;
    // ... retry operation
}
```

### Timeout Configuration

Configure timeouts to prevent indefinite hangs:

```rust
let options = ClaudeAgentOptions::new()
    .with_timeout_secs(60)  // 60 second timeout
    .with_max_turns(5);

// Or disable timeout (not recommended)
let options = ClaudeAgentOptions::new()
    .with_timeout_secs(0);  // No timeout
```

Default timeout is 300 seconds (5 minutes).

### Stream Error Handling

When consuming message streams, handle errors per-message:

```rust
while let Some(result) = stream.next().await {
    match result {
        Ok(Message::Assistant(msg)) => {
            println!("{}", msg.text());
        }
        Ok(Message::Result(result)) => {
            if result.is_error {
                eprintln!("Query failed: {:?}", result.result);
            }
        }
        Err(e) => {
            eprintln!("Stream error: {}", e);
            break;
        }
        _ => {}
    }
}
```

### Result Type

All SDK functions return `Result<T, ClaudeSDKError>`:

```rust
use claude_agents_sdk::Result;

async fn my_function() -> Result<String> {
    let mut client = ClaudeClient::new(None, None);
    client.connect().await?;  // Propagates errors with ?
    // ...
    Ok("success".to_string())
}
```

## Architecture

The SDK communicates with the Claude Code CLI via a subprocess:

```
┌─────────────────┐     stdin/stdout      ┌──────────────┐
│  Rust SDK       │ ◄──── JSON-RPC ────► │  Claude CLI  │
│  (your code)    │                       │  (subprocess)│
└─────────────────┘                       └──────────────┘
```

Key internal components:
- `Transport` - Abstract communication layer
- `SubprocessTransport` - Subprocess implementation
- `Query` - Control protocol handler
- `InternalClient` - Core query processing

## Comparison with Python SDK

| Feature | Python | Rust |
|---------|--------|------|
| One-shot query | `query()` | `query()` |
| Streaming client | `ClaudeSDKClient` | `ClaudeClient` |
| Tool callback | `can_use_tool` | `can_use_tool` |
| Hooks | `hooks` dict | `hooks` HashMap |
| Async | asyncio | tokio |
| Type safety | TypedDict | Enums + Structs |

## License

MIT License - see [LICENSE](LICENSE) for details.
