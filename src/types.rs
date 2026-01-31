//! Type definitions for the Claude Agents SDK.
//!
//! This module contains all the type definitions used throughout the SDK,
//! including messages, content blocks, options, hooks, and control protocol types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

// ============================================================================
// Permission Types
// ============================================================================

/// Permission modes controlling how the CLI handles tool permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default permission mode - asks for permission on sensitive operations.
    #[default]
    #[serde(rename = "default")]
    Default,
    /// Automatically accept all file edits.
    #[serde(rename = "acceptEdits")]
    AcceptEdits,
    /// Plan mode - only plans actions without executing.
    #[serde(rename = "plan")]
    Plan,
    /// Bypass all permission checks (dangerous).
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
}

/// Permission behavior for tool use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    /// Allow the tool to execute.
    Allow,
    /// Deny the tool execution.
    Deny,
    /// Ask for permission.
    Ask,
}

/// Destination for permission updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateDestination {
    /// Store in user settings.
    UserSettings,
    /// Store in project settings.
    ProjectSettings,
    /// Store in local settings.
    LocalSettings,
    /// Store in current session only.
    Session,
}

/// Permission rule value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRuleValue {
    /// The tool name this rule applies to.
    #[serde(rename = "toolName")]
    pub tool_name: String,
    /// Optional rule content.
    #[serde(rename = "ruleContent", skip_serializing_if = "Option::is_none")]
    pub rule_content: Option<String>,
}

/// Type of permission update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateType {
    /// Add new permission rules.
    AddRules,
    /// Replace existing permission rules.
    ReplaceRules,
    /// Remove permission rules.
    RemoveRules,
    /// Set the permission mode.
    SetMode,
    /// Add directories to permissions.
    AddDirectories,
    /// Remove directories from permissions.
    RemoveDirectories,
}

/// Permission update configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionUpdate {
    /// Type of update to perform.
    #[serde(rename = "type")]
    pub update_type: PermissionUpdateType,
    /// Rules to add/replace/remove.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<PermissionRuleValue>>,
    /// Behavior for the rules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<PermissionBehavior>,
    /// Mode to set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<PermissionMode>,
    /// Directories to add/remove.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directories: Option<Vec<String>>,
    /// Where to store the update.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<PermissionUpdateDestination>,
}

// ============================================================================
// Tool Permission Callback Types
// ============================================================================

/// Context information for tool permission callbacks.
#[derive(Debug, Clone, Default)]
pub struct ToolPermissionContext {
    /// Permission suggestions from CLI.
    pub suggestions: Vec<PermissionUpdate>,
}

/// Allow permission result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResultAllow {
    /// Always "allow".
    pub behavior: String,
    /// Updated input for the tool.
    #[serde(rename = "updatedInput", skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
    /// Permission updates to apply.
    #[serde(rename = "updatedPermissions", skip_serializing_if = "Option::is_none")]
    pub updated_permissions: Option<Vec<PermissionUpdate>>,
}

impl PermissionResultAllow {
    /// Create a new allow result.
    pub fn new() -> Self {
        Self {
            behavior: "allow".to_string(),
            updated_input: None,
            updated_permissions: None,
        }
    }

    /// Create an allow result with updated input.
    pub fn with_updated_input(input: serde_json::Value) -> Self {
        Self {
            behavior: "allow".to_string(),
            updated_input: Some(input),
            updated_permissions: None,
        }
    }
}

impl Default for PermissionResultAllow {
    fn default() -> Self {
        Self::new()
    }
}

/// Deny permission result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResultDeny {
    /// Always "deny".
    pub behavior: String,
    /// Message explaining why permission was denied.
    #[serde(default)]
    pub message: String,
    /// Whether to interrupt execution.
    #[serde(default)]
    pub interrupt: bool,
}

impl PermissionResultDeny {
    /// Create a new deny result.
    pub fn new() -> Self {
        Self {
            behavior: "deny".to_string(),
            message: String::new(),
            interrupt: false,
        }
    }

