# Development Plan

## Completed

- [x] Phase 1-3: Core integration tests (test_core, test_client, test_errors)
- [x] Phase 4: Concurrent session testing
- [x] Phase 5: Edge case testing
- [x] Phase 6: Hook integration tests
- [x] Phase 7: Budget/cost verification
- [x] Phase 8: Context & session management
- [x] Phase 10: Docker infrastructure & CI

## In Progress

### 1. Fix Compiler Warnings
Clean up unused imports and functions in test helpers.

## Backlog

### 2. Run Integration Tests
Verify all tests pass against the real Claude API.

### 3. Property-Based Testing
Add `proptest` for fuzzing inputs - message parsing, option validation, edge cases.

### 4. Add More Examples
Expand `examples/` with real-world use cases:
- Error handling patterns
- Streaming with progress
- Tool callbacks
- Hook usage

### 5. Improve Documentation
- Add more rustdoc examples
- Expand README with architecture diagram
- API reference improvements

### 6. MCP Integration Tests
Tests for the MCP feature flag functionality.

### 7. Changelog Update
Document new test infrastructure in CHANGELOG.md.

### 8. Version Bump
Prepare for next release when ready.
