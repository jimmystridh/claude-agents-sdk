//! Subprocess transport implementation for the Claude CLI.
//!
//! This module provides the concrete implementation of the Transport trait
//! that communicates with the Claude CLI via subprocess stdin/stdout.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_stream::Stream;
use tracing::{debug, error, trace, warn};

use super::Transport;
use crate::errors::{ClaudeSDKError, Result};
use crate::types::*;

/// Default maximum buffer size (1MB).
const DEFAULT_MAX_BUFFER_SIZE: usize = 1024 * 1024;

/// Default CLI command name.
const DEFAULT_CLI_PATH: &str = "claude";

/// Subprocess-based transport for communicating with the Claude CLI.
///
/// This transport spawns the Claude CLI as a subprocess and communicates
/// via JSON over stdin/stdout. It handles:
/// - Process lifecycle management
/// - Command-line argument construction from options
/// - Bidirectional JSON message passing
/// - Buffer size limits and error handling
pub struct SubprocessTransport {
    /// CLI path.
    cli_path: PathBuf,
    /// Command-line arguments.
    args: Vec<String>,
    /// Environment variables.
    env: HashMap<String, String>,
    /// Maximum buffer size.
    max_buffer_size: usize,
    /// Child process handle.
    process: Option<Child>,
    /// Stdin handle (wrapped in mutex for thread safety).
    stdin: Option<Arc<Mutex<tokio::process::ChildStdin>>>,
    /// Stdout lines stream receiver.
    stdout_rx: Option<tokio::sync::mpsc::Receiver<Result<serde_json::Value>>>,
    /// Stderr callback.
    stderr_callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
    /// Whether the transport is ready.
    ready: bool,
    /// Whether we're in streaming mode.
    streaming_mode: bool,
    /// Initial prompt for non-streaming mode.
    #[allow(dead_code)]
    initial_prompt: Option<String>,
    /// Working directory.
    cwd: Option<PathBuf>,
}