    /// Create a deny result with a message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            behavior: "deny".to_string(),
            message: message.into(),
            interrupt: false,
        }
    }

    /// Create a deny result that also interrupts.
    pub fn with_interrupt(message: impl Into<String>) -> Self {
        Self {
            behavior: "deny".to_string(),
            message: message.into(),
            interrupt: true,
        }
    }
}

impl Default for PermissionResultDeny {
    fn default() -> Self {
        Self::new()
    }
}

/// Permission result from a tool permission callback.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PermissionResult {
    /// Allow the tool to execute.
    Allow(PermissionResultAllow),
    /// Deny the tool execution.
    Deny(PermissionResultDeny),
}

impl PermissionResult {
    /// Create an allow result.
    pub fn allow() -> Self {
        Self::Allow(PermissionResultAllow::new())
    }

    /// Create a deny result.
    pub fn deny() -> Self {
        Self::Deny(PermissionResultDeny::new())
    }

    /// Create a deny result with a message.
    pub fn deny_with_message(message: impl Into<String>) -> Self {
        Self::Deny(PermissionResultDeny::with_message(message))
    }
}

/// The async future type returned by tool permission callbacks.
pub type CanUseToolFuture = Pin<Box<dyn Future<Output = PermissionResult> + Send>>;

/// Type alias for the tool permission callback function.
///
/// This callback is invoked when Claude wants to use a tool, allowing you to
/// approve, deny, or modify the tool execution.
///
/// # Arguments
/// * `tool_name` - Name of the tool being requested (e.g., "Bash", "Read", "Write")
/// * `input` - The tool input as JSON (tool-specific parameters)
/// * `context` - Additional context including permission suggestions
///
/// # Returns
/// A [`PermissionResult`] indicating whether to allow or deny the tool use.
///
/// # Example
/// ```ignore
/// let callback: CanUseTool = Arc::new(|tool_name, input, context| {
///     Box::pin(async move {
///         if tool_name == "Bash" {
///             PermissionResult::deny_with_message("Bash is disabled")
///         } else {
///             PermissionResult::allow()
///         }
///     })
/// });
/// ```
pub type CanUseTool =
    Arc<dyn Fn(String, serde_json::Value, ToolPermissionContext) -> CanUseToolFuture + Send + Sync>;

// ============================================================================
// Hook Types
// ============================================================================

/// Hook event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Before a tool is used.
    PreToolUse,
    /// After a tool is used.
    PostToolUse,
    /// After a tool use fails.
    PostToolUseFailure,
    /// When user submits a prompt.
    UserPromptSubmit,
    /// Stop hook.
    Stop,
    /// Subagent stop hook.
    SubagentStop,
    /// Before context compaction.
    PreCompact,
}

/// Base hook input fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseHookInput {
    /// Session ID.
    pub session_id: String,
    /// Path to transcript file.
    pub transcript_path: String,
    /// Current working directory.
    pub cwd: String,
    /// Current permission mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
}

/// Input for PreToolUse hook events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUseHookInput {
    /// Base fields.
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String,
    /// Name of the tool being used.
    pub tool_name: String,
    /// Input to the tool.
    pub tool_input: serde_json::Value,
}

/// Input for PostToolUse hook events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseHookInput {
    /// Base fields.
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String,
    /// Name of the tool that was used.
    pub tool_name: String,
    /// Input that was passed to the tool.
    pub tool_input: serde_json::Value,
    /// Response from the tool.
    pub tool_response: serde_json::Value,
}

/// Input for PostToolUseFailure hook events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseFailureHookInput {
    /// Base fields.
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String,
    /// Name of the tool that failed.
    pub tool_name: String,
    /// Input that was passed to the tool.
    pub tool_input: serde_json::Value,
    /// Tool use ID.
    pub tool_use_id: String,
    /// Error message.
    pub error: String,
    /// Whether the failure was due to an interrupt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_interrupt: Option<bool>,
}

/// Input for UserPromptSubmit hook events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitHookInput {
    /// Base fields.
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String,
    /// The submitted prompt.
    pub prompt: String,
}

/// Input for Stop hook events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopHookInput {
    /// Base fields.
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String,
    /// Whether stop hook is active.
    pub stop_hook_active: bool,
}

