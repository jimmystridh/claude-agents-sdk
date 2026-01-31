//! Query control protocol handler.
//!
//! This module implements the bidirectional control protocol for communicating
//! with the Claude CLI. It handles:
//! - Message routing (regular messages vs control requests/responses)
//! - Tool permission callbacks
//! - Hook callback invocation
//! - MCP server message routing
//! - Control request/response lifecycle

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tracing::{debug, error, trace, warn};
use uuid::Uuid;

use super::message_parser::{
    is_control_request, is_control_response, parse_control_request, parse_control_response,
    parse_message,
};
use super::transport::{SubprocessTransport, Transport};
use crate::errors::{ClaudeSDKError, Result};
use crate::types::*;

/// Counter for generating unique request IDs.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique request ID.
fn generate_request_id() -> String {
    let count = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let uuid = Uuid::new_v4();
    let uuid_str = uuid.to_string();
    format!("req_{}_{}", count, &uuid_str[..8])
}

/// Pending control request waiting for response.
struct PendingRequest {
    sender: oneshot::Sender<Result<serde_json::Value>>,
}

/// Default timeout for CLI operations in seconds (5 minutes).
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Query handler for the control protocol.
///
/// This type manages the bidirectional control protocol with the CLI,
/// routing messages to appropriate handlers and managing the lifecycle
/// of control requests.
pub struct Query {
    /// Transport for CLI communication.
    transport: Arc<Mutex<SubprocessTransport>>,
    /// Channel for sending messages to the user (taken when start() is called).
    message_tx: Option<mpsc::Sender<Result<Message>>>,
    /// Pending control requests awaiting responses.
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
    /// Tool permission callback.
    can_use_tool: Option<CanUseTool>,
    /// Hook configurations.
    hooks: Option<HashMap<HookEvent, Vec<HookMatcher>>>,
    /// Hook callback registry (callback_id -> callback function).
    hook_callbacks: Arc<RwLock<HashMap<String, HookCallback>>>,
    /// Whether the query has been started.
    started: bool,
    /// Background task handle.
    reader_task: Option<tokio::task::JoinHandle<()>>,
    /// Shutdown signal sender.
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Server initialization result (stored after initialize()).
    initialization_result: Arc<RwLock<Option<serde_json::Value>>>,
    /// Timeout for CLI operations in seconds (0 = no timeout).
    timeout_secs: u64,
}

impl Query {
    /// Create a new Query handler.
    pub fn new(
        transport: SubprocessTransport,
        options: &ClaudeAgentOptions,
    ) -> (Self, mpsc::Receiver<Result<Message>>) {
        let (message_tx, message_rx) = mpsc::channel(256);

        let query = Self {
            transport: Arc::new(Mutex::new(transport)),
            message_tx: Some(message_tx),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            can_use_tool: options.can_use_tool.clone(),
            hooks: options.hooks.clone(),
            hook_callbacks: Arc::new(RwLock::new(HashMap::new())),
            started: false,
            reader_task: None,
            shutdown_tx: None,
            initialization_result: Arc::new(RwLock::new(None)),
            timeout_secs: options.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS),
        };