impl SubprocessTransport {
    /// Create a new subprocess transport with the given options.
    pub fn new(options: &ClaudeAgentOptions, initial_prompt: Option<String>) -> Result<Self> {
        let cli_path = options
            .cli_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CLI_PATH));

        // Validate CLI exists
        if !cli_path.exists() {
            // Try to find in PATH
            if which::which(&cli_path).is_err() {
                return Err(ClaudeSDKError::cli_not_found(format!(
                    "Claude CLI not found at '{}'. Please ensure Claude Code is installed.",
                    cli_path.display()
                )));
            }
        }

        let streaming_mode = initial_prompt.is_none();
        let args = Self::build_args(options, streaming_mode, initial_prompt.as_deref())?;
        let env = Self::build_env(options);
        let max_buffer_size = options.max_buffer_size.unwrap_or(DEFAULT_MAX_BUFFER_SIZE);

        Ok(Self {
            cli_path,
            args,
            env,
            max_buffer_size,
            process: None,
            stdin: None,
            stdout_rx: None,
            stderr_callback: options.stderr.clone(),
            ready: false,
            streaming_mode,
            initial_prompt,
            cwd: options.cwd.clone(),
        })
    }

    /// Build command-line arguments from options.
    fn build_args(
        options: &ClaudeAgentOptions,
        streaming_mode: bool,
        initial_prompt: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut args = Vec::new();

        // Output format
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());

        // Verbose mode for control protocol
        args.push("--verbose".to_string());

        // Streaming mode input format
        if streaming_mode {
            args.push("--input-format".to_string());
            args.push("stream-json".to_string());
        }

        // System prompt handling:
        // - None: Pass empty string to explicitly disable default system prompt
        // - Text: Pass the custom system prompt
        // - Preset without append: No flags (use CLI's default system prompt)
        // - Preset with append: Only --append-system-prompt (append to CLI default)
        match &options.system_prompt {
            None => {
                // Explicitly disable system prompt
                args.push("--system-prompt".to_string());
                args.push(String::new());
            }
            Some(SystemPromptConfig::Text(text)) => {
                args.push("--system-prompt".to_string());
                args.push(text.clone());
            }
            Some(SystemPromptConfig::Preset(preset)) => {
                // For preset, only add append flag if present
                // Otherwise, let CLI use its default system prompt
                if let Some(ref append) = preset.append {
                    args.push("--append-system-prompt".to_string());
                    args.push(append.clone());
                }
            }
        }

        // Permission mode
        if let Some(mode) = options.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(
                match mode {
                    PermissionMode::Default => "default",
                    PermissionMode::AcceptEdits => "acceptEdits",
                    PermissionMode::Plan => "plan",
                    PermissionMode::BypassPermissions => "bypassPermissions",
                }
                .to_string(),
            );
        }

        // Model
        if let Some(ref model) = options.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        // Fallback model
        if let Some(ref model) = options.fallback_model {
            args.push("--fallback-model".to_string());
            args.push(model.clone());
        }

        // Max turns
        if let Some(turns) = options.max_turns {
            args.push("--max-turns".to_string());
            args.push(turns.to_string());
        }

        // Max budget
        if let Some(budget) = options.max_budget_usd {
            args.push("--max-budget-usd".to_string());
            args.push(budget.to_string());
        }

        // Max thinking tokens
        if let Some(tokens) = options.max_thinking_tokens {
            args.push("--max-thinking-tokens".to_string());
            args.push(tokens.to_string());
        }

        // Continue conversation
        if options.continue_conversation {
            args.push("--continue".to_string());
        }

        // Resume session
        if let Some(ref session) = options.resume {
            args.push("--resume".to_string());
            args.push(session.clone());
        }

        // Fork session
        if options.fork_session {
            args.push("--fork-session".to_string());
        }

        // Allowed tools
        for tool in &options.allowed_tools {
            args.push("--allowed-tools".to_string());
            args.push(tool.clone());
        }

        // Disallowed tools
        for tool in &options.disallowed_tools {
            args.push("--disallowed-tools".to_string());
            args.push(tool.clone());
        }

        // Tools
        if let Some(ref tools) = options.tools {
            match tools {
                ToolsConfig::List(list) => {
                    for tool in list {
                        args.push("--tools".to_string());
                        args.push(tool.clone());
                    }
                }
                ToolsConfig::Preset(preset) => {
                    args.push("--tools-preset".to_string());
                    args.push(preset.preset.clone());
                }
            }
        }

        // MCP servers
        match &options.mcp_servers {
            McpServersConfig::Path(path) => {
                args.push("--mcp-config".to_string());
                args.push(path.to_string_lossy().to_string());
            }
            McpServersConfig::Map(servers) if !servers.is_empty() => {
                let json = serde_json::to_string(servers).map_err(|e| {
                    ClaudeSDKError::configuration(format!("Failed to serialize MCP servers: {}", e))
                })?;
                args.push("--mcp-servers".to_string());
                args.push(json);
            }
            _ => {}
        }

        // User
        if let Some(ref user) = options.user {
            args.push("--user".to_string());
            args.push(user.clone());
        }

        // Settings
        if let Some(ref settings) = options.settings {
            args.push("--settings".to_string());
            args.push(settings.clone());
        }

        // Setting sources
        if let Some(ref sources) = options.setting_sources {
            for source in sources {
                args.push("--setting-source".to_string());
                args.push(
                    match source {
                        SettingSource::User => "user",
                        SettingSource::Project => "project",
                        SettingSource::Local => "local",
                    }
                    .to_string(),
                );
            }
        }

        // Additional directories
        for dir in &options.add_dirs {
            args.push("--add-dir".to_string());
            args.push(dir.to_string_lossy().to_string());
        }

        // Include partial messages
        if options.include_partial_messages {
            args.push("--include-partial-messages".to_string());
        }

        // File checkpointing
        if options.enable_file_checkpointing {
            args.push("--enable-file-checkpointing".to_string());
        }

        // Sandbox settings
        if let Some(ref sandbox) = options.sandbox {
            let json = serde_json::to_string(sandbox).map_err(|e| {
                ClaudeSDKError::configuration(format!(
                    "Failed to serialize sandbox settings: {}",
                    e
                ))
            })?;
            args.push("--sandbox".to_string());
            args.push(json);
        }

        // Output format
        if let Some(ref format) = options.output_format {
            let json = serde_json::to_string(format).map_err(|e| {
                ClaudeSDKError::configuration(format!("Failed to serialize output format: {}", e))
            })?;
            args.push("--output-format-schema".to_string());
            args.push(json);
        }

        // Agents
        if let Some(ref agents) = options.agents {
            let json = serde_json::to_string(agents).map_err(|e| {
                ClaudeSDKError::configuration(format!("Failed to serialize agents: {}", e))
            })?;
            args.push("--agents".to_string());
            args.push(json);
        }

        // Beta features
        for beta in &options.betas {
            args.push("--beta".to_string());
            args.push(
                serde_json::to_string(beta)
                    .unwrap_or_else(|_| format!("{:?}", beta))
                    .trim_matches('"')
                    .to_string(),
            );
        }

        // Extra args
        for (key, value) in &options.extra_args {
            args.push(format!("--{}", key));
            if let Some(v) = value {
                args.push(v.clone());
            }
        }

        // Non-streaming mode: add prompt
        if !streaming_mode {
            if let Some(prompt) = initial_prompt {
                args.push("--print".to_string());
                args.push("--".to_string());
                args.push(prompt.to_string());
            }
        }

        Ok(args)
    }

    /// Build environment variables.
    fn build_env(options: &ClaudeAgentOptions) -> HashMap<String, String> {
        let mut env = std::env::vars().collect::<HashMap<_, _>>();

        // Override with user-specified env vars
        for (key, value) in &options.env {
            env.insert(key.clone(), value.clone());
        }

        // Required SDK env vars
        env.insert("CLAUDE_SDK".to_string(), "true".to_string());

        env
    }

    /// Start reading stdout in background task.
    fn spawn_stdout_reader(
        stdout: tokio::process::ChildStdout,
        max_buffer_size: usize,
    ) -> tokio::sync::mpsc::Receiver<Result<serde_json::Value>> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        tokio::spawn(async move {
            let reader = BufReader::with_capacity(max_buffer_size, stdout);
            let mut lines = reader.lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let display_len = line.len().min(200);
                        trace!("Received line from CLI: {}", &line[..display_len]);

                        let result = match serde_json::from_str(&line) {
                            Ok(value) => Ok(value),
                            Err(e) => Err(ClaudeSDKError::json_decode_with_context(
                                "Failed to parse JSON from CLI",
                                Some(line),
                                None,
                                e,
                            )),
                        };

                        if tx.send(result).await.is_err() {
                            debug!("Stdout reader: receiver dropped");
                            break;
                        }
                    }
                    Ok(None) => {
                        debug!("Stdout reader: EOF received");
                        break;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(ClaudeSDKError::cli_connection_with_source(
                                "Failed to read from CLI stdout",
                                e,
                            )))
                            .await;
                        break;
                    }
                }
            }

            debug!("Stdout reader task finished");
        });

        rx
    }

    /// Start reading stderr in background task.
    fn spawn_stderr_reader(
        stderr: tokio::process::ChildStderr,
        callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        trace!("CLI stderr: {}", line);
                        if let Some(ref cb) = callback {
                            cb(line);
                        }
                    }
                    Ok(None) => {
                        // EOF
                        break;
                    }
                    Err(e) => {
                        warn!("Error reading stderr: {}", e);
                        break;
                    }
                }
            }

            debug!("Stderr reader task finished");
        });
    }
}