/// Input for SubagentStop hook events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopHookInput {
    /// Base fields.
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String,
    /// Whether stop hook is active.
    pub stop_hook_active: bool,
}

/// Trigger for PreCompact hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactTrigger {
    /// Manual compaction.
    Manual,
    /// Automatic compaction.
    Auto,
}

/// Input for PreCompact hook events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactHookInput {
    /// Base fields.
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String,
    /// What triggered the compaction.
    pub trigger: CompactTrigger,
    /// Custom instructions for compaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
}

/// Union of all hook input types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum HookInput {
    /// PreToolUse hook input.
    PreToolUse(PreToolUseHookInput),
    /// PostToolUse hook input.
    PostToolUse(PostToolUseHookInput),
    /// PostToolUseFailure hook input.
    PostToolUseFailure(PostToolUseFailureHookInput),
    /// UserPromptSubmit hook input.
    UserPromptSubmit(UserPromptSubmitHookInput),
    /// Stop hook input.
    Stop(StopHookInput),
    /// SubagentStop hook input.
    SubagentStop(SubagentStopHookInput),
    /// PreCompact hook input.
    PreCompact(PreCompactHookInput),
}

/// Hook-specific output for PreToolUse events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreToolUseHookSpecificOutput {
    /// Event name.
    #[serde(rename = "hookEventName")]
    pub hook_event_name: String,
    /// Permission decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<PermissionBehavior>,
    /// Reason for decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,
    /// Updated input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
}

/// Hook-specific output for PostToolUse events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostToolUseHookSpecificOutput {
    /// Event name.
    #[serde(rename = "hookEventName")]
    pub hook_event_name: String,
    /// Additional context to add.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Hook-specific output for PostToolUseFailure events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostToolUseFailureHookSpecificOutput {
    /// Event name.
    #[serde(rename = "hookEventName")]
    pub hook_event_name: String,
    /// Additional context to add.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Hook-specific output for UserPromptSubmit events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPromptSubmitHookSpecificOutput {
    /// Event name.
    #[serde(rename = "hookEventName")]
    pub hook_event_name: String,
    /// Additional context to add.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Union of hook-specific outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookSpecificOutput {
    /// PreToolUse specific output.
    PreToolUse(PreToolUseHookSpecificOutput),
    /// PostToolUse specific output.
    PostToolUse(PostToolUseHookSpecificOutput),
    /// PostToolUseFailure specific output.
    PostToolUseFailure(PostToolUseFailureHookSpecificOutput),
    /// UserPromptSubmit specific output.
    UserPromptSubmit(UserPromptSubmitHookSpecificOutput),
}

/// Async hook output that defers execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncHookOutput {
    /// Set to true to defer execution.
    #[serde(rename = "async")]
    pub async_: bool,
    /// Optional timeout in milliseconds.
    #[serde(rename = "asyncTimeout", skip_serializing_if = "Option::is_none")]
    pub async_timeout: Option<u64>,
}

/// Synchronous hook output with control fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncHookOutput {
    /// Whether to continue execution.
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_: Option<bool>,
    /// Whether to suppress output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    /// Reason for stopping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Decision (e.g., "block").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    /// System message to display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
    /// Reason for decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Hook-specific output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

/// Hook output type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookOutput {
    /// Async output.
    Async(AsyncHookOutput),
    /// Sync output.
    Sync(SyncHookOutput),
}

impl Default for HookOutput {
    fn default() -> Self {
        Self::Sync(SyncHookOutput::default())
    }
}

/// Context for hook callbacks.
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    // Reserved for future abort signal support.
}

/// The async future type returned by hook callbacks.
pub type HookCallbackFuture = Pin<Box<dyn Future<Output = HookOutput> + Send>>;