        (query, message_rx)
    }

    /// Start the query handler.
    ///
    /// This spawns a background task that reads messages from the transport
    /// and routes them appropriately.
    pub async fn start(&mut self) -> Result<()> {
        if self.started {
            return Ok(());
        }

        // Get the stdout receiver from transport
        let stdout_rx = {
            let mut transport = self.transport.lock().await;
            transport.take_stdout_rx().ok_or_else(|| {
                ClaudeSDKError::internal("Transport stdout receiver already taken")
            })?
        };

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Clone references for the background task
        let transport = Arc::clone(&self.transport);
        // Take ownership of message_tx so that when the reader task finishes,
        // the channel closes and the consumer stream ends
        let message_tx = self.message_tx.take().ok_or_else(|| {
            ClaudeSDKError::internal("Query already started (message_tx already taken)")
        })?;
        let pending_requests = Arc::clone(&self.pending_requests);
        let can_use_tool = self.can_use_tool.clone();
        let hook_callbacks = Arc::clone(&self.hook_callbacks);

        // Spawn background reader task
        let reader_task = tokio::spawn(async move {
            Self::read_messages(
                stdout_rx,
                transport,
                message_tx,
                pending_requests,
                can_use_tool,
                hook_callbacks,
                &mut shutdown_rx,
            )
            .await;
        });

        self.reader_task = Some(reader_task);
        self.started = true;

        debug!("Query handler started");
        Ok(())
    }

    /// Background task that reads and routes messages.
    async fn read_messages(
        mut stdout_rx: mpsc::Receiver<Result<serde_json::Value>>,
        transport: Arc<Mutex<SubprocessTransport>>,
        message_tx: mpsc::Sender<Result<Message>>,
        pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
        can_use_tool: Option<CanUseTool>,
        hook_callbacks: Arc<RwLock<HashMap<String, HookCallback>>>,
        shutdown_rx: &mut mpsc::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                biased;

                _ = shutdown_rx.recv() => {
                    debug!("Query reader received shutdown signal");
                    break;
                }

                msg = stdout_rx.recv() => {
                    match msg {
                        Some(Ok(raw)) => {
                            let msg_type = raw.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                            trace!("Query received raw message of type: {}", msg_type);

                            // Route by message type
                            if is_control_response(&raw) {
                                debug!("Routing control response");
                                Self::handle_control_response(raw, &pending_requests).await;
                            } else if is_control_request(&raw) {
                                debug!("Routing control request");
                                Self::handle_control_request(
                                    raw,
                                    &transport,
                                    &can_use_tool,
                                    &hook_callbacks,
                                ).await;
                            } else {
                                // Regular message
                                debug!("Routing regular message of type: {}", msg_type);
                                match parse_message(raw) {
                                    Ok(msg) => {
                                        if message_tx.send(Ok(msg)).await.is_err() {
                                            debug!("Message receiver dropped");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse message: {}", e);
                                        if message_tx.send(Err(e)).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("Error reading from transport: {}", e);
                            let _ = message_tx.send(Err(e)).await;
                            break;
                        }
                        None => {
                            debug!("Transport stream ended");
                            break;
                        }
                    }
                }
            }
        }

        debug!("Query reader task finished");
    }

    /// Handle a control response from the CLI.
    async fn handle_control_response(
        raw: serde_json::Value,
        pending_requests: &RwLock<HashMap<String, PendingRequest>>,
    ) {
        match parse_control_response(raw) {
            Ok(response) => {
                let request_id = response.request_id().to_string();
                let mut pending = pending_requests.write().await;

                if let Some(request) = pending.remove(&request_id) {
                    let result = if response.is_success() {
                        Ok(response.data().cloned().unwrap_or(serde_json::Value::Null))
                    } else {
                        Err(ClaudeSDKError::control_protocol_with_id(
                            response.error().unwrap_or("Unknown error"),
                            request_id,
                        ))
                    };

                    let _ = request.sender.send(result);
                } else {
                    warn!("Received response for unknown request: {}", request_id);
                }
            }
            Err(e) => {
                error!("Failed to parse control response: {}", e);
            }
        }
    }

    /// Handle a control request from the CLI.
    async fn handle_control_request(
        raw: serde_json::Value,
        transport: &Arc<Mutex<SubprocessTransport>>,
        can_use_tool: &Option<CanUseTool>,
        hook_callbacks: &RwLock<HashMap<String, HookCallback>>,
    ) {
        let request = match parse_control_request(raw.clone()) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to parse control request: {}", e);
                return;
            }
        };

        let request_id = request.request_id.clone();
        let response = Self::process_control_request(request, can_use_tool, hook_callbacks).await;

        // Send response back to CLI
        let response_msg = match response {
            Ok(data) => serde_json::json!({
                "type": "control_response",
                "response": {
                    "subtype": "success",
                    "request_id": request_id,
                    "response": data
                }
            }),
            Err(e) => serde_json::json!({
                "type": "control_response",
                "response": {
                    "subtype": "error",
                    "request_id": request_id,
                    "error": e.to_string()
                }
            }),
        };

        let transport = transport.lock().await;
        if let Err(e) = transport.write(&response_msg.to_string()).await {
            error!("Failed to send control response: {}", e);
        }
    }

    /// Process a control request and return the response.
    async fn process_control_request(
        request: ControlRequest,
        can_use_tool: &Option<CanUseTool>,
        hook_callbacks: &RwLock<HashMap<String, HookCallback>>,
    ) -> Result<serde_json::Value> {
        match request.request {
            ControlRequestPayload::CanUseTool {
                tool_name,
                input,
                permission_suggestions,
                ..
            } => {
                if let Some(callback) = can_use_tool {
                    let context = ToolPermissionContext {
                        suggestions: permission_suggestions
                            .map(|s| {
                                s.into_iter()
                                    .filter_map(|v| serde_json::from_value(v).ok())
                                    .collect()
                            })
                            .unwrap_or_default(),
                    };

                    let result = callback(tool_name, input, context).await;
                    serde_json::to_value(result).map_err(|e| {
                        ClaudeSDKError::internal(format!(
                            "Failed to serialize PermissionResult: {}",
                            e
                        ))
                    })
                } else {
                    // No callback - default to allow
                    Ok(serde_json::json!({"behavior": "allow"}))
                }
            }

            ControlRequestPayload::HookCallback {
                callback_id,
                input,
                tool_use_id,
            } => {
                let callbacks = hook_callbacks.read().await;
                if let Some(callback) = callbacks.get(&callback_id) {
                    // Parse the hook input
                    let hook_input: HookInput = serde_json::from_value(input).map_err(|e| {
                        ClaudeSDKError::message_parse(format!("Failed to parse hook input: {}", e))
                    })?;

                    let context = HookContext::default();
                    let output = callback(hook_input, tool_use_id, context).await;

                    // Convert output for CLI (handle field renaming)
                    let mut output_value = serde_json::to_value(&output).map_err(|e| {
                        ClaudeSDKError::internal(format!("Failed to serialize HookOutput: {}", e))
                    })?;

                    // Rename async_ to async and continue_ to continue
                    if let serde_json::Value::Object(ref mut map) = output_value {
                        if let Some(v) = map.remove("async_") {
                            map.insert("async".to_string(), v);
                        }
                        if let Some(v) = map.remove("continue_") {
                            map.insert("continue".to_string(), v);
                        }
                    }

                    Ok(output_value)
                } else {
                    warn!("Unknown hook callback ID: {}", callback_id);
                    Ok(serde_json::json!({}))
                }
            }

            ControlRequestPayload::Initialize { .. } => {
                // CLI is initializing - acknowledge
                debug!("Received initialize request from CLI");
                Ok(serde_json::json!({"initialized": true}))
            }

            ControlRequestPayload::McpMessage {
                server_name,
                message: _,
            } => {
                // MCP message routing - this would be implemented with full MCP support
                debug!("Received MCP message for server: {}", server_name);
                Err(ClaudeSDKError::internal(format!(
                    "MCP server '{}' not found (SDK MCP not yet implemented)",
                    server_name
                )))
            }

            _ => {
                warn!("Unhandled control request type");
                Ok(serde_json::Value::Null)
            }
        }
    }

    /// Send a control request to the CLI and wait for response.
    pub async fn send_control_request(
        &self,
        payload: ControlRequestPayload,
    ) -> Result<serde_json::Value> {
        let request_id = generate_request_id();

        let request = serde_json::json!({
            "type": "control_request",
            "request_id": request_id,
            "request": payload
        });

        // Register pending request
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), PendingRequest { sender: tx });
        }

        // Send request
        {
            let transport = self.transport.lock().await;
            transport.write(&request.to_string()).await?;
        }

        // Wait for response with timeout (use configured timeout, or no timeout if 0)
        let timeout_duration = if self.timeout_secs == 0 {
            // Use a very long timeout (effectively no timeout)
            std::time::Duration::from_secs(86400 * 365) // 1 year
        } else {
            std::time::Duration::from_secs(self.timeout_secs)
        };

        match tokio::time::timeout(timeout_duration, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(ClaudeSDKError::internal("Control request receiver dropped")),
            Err(_) => {
                // Remove from pending
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err(ClaudeSDKError::timeout(self.timeout_secs * 1000))
            }
        }
    }

    /// Initialize the streaming session with the CLI.
    pub async fn initialize(&self) -> Result<serde_json::Value> {
        // Build hooks configuration for initialization
        let hooks_config = self.build_hooks_config().await;

        let result = self
            .send_control_request(ControlRequestPayload::Initialize {
                hooks: hooks_config,
            })
            .await?;

        // Store the initialization result for later retrieval
        {
            let mut init_result = self.initialization_result.write().await;
            *init_result = Some(result.clone());
        }

        Ok(result)
    }

    /// Get the server initialization info.
    ///
    /// Returns the initialization response from the CLI, which includes
    /// available commands, output styles, and server capabilities.
    pub async fn get_server_info(&self) -> Option<serde_json::Value> {
        let init_result = self.initialization_result.read().await;
        init_result.clone()
    }

    /// Build hooks configuration for the initialize request.
    async fn build_hooks_config(&self) -> Option<serde_json::Value> {
        let hooks = self.hooks.as_ref()?;
        let mut config = serde_json::Map::new();

        for (event, matchers) in hooks {
            let mut event_config = Vec::new();

            for (matcher_idx, matcher) in matchers.iter().enumerate() {
                let mut matcher_config = serde_json::Map::new();

                if let Some(ref pattern) = matcher.matcher {
                    matcher_config.insert("matcher".to_string(), serde_json::json!(pattern));
                }

                if let Some(timeout) = matcher.timeout {
                    matcher_config.insert("timeout".to_string(), serde_json::json!(timeout));
                }

                // Register callbacks with unique IDs across all matchers for this event
                let mut callback_ids = Vec::new();
                for (callback_idx, callback) in matcher.hooks.iter().enumerate() {
                    // Include matcher index to ensure uniqueness across matchers
                    let callback_id = format!(
                        "{}_{}_{}",
                        serde_json::to_string(event).unwrap_or_default(),
                        matcher_idx,
                        callback_idx
                    );
                    callback_ids.push(callback_id.clone());

                    let mut callbacks = self.hook_callbacks.write().await;
                    callbacks.insert(callback_id, callback.clone());
                }

                if !callback_ids.is_empty() {
                    matcher_config
                        .insert("callbackIds".to_string(), serde_json::json!(callback_ids));
                }

                event_config.push(serde_json::Value::Object(matcher_config));
            }

            let event_name = match event {
                HookEvent::PreToolUse => "PreToolUse",
                HookEvent::PostToolUse => "PostToolUse",
                HookEvent::PostToolUseFailure => "PostToolUseFailure",
                HookEvent::UserPromptSubmit => "UserPromptSubmit",
                HookEvent::Stop => "Stop",
                HookEvent::SubagentStop => "SubagentStop",
                HookEvent::PreCompact => "PreCompact",
            };

            config.insert(
                event_name.to_string(),
                serde_json::Value::Array(event_config),
            );
        }

        Some(serde_json::Value::Object(config))
    }

    /// Send an interrupt request.
    pub async fn interrupt(&self) -> Result<()> {
        self.send_control_request(ControlRequestPayload::Interrupt)
            .await?;
        Ok(())
    }

    /// Set the permission mode.
    pub async fn set_permission_mode(&self, mode: PermissionMode) -> Result<()> {
        let mode_str = match mode {
            PermissionMode::Default => "default",
            PermissionMode::AcceptEdits => "acceptEdits",
            PermissionMode::Plan => "plan",
            PermissionMode::BypassPermissions => "bypassPermissions",
        };

        self.send_control_request(ControlRequestPayload::SetPermissionMode {
            mode: mode_str.to_string(),
        })
        .await?;
        Ok(())
    }

    /// Set the model.
    pub async fn set_model(&self, model: impl Into<String>) -> Result<()> {
        self.send_control_request(ControlRequestPayload::SetModel {
            model: model.into(),
        })
        .await?;
        Ok(())
    }

    /// Rewind files to a specific user message.
    pub async fn rewind_files(&self, user_message_id: impl Into<String>) -> Result<()> {
        self.send_control_request(ControlRequestPayload::RewindFiles {
            user_message_id: user_message_id.into(),
        })
        .await?;
        Ok(())
    }

    /// Get current MCP server connection status.
    ///
    /// Returns a JSON object (typically containing a `mcpServers` array) with status
    /// for all configured MCP servers.
    pub async fn get_mcp_status(&self) -> Result<serde_json::Value> {
        self.send_control_request(ControlRequestPayload::McpStatus)
            .await
    }

    /// Send a user message to the CLI.
    pub async fn send_message(&self, message: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": message
            },
            "parent_tool_use_id": serde_json::Value::Null,
            "session_id": "default"
        });

        let transport = self.transport.lock().await;
        transport.write(&msg.to_string()).await
    }

    /// Stop the query handler.
    pub async fn stop(&mut self) -> Result<()> {
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Wait for reader task
        if let Some(task) = self.reader_task.take() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), task).await;
        }

        // Close transport
        let mut transport = self.transport.lock().await;
        transport.close().await?;

        self.started = false;
        Ok(())
    }

    /// Check if the query is running.
    pub fn is_started(&self) -> bool {
        self.started
    }
}

impl Drop for Query {
    fn drop(&mut self) {
        // Cancel reader task if still running
        if let Some(task) = self.reader_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_request_id() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();

        assert!(id1.starts_with("req_"));
        assert!(id2.starts_with("req_"));
        assert_ne!(id1, id2);
    }
}
