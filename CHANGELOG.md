# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2025-01-02

### Fixed

- Removed unnecessary `unsafe impl Send for QueryStream` - the type is automatically Send
- Changed `into_stream()` to return `Result` instead of panicking with `.expect()`
- Fixed MCP calculator example to use correct `ToolInputSchema` and `ToolResult` APIs

## [0.1.0] - 2025-01-02

### Added

- Initial release of the Claude Agents SDK for Rust
- `query()` function for one-shot queries returning async streams
- `ClaudeClient` for bidirectional streaming communication
- Full type definitions for Claude Code CLI messages
- Tool permission callbacks (`CanUseTool`)
- Hook system for pre/post tool use events
- MCP server configuration support
- Comprehensive error types with `thiserror`
- Builder pattern for `ClaudeAgentOptions`
- Support for all Claude CLI options (model, system prompt, permissions, etc.)

[Unreleased]: https://github.com/jimmystridh/claude-agents-sdk/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/jimmystridh/claude-agents-sdk/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/jimmystridh/claude-agents-sdk/releases/tag/v0.1.0