/// Type alias for hook callback functions.
///
/// Hooks are invoked at specific points during Claude's execution, allowing you
/// to observe or modify behavior.
///
/// # Arguments
/// * `input` - The hook input containing event-specific data
/// * `tool_use_id` - Optional tool use ID (for pre/post tool hooks)
/// * `context` - Hook context (reserved for future use)
///
/// # Returns
/// A [`HookOutput`] that can modify the tool input, block execution, or log a message.
///
/// # Example
/// ```ignore
/// let callback: HookCallback = Arc::new(|input, tool_use_id, context| {
///     Box::pin(async move {
///         println!("Tool called: {:?}", tool_use_id);
///         HookOutput::default()  // Continue without modification
///     })
/// });
/// ```
pub type HookCallback =
    Arc<dyn Fn(HookInput, Option<String>, HookContext) -> HookCallbackFuture + Send + Sync>;

/// Hook matcher configuration.
#[derive(Clone, Default)]
pub struct HookMatcher {
    /// Pattern to match (e.g., tool name or regex).
    pub matcher: Option<String>,
    /// List of hook callbacks.
    pub hooks: Vec<HookCallback>,
    /// Timeout in seconds.
    pub timeout: Option<f64>,
}

impl std::fmt::Debug for HookMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookMatcher")
            .field("matcher", &self.matcher)
            .field("hooks", &format!("[{} callbacks]", self.hooks.len()))
            .field("timeout", &self.timeout)
            .finish()
    }
}

// ============================================================================
// MCP Server Configuration
// ============================================================================

/// MCP stdio server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStdioServerConfig {
    /// Server type (always "stdio").
    #[serde(rename = "type", default = "default_stdio")]
    pub server_type: String,
    /// Command to run.
    pub command: String,
    /// Command arguments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Environment variables.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

fn default_stdio() -> String {
    "stdio".to_string()
}

/// MCP SSE server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSSEServerConfig {
    /// Server type (always "sse").
    #[serde(rename = "type")]
    pub server_type: String,
    /// Server URL.
    pub url: String,
    /// HTTP headers.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
}

/// MCP HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHttpServerConfig {
    /// Server type (always "http").
    #[serde(rename = "type")]
    pub server_type: String,
    /// Server URL.
    pub url: String,
    /// HTTP headers.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
}

/// MCP server configuration union.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpServerConfig {
    /// Stdio-based MCP server.
    #[serde(rename = "stdio")]
    Stdio(McpStdioServerConfig),
    /// SSE-based MCP server.
    #[serde(rename = "sse")]
    SSE(McpSSEServerConfig),
    /// HTTP-based MCP server.
    #[serde(rename = "http")]
    Http(McpHttpServerConfig),
}

/// SDK plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkPluginConfig {
    /// Plugin type.
    #[serde(rename = "type")]
    pub plugin_type: String,
    /// Path to the plugin.
    pub path: String,
}

// ============================================================================
// Sandbox Configuration
// ============================================================================

/// Network configuration for sandbox.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxNetworkConfig {
    /// Unix socket paths accessible in sandbox.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_unix_sockets: Vec<String>,
    /// Allow all Unix sockets.
    #[serde(default)]
    pub allow_all_unix_sockets: bool,
    /// Allow binding to localhost ports.
    #[serde(default)]
    pub allow_local_binding: bool,
    /// HTTP proxy port.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy_port: Option<u16>,
    /// SOCKS5 proxy port.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks_proxy_port: Option<u16>,
}

/// Violations to ignore in sandbox.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxIgnoreViolations {
    /// File paths to ignore.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file: Vec<String>,
    /// Network hosts to ignore.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub network: Vec<String>,
}

/// Sandbox settings configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxSettings {
    /// Enable bash sandboxing.
    #[serde(default)]
    pub enabled: bool,
    /// Auto-approve bash when sandboxed.
    #[serde(default = "default_true")]
    pub auto_allow_bash_if_sandboxed: bool,
    /// Commands to exclude from sandbox.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_commands: Vec<String>,
    /// Allow commands to bypass sandbox.
    #[serde(default = "default_true")]
    pub allow_unsandboxed_commands: bool,
    /// Network configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<SandboxNetworkConfig>,
    /// Violations to ignore.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_violations: Option<SandboxIgnoreViolations>,
    /// Enable weaker nested sandbox.
    #[serde(default)]
    pub enable_weaker_nested_sandbox: bool,
}

fn default_true() -> bool {
    true
}