#[async_trait]
impl Transport for SubprocessTransport {
    async fn connect(&mut self) -> Result<()> {
        debug!(
            "Starting CLI process: {} {:?}",
            self.cli_path.display(),
            self.args
        );

        let mut cmd = Command::new(&self.cli_path);
        cmd.args(&self.args)
            .envs(&self.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // In non-streaming mode (using --print), we don't need stdin
        // Using Stdio::null() allows the CLI to complete without waiting for input
        if self.streaming_mode {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }

        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }

        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ClaudeSDKError::cli_not_found(format!(
                    "Failed to start Claude CLI at '{}': {}",
                    self.cli_path.display(),
                    e
                ))
            } else {
                ClaudeSDKError::cli_connection_with_source(
                    format!("Failed to start Claude CLI: {}", e),
                    e,
                )
            }
        })?;

        // Take stdin and wrap in mutex (only available in streaming mode)
        if self.streaming_mode {
            let stdin = child.stdin.take().ok_or_else(|| {
                ClaudeSDKError::cli_connection("Failed to open stdin to CLI process")
            })?;
            self.stdin = Some(Arc::new(Mutex::new(stdin)));
        }

        // Take stdout and start reader task
        let stdout = child.stdout.take().ok_or_else(|| {
            ClaudeSDKError::cli_connection("Failed to open stdout from CLI process")
        })?;
        self.stdout_rx = Some(Self::spawn_stdout_reader(stdout, self.max_buffer_size));

        // Take stderr and start reader task
        if let Some(stderr) = child.stderr.take() {
            Self::spawn_stderr_reader(stderr, self.stderr_callback.clone());
        }

        self.process = Some(child);
        self.ready = true;

        debug!("CLI process started successfully");
        Ok(())
    }

    async fn write(&self, data: &str) -> Result<()> {
        let stdin = self
            .stdin
            .as_ref()
            .ok_or_else(|| ClaudeSDKError::cli_connection("Transport not connected"))?;

        let mut stdin_guard = stdin.lock().await;
        trace!("Writing to CLI: {}", &data[..data.len().min(200)]);

        stdin_guard.write_all(data.as_bytes()).await.map_err(|e| {
            ClaudeSDKError::cli_connection_with_source("Failed to write to CLI stdin", e)
        })?;

        stdin_guard.write_all(b"\n").await.map_err(|e| {
            ClaudeSDKError::cli_connection_with_source("Failed to write newline to CLI stdin", e)
        })?;

        stdin_guard.flush().await.map_err(|e| {
            ClaudeSDKError::cli_connection_with_source("Failed to flush CLI stdin", e)
        })?;

        Ok(())
    }

    fn message_stream(&self) -> Pin<Box<dyn Stream<Item = Result<serde_json::Value>> + Send + '_>> {
        // The message_stream method from the Transport trait cannot be properly
        // implemented with &self because we need to take ownership of the channel.
        // Users should use take_stdout_rx() instead which takes &mut self.
        //
        // This returns an empty stream - the actual message stream is obtained
        // via take_stdout_rx() on SubprocessTransport directly.
        //
        // Note: If custom transport support is re-added in the future, this trait
        // method should be redesigned to use &mut self or an Arc-wrapped receiver.
        warn!("message_stream() called on SubprocessTransport - use take_stdout_rx() instead");
        Box::pin(futures::stream::empty())
    }

    async fn close(&mut self) -> Result<()> {
        self.ready = false;

        // Close stdin first
        if let Some(stdin) = self.stdin.take() {
            drop(stdin);
        }

        // Wait for process to exit or kill it
        if let Some(mut process) = self.process.take() {
            // Give it a moment to exit gracefully
            match tokio::time::timeout(std::time::Duration::from_secs(2), process.wait()).await {
                Ok(Ok(status)) => {
                    debug!("CLI process exited with status: {:?}", status);
                }
                Ok(Err(e)) => {
                    error!("Error waiting for CLI process: {}", e);
                }
                Err(_) => {
                    warn!("CLI process did not exit in time, killing");
                    let _ = process.kill().await;
                }
            }
        }

        Ok(())
    }

    async fn end_input(&self) -> Result<()> {
        // Closing stdin signals EOF to the process
        if let Some(stdin) = &self.stdin {
            let mut guard = stdin.lock().await;
            guard.shutdown().await.map_err(|e| {
                ClaudeSDKError::cli_connection_with_source("Failed to shutdown stdin", e)
            })?;
        }
        Ok(())
    }

    fn is_ready(&self) -> bool {
        self.ready
    }
}