// ============================================================================
// Content Block Types
// ============================================================================

/// Text content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    /// The text content.
    pub text: String,
}

/// Thinking content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBlock {
    /// The thinking content.
    pub thinking: String,
    /// Signature for verification.
    pub signature: String,
}

/// Tool use content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseBlock {
    /// Tool use ID.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Tool input.
    pub input: serde_json::Value,
}

/// Tool result content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultBlock {
    /// ID of the tool use this is a result for.
    pub tool_use_id: String,
    /// Result content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    /// Whether this is an error result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Content block union type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Text block.
    #[serde(rename = "text")]
    Text(TextBlock),
    /// Thinking block.
    #[serde(rename = "thinking")]
    Thinking(ThinkingBlock),
    /// Tool use block.
    #[serde(rename = "tool_use")]
    ToolUse(ToolUseBlock),
    /// Tool result block.
    #[serde(rename = "tool_result")]
    ToolResult(ToolResultBlock),
}

impl ContentBlock {
    /// Get the text if this is a text block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ContentBlock::Text(block) => Some(&block.text),
            _ => None,
        }
    }

    /// Check if this is a tool use block.
    pub fn is_tool_use(&self) -> bool {
        matches!(self, ContentBlock::ToolUse(_))
    }
}

// ============================================================================
// Message Types
// ============================================================================

/// Assistant message error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantMessageError {
    /// Authentication failed.
    AuthenticationFailed,
    /// Billing error.
    BillingError,
    /// Rate limit exceeded.
    RateLimit,
    /// Invalid request.
    InvalidRequest,
    /// Server error.
    ServerError,
    /// Unknown error.
    Unknown,
}

/// User message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    /// Message content (string or content blocks).
    pub content: UserMessageContent,
    /// Unique identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// Parent tool use ID if this is a tool result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
}

/// User message content can be a string or content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserMessageContent {
    /// Plain text content.
    Text(String),
    /// Content blocks.
    Blocks(Vec<ContentBlock>),
}

impl UserMessage {
    /// Get text content if this is a simple text message.
    pub fn text(&self) -> Option<&str> {
        match &self.content {
            UserMessageContent::Text(s) => Some(s),
            UserMessageContent::Blocks(blocks) => {
                if blocks.len() == 1 {
                    blocks[0].as_text()
                } else {
                    None
                }
            }
        }
    }
}

/// Assistant message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// Content blocks.
    pub content: Vec<ContentBlock>,
    /// Model that generated this message.
    pub model: String,
    /// Parent tool use ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
    /// Error if the message failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AssistantMessageError>,
}

impl AssistantMessage {
    /// Get all text content from this message.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| block.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get all tool use blocks.
    pub fn tool_uses(&self) -> Vec<&ToolUseBlock> {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse(tu) => Some(tu),
                _ => None,
            })
            .collect()
    }
}

/// System message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    /// Message subtype.
    pub subtype: String,
    /// Message data.
    pub data: serde_json::Value,
}

/// Result message with cost and usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMessage {
    /// Message subtype.
    pub subtype: String,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// API duration in milliseconds.
    pub duration_api_ms: u64,
    /// Whether the result is an error.
    pub is_error: bool,
    /// Number of turns in the conversation.
    pub num_turns: u32,
    /// Session ID.
    pub session_id: String,
    /// Total cost in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<serde_json::Value>,
    /// Result text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Structured output if output_format was specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<serde_json::Value>,
}

/// Stream event for partial message updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Event UUID.
    pub uuid: String,
    /// Session ID.
    pub session_id: String,
    /// Raw Anthropic API stream event.
    pub event: serde_json::Value,
    /// Parent tool use ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
}

/// Message union type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    /// User message.
    #[serde(rename = "user")]
    User(UserMessage),
    /// Assistant message.
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    /// System message.
    #[serde(rename = "system")]
    System(SystemMessage),
    /// Result message.
    #[serde(rename = "result")]
    Result(ResultMessage),
    /// Stream event.
    #[serde(rename = "stream_event")]
    StreamEvent(StreamEvent),
}

impl Message {
    /// Check if this is a result message.
    pub fn is_result(&self) -> bool {
        matches!(self, Message::Result(_))
    }

    /// Check if this is an assistant message.
    pub fn is_assistant(&self) -> bool {
        matches!(self, Message::Assistant(_))
    }

    /// Get as assistant message if applicable.
    pub fn as_assistant(&self) -> Option<&AssistantMessage> {
        match self {
            Message::Assistant(msg) => Some(msg),
            _ => None,
        }
    }

    /// Get as result message if applicable.
    pub fn as_result(&self) -> Option<&ResultMessage> {
        match self {
            Message::Result(msg) => Some(msg),
            _ => None,
        }
    }
}

// ============================================================================
// Agent Configuration
// ============================================================================

/// System prompt preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPromptPreset {
    /// Type (always "preset").
    #[serde(rename = "type")]
    pub preset_type: String,
    /// Preset name.
    pub preset: String,
    /// Text to append.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<String>,
}

/// Tools preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsPreset {
    /// Type (always "preset").
    #[serde(rename = "type")]
    pub preset_type: String,
    /// Preset name.
    pub preset: String,
}

/// System prompt configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPromptConfig {
    /// Plain text system prompt.
    Text(String),
    /// Preset configuration.
    Preset(SystemPromptPreset),
}

/// Tools configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolsConfig {
    /// List of tool names.
    List(Vec<String>),
    /// Preset configuration.
    Preset(ToolsPreset),
}

/// Agent model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentModel {
    /// Sonnet model.
    Sonnet,
    /// Opus model.
    Opus,
    /// Haiku model.
    Haiku,
    /// Inherit from parent.
    Inherit,
}

/// Agent definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent description.
    pub description: String,
    /// Agent prompt.
    pub prompt: String,
    /// Allowed tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<AgentModel>,
}

/// Setting source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingSource {
    /// User settings.
    User,
    /// Project settings.
    Project,
    /// Local settings.
    Local,
}

/// SDK Beta features.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SdkBeta {
    /// Extended context beta.
    #[serde(rename = "context-1m-2025-08-07")]
    Context1m,
}

/// MCP servers configuration.
#[derive(Debug, Clone)]
pub enum McpServersConfig {
    /// Map of server configurations.
    Map(HashMap<String, McpServerConfig>),
    /// Path to configuration file.
    Path(PathBuf),
}

impl Default for McpServersConfig {
    fn default() -> Self {
        Self::Map(HashMap::new())
    }
}

/// Query options for Claude SDK.
#[derive(Clone, Default)]
pub struct ClaudeAgentOptions {
    /// Tools to use.
    pub tools: Option<ToolsConfig>,
    /// Allowed tools.
    pub allowed_tools: Vec<String>,
    /// System prompt.
    pub system_prompt: Option<SystemPromptConfig>,
    /// MCP server configurations.
    pub mcp_servers: McpServersConfig,
    /// Permission mode.
    pub permission_mode: Option<PermissionMode>,
    /// Continue previous conversation.
    pub continue_conversation: bool,
    /// Resume session ID.
    pub resume: Option<String>,
    /// Maximum turns.
    pub max_turns: Option<u32>,
    /// Maximum budget in USD.
    pub max_budget_usd: Option<f64>,
    /// Disallowed tools.
    pub disallowed_tools: Vec<String>,
    /// Model to use.
    pub model: Option<String>,
    /// Fallback model.
    pub fallback_model: Option<String>,
    /// Beta features.
    pub betas: Vec<SdkBeta>,
    /// Permission prompt tool name.
    pub permission_prompt_tool_name: Option<String>,
    /// Working directory.
    pub cwd: Option<PathBuf>,
    /// Path to CLI executable.
    pub cli_path: Option<PathBuf>,
    /// Settings string.
    pub settings: Option<String>,
    /// Additional directories.
    pub add_dirs: Vec<PathBuf>,
    /// Environment variables.
    pub env: HashMap<String, String>,
    /// Extra CLI arguments.
    pub extra_args: HashMap<String, Option<String>>,
    /// Maximum buffer size for stdout.
    pub max_buffer_size: Option<usize>,
    /// Callback for stderr output.
    pub stderr: Option<Arc<dyn Fn(String) + Send + Sync>>,
    /// Tool permission callback.
    pub can_use_tool: Option<CanUseTool>,
    /// Hook configurations.
    pub hooks: Option<HashMap<HookEvent, Vec<HookMatcher>>>,
    /// User identifier.
    pub user: Option<String>,
    /// Include partial messages in stream.
    pub include_partial_messages: bool,
    /// Fork session when resuming.
    pub fork_session: bool,
    /// Agent definitions.
    pub agents: Option<HashMap<String, AgentDefinition>>,
    /// Setting sources.
    pub setting_sources: Option<Vec<SettingSource>>,
    /// Sandbox settings.
    pub sandbox: Option<SandboxSettings>,
    /// Plugin configurations.
    pub plugins: Vec<SdkPluginConfig>,
    /// Maximum thinking tokens.
    pub max_thinking_tokens: Option<u32>,
    /// Output format for structured outputs.
    pub output_format: Option<serde_json::Value>,
    /// Enable file checkpointing.
    pub enable_file_checkpointing: bool,
    /// Timeout in seconds for CLI operations (default: 300 = 5 minutes).
    /// Set to 0 to disable timeout.
    pub timeout_secs: Option<u64>,
}