impl SubprocessTransport {
    /// Get the stdout receiver for message reading.
    pub fn take_stdout_rx(
        &mut self,
    ) -> Option<tokio::sync::mpsc::Receiver<Result<serde_json::Value>>> {
        self.stdout_rx.take()
    }

    /// Check if in streaming mode.
    pub fn is_streaming_mode(&self) -> bool {
        self.streaming_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_basic() {
        let options = ClaudeAgentOptions::default();
        let args = SubprocessTransport::build_args(&options, true, None).unwrap();

        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"stream-json".to_string()));
        assert!(args.contains(&"--verbose".to_string()));
        assert!(args.contains(&"--input-format".to_string()));
    }

    #[test]
    fn test_build_args_with_model() {
        let options = ClaudeAgentOptions::new().with_model("claude-3-sonnet");
        let args = SubprocessTransport::build_args(&options, true, None).unwrap();

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"claude-3-sonnet".to_string()));
    }

    #[test]
    fn test_build_args_non_streaming() {
        let options = ClaudeAgentOptions::default();
        let args = SubprocessTransport::build_args(&options, false, Some("Hello")).unwrap();

        assert!(args.contains(&"--print".to_string()));
        assert!(args.contains(&"Hello".to_string()));
        assert!(!args.contains(&"--input-format".to_string()));
    }

    #[test]
    fn test_build_env() {
        let mut options = ClaudeAgentOptions::default();
        options
            .env
            .insert("CUSTOM_VAR".to_string(), "value".to_string());

        let env = SubprocessTransport::build_env(&options);

        assert_eq!(env.get("CLAUDE_SDK"), Some(&"true".to_string()));
        assert_eq!(env.get("CUSTOM_VAR"), Some(&"value".to_string()));
    }

    #[test]
    fn test_build_args_system_prompt_none() {
        // When system_prompt is None, should pass empty string to disable default
        let options = ClaudeAgentOptions::default();
        let args = SubprocessTransport::build_args(&options, true, None).unwrap();

        let sp_idx = args.iter().position(|a| a == "--system-prompt");
        assert!(sp_idx.is_some(), "Should have --system-prompt flag");
        assert_eq!(
            args[sp_idx.unwrap() + 1],
            "",
            "System prompt should be empty string"
        );
    }

    #[test]
    fn test_build_args_system_prompt_string() {
        // When system_prompt is a string, should pass it directly
        let options = ClaudeAgentOptions::new().with_system_prompt("You are a pirate.");
        let args = SubprocessTransport::build_args(&options, true, None).unwrap();

        let sp_idx = args.iter().position(|a| a == "--system-prompt");
        assert!(sp_idx.is_some(), "Should have --system-prompt flag");
        assert_eq!(
            args[sp_idx.unwrap() + 1],
            "You are a pirate.",
            "System prompt should match"
        );
    }

    #[test]
    fn test_build_args_system_prompt_preset_no_append() {
        use crate::types::{SystemPromptConfig, SystemPromptPreset};

        // When system_prompt is a preset without append, should NOT pass any system-prompt flags
        let mut options = ClaudeAgentOptions::new();
        options.system_prompt = Some(SystemPromptConfig::Preset(SystemPromptPreset {
            preset_type: "preset".to_string(),
            preset: "claude_code".to_string(),
            append: None,
        }));
        let args = SubprocessTransport::build_args(&options, true, None).unwrap();

        assert!(
            !args.contains(&"--system-prompt".to_string()),
            "Should NOT have --system-prompt flag for preset"
        );
        assert!(
            !args.contains(&"--append-system-prompt".to_string()),
            "Should NOT have --append-system-prompt flag without append"
        );
    }

    #[test]
    fn test_build_args_system_prompt_preset_with_append() {
        use crate::types::{SystemPromptConfig, SystemPromptPreset};

        // When system_prompt is a preset with append, should only pass --append-system-prompt
        let mut options = ClaudeAgentOptions::new();
        options.system_prompt = Some(SystemPromptConfig::Preset(SystemPromptPreset {
            preset_type: "preset".to_string(),
            preset: "claude_code".to_string(),
            append: Some("Be concise.".to_string()),
        }));
        let args = SubprocessTransport::build_args(&options, true, None).unwrap();

        assert!(
            !args.contains(&"--system-prompt".to_string()),
            "Should NOT have --system-prompt flag for preset"
        );

        let append_idx = args.iter().position(|a| a == "--append-system-prompt");
        assert!(
            append_idx.is_some(),
            "Should have --append-system-prompt flag"
        );
        assert_eq!(
            args[append_idx.unwrap() + 1],
            "Be concise.",
            "Append text should match"
        );
    }
}