impl std::fmt::Debug for ClaudeAgentOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeAgentOptions")
            .field("tools", &self.tools)
            .field("allowed_tools", &self.allowed_tools)
            .field("system_prompt", &self.system_prompt)
            .field("permission_mode", &self.permission_mode)
            .field("continue_conversation", &self.continue_conversation)
            .field("resume", &self.resume)
            .field("max_turns", &self.max_turns)
            .field("max_budget_usd", &self.max_budget_usd)
            .field("disallowed_tools", &self.disallowed_tools)
            .field("model", &self.model)
            .field(
                "can_use_tool",
                &self.can_use_tool.as_ref().map(|_| "<callback>"),
            )
            .field(
                "hooks",
                &self.hooks.as_ref().map(|h| format!("{} events", h.len())),
            )
            .field("stderr", &self.stderr.as_ref().map(|_| "<callback>"))
            .finish_non_exhaustive()
    }
}

impl ClaudeAgentOptions {
    /// Create new options with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(SystemPromptConfig::Text(prompt.into()));
        self
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the permission mode.
    pub fn with_permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = Some(mode);
        self
    }

    /// Set max turns.
    pub fn with_max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    /// Set working directory.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set allowed tools.
    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Enable partial message streaming.
    pub fn with_partial_messages(mut self) -> Self {
        self.include_partial_messages = true;
        self
    }

    /// Set the timeout for CLI operations in seconds.
    ///
    /// Default is 300 seconds (5 minutes). Set to 0 to disable timeout.
    /// This timeout applies to:
    /// - Initial CLI process startup
    /// - Control protocol requests (initialize, can_use_tool, etc.)
    /// - Waiting for CLI responses
    pub fn with_timeout_secs(mut self, timeout: u64) -> Self {
        self.timeout_secs = Some(timeout);
        self
    }

    /// Set the can_use_tool callback.
    pub fn with_can_use_tool<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, serde_json::Value, ToolPermissionContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = PermissionResult> + Send + 'static,
    {
        self.can_use_tool = Some(Arc::new(move |name, input, ctx| {
            Box::pin(callback(name, input, ctx))
        }));
        self
    }
}

// ============================================================================
// Control Protocol Types
// ============================================================================

/// Control request subtypes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype")]
pub enum ControlRequestPayload {
    /// Interrupt request.
    #[serde(rename = "interrupt")]
    Interrupt,
    /// Tool permission request.
    #[serde(rename = "can_use_tool")]
    CanUseTool {
        /// Tool name.
        tool_name: String,
        /// Tool input.
        input: serde_json::Value,
        /// Permission suggestions.
        permission_suggestions: Option<Vec<serde_json::Value>>,
        /// Blocked path.
        blocked_path: Option<String>,
    },
    /// Initialize request.
    #[serde(rename = "initialize")]
    Initialize {
        /// Hook configurations.
        hooks: Option<serde_json::Value>,
    },
    /// Set permission mode request.
    #[serde(rename = "set_permission_mode")]
    SetPermissionMode {
        /// New mode.
        mode: String,
    },
    /// Set model request.
    #[serde(rename = "set_model")]
    SetModel {
        /// New model.
        model: String,
    },
    /// Hook callback request.
    #[serde(rename = "hook_callback")]
    HookCallback {
        /// Callback ID.
        callback_id: String,
        /// Hook input.
        input: serde_json::Value,
        /// Tool use ID.
        tool_use_id: Option<String>,
    },
    /// MCP message request.
    #[serde(rename = "mcp_message")]
    McpMessage {
        /// Server name.
        server_name: String,
        /// JSONRPC message.
        message: serde_json::Value,
    },
    /// MCP status request.
    #[serde(rename = "mcp_status")]
    McpStatus,
    /// Rewind files request.
    #[serde(rename = "rewind_files")]
    RewindFiles {
        /// User message ID to rewind to.
        user_message_id: String,
    },
}

/// Control request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequest {
    /// Always "control_request".
    #[serde(rename = "type")]
    pub request_type: String,
    /// Request ID.
    pub request_id: String,
    /// Request payload.
    pub request: ControlRequestPayload,
}

/// Success response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSuccessResponse {
    /// Always "success".
    pub subtype: String,
    /// Request ID.
    pub request_id: String,
    /// Response data.
    pub response: Option<serde_json::Value>,
}

/// Error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlErrorResponse {
    /// Always "error".
    pub subtype: String,
    /// Request ID.
    pub request_id: String,
    /// Error message.
    pub error: String,
}

/// Control response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype")]
pub enum ControlResponsePayload {
    /// Success response.
    #[serde(rename = "success")]
    Success {
        /// Request ID.
        request_id: String,
        /// Response data.
        response: Option<serde_json::Value>,
    },
    /// Error response.
    #[serde(rename = "error")]
    Error {
        /// Request ID.
        request_id: String,
        /// Error message.
        error: String,
    },
}

/// Control response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponse {
    /// Always "control_response".
    #[serde(rename = "type")]
    pub response_type: String,
    /// Response payload.
    pub response: ControlResponsePayload,
}

impl ControlResponse {
    /// Get the request ID.
    pub fn request_id(&self) -> &str {
        match &self.response {
            ControlResponsePayload::Success { request_id, .. } => request_id,
            ControlResponsePayload::Error { request_id, .. } => request_id,
        }
    }

    /// Check if this is a success response.
    pub fn is_success(&self) -> bool {
        matches!(&self.response, ControlResponsePayload::Success { .. })
    }

    /// Get the response data if successful.
    pub fn data(&self) -> Option<&serde_json::Value> {
        match &self.response {
            ControlResponsePayload::Success { response, .. } => response.as_ref(),
            ControlResponsePayload::Error { .. } => None,
        }
    }

    /// Get the error message if failed.
    pub fn error(&self) -> Option<&str> {
        match &self.response {
            ControlResponsePayload::Success { .. } => None,
            ControlResponsePayload::Error { error, .. } => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_result_allow() {
        let result = PermissionResult::allow();
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("allow"));
    }

    #[test]
    fn test_message_parsing() {
        let json = r#"{"type": "assistant", "content": [{"type": "text", "text": "Hello"}], "model": "claude-3"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert!(msg.is_assistant());
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::Text(TextBlock {
            text: "Hello".to_string(),
        });
        assert_eq!(block.as_text(), Some("Hello"));
    }

    #[test]
    fn test_options_builder() {
        let opts = ClaudeAgentOptions::new()
            .with_model("claude-3-sonnet")
            .with_max_turns(5)
            .with_permission_mode(PermissionMode::AcceptEdits);

        assert_eq!(opts.model, Some("claude-3-sonnet".to_string()));
        assert_eq!(opts.max_turns, Some(5));
        assert_eq!(opts.permission_mode, Some(PermissionMode::AcceptEdits));
    }
}
