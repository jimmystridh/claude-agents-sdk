//! Error types for the Claude Agents SDK.
//!
//! This module provides a comprehensive error hierarchy for handling various
//! failure modes when interacting with the Claude Code CLI.

use std::io;
use thiserror::Error;

/// Main error type for the Claude Agents SDK.
///
/// All errors in this SDK derive from this type, making it easy to handle
/// errors at a high level or match on specific error variants.
#[derive(Error, Debug)]
pub enum ClaudeSDKError {
    /// The Claude Code CLI was not found on the system.
    #[error("Claude Code CLI not found: {message}")]
    CLINotFound {
        /// Detailed error message
        message: String,
    },

    /// Failed to connect to or communicate with the CLI process.
    #[error("CLI connection error: {message}")]
    CLIConnection {
        /// Detailed error message
        message: String,
        /// Underlying IO error if applicable
        #[source]
        source: Option<io::Error>,
    },

    /// The CLI process exited with an error.
    #[error("Process error (exit code {exit_code:?}): {message}")]
    Process {
        /// Exit code of the process
        exit_code: Option<i32>,
        /// Error message
        message: String,
        /// Captured stderr output
        stderr: Option<String>,
    },

    /// Failed to decode JSON from the CLI.
    #[error("JSON decode error: {message}")]
    JSONDecode {
        /// Error message describing the parse failure
        message: String,
        /// The raw data that failed to parse
        raw_data: Option<String>,
        /// The buffer content at time of error
        buffer_content: Option<String>,
        /// Underlying serde_json error
        #[source]
        source: Option<serde_json::Error>,
    },

    /// Failed to parse a message into the expected type.
    #[error("Message parse error: {message}")]
    MessageParse {
        /// Error message
        message: String,
        /// The raw message data that failed to parse
        raw_message: Option<serde_json::Value>,
    },

    /// An invalid configuration was provided.
    #[error("Configuration error: {message}")]
    Configuration {
        /// Error message
        message: String,
    },

    /// A control protocol error occurred.
    #[error("Control protocol error: {message}")]
    ControlProtocol {
        /// Error message
        message: String,
        /// Request ID if applicable
        request_id: Option<String>,
    },

    /// The operation was interrupted.
    #[error("Operation interrupted")]
    Interrupted,

    /// A timeout occurred.
    #[error("Operation timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds
        duration_ms: u64,
    },

    /// The CLI version is too old.
    #[error("CLI version {found} is below minimum required version {required}")]
    VersionMismatch {
        /// The version that was found
        found: String,
        /// The minimum required version
        required: String,
    },

    /// An IO error occurred.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// A channel communication error occurred.
    #[error("Channel error: {message}")]
    Channel {
        /// Error message
        message: String,
    },

    /// An internal error that should not normally occur.
    #[error("Internal error: {message}")]
    Internal {
        /// Error message
        message: String,
    },
}

impl ClaudeSDKError {
    /// Create a CLI not found error.
    pub fn cli_not_found(message: impl Into<String>) -> Self {
        Self::CLINotFound {
            message: message.into(),
        }
    }

    /// Create a CLI connection error.
    pub fn cli_connection(message: impl Into<String>) -> Self {
        Self::CLIConnection {
            message: message.into(),
            source: None,
        }
    }

    /// Create a CLI connection error with an underlying IO error.
    pub fn cli_connection_with_source(message: impl Into<String>, source: io::Error) -> Self {
        Self::CLIConnection {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create a process error.
    pub fn process(exit_code: Option<i32>, message: impl Into<String>) -> Self {
        Self::Process {
            exit_code,
            message: message.into(),
            stderr: None,
        }
    }

    /// Create a process error with stderr output.
    pub fn process_with_stderr(
        exit_code: Option<i32>,
        message: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self::Process {
            exit_code,
            message: message.into(),
            stderr: Some(stderr.into()),
        }
    }

    /// Create a JSON decode error.
    pub fn json_decode(message: impl Into<String>) -> Self {
        Self::JSONDecode {
            message: message.into(),
            raw_data: None,
            buffer_content: None,
            source: None,
        }
    }

    /// Create a JSON decode error with context.
    pub fn json_decode_with_context(
        message: impl Into<String>,
        raw_data: Option<String>,
        buffer_content: Option<String>,
        source: serde_json::Error,
    ) -> Self {
        Self::JSONDecode {
            message: message.into(),
            raw_data,
            buffer_content,
            source: Some(source),
        }
    }

    /// Create a message parse error.
    pub fn message_parse(message: impl Into<String>) -> Self {
        Self::MessageParse {
            message: message.into(),
            raw_message: None,
        }
    }

    /// Create a message parse error with raw message data.
    pub fn message_parse_with_raw(
        message: impl Into<String>,
        raw_message: serde_json::Value,
    ) -> Self {
        Self::MessageParse {
            message: message.into(),
            raw_message: Some(raw_message),
        }
    }

    /// Create a configuration error.
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a control protocol error.
    pub fn control_protocol(message: impl Into<String>) -> Self {
        Self::ControlProtocol {
            message: message.into(),
            request_id: None,
        }
    }

    /// Create a control protocol error with request ID.
    pub fn control_protocol_with_id(message: impl Into<String>, request_id: impl Into<String>) -> Self {
        Self::ControlProtocol {
            message: message.into(),
            request_id: Some(request_id.into()),
        }
    }

    /// Create a timeout error.
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }

    /// Create a version mismatch error.
    pub fn version_mismatch(found: impl Into<String>, required: impl Into<String>) -> Self {
        Self::VersionMismatch {
            found: found.into(),
            required: required.into(),
        }
    }

    /// Create a channel error.
    pub fn channel(message: impl Into<String>) -> Self {
        Self::Channel {
            message: message.into(),
        }
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Check if this error indicates the CLI was not found.
    pub fn is_cli_not_found(&self) -> bool {
        matches!(self, Self::CLINotFound { .. })
    }

    /// Check if this error is recoverable (might succeed if retried).
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::CLIConnection { .. } | Self::Timeout { .. } | Self::Channel { .. }
        )
    }
}

/// Result type alias for SDK operations.
pub type Result<T> = std::result::Result<T, ClaudeSDKError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ClaudeSDKError::cli_not_found("claude not in PATH");
        assert!(err.to_string().contains("claude not in PATH"));
    }

    #[test]
    fn test_process_error_with_exit_code() {
        let err = ClaudeSDKError::process(Some(1), "command failed");
        assert!(err.to_string().contains("exit code"));
        assert!(err.to_string().contains("1"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(ClaudeSDKError::timeout(1000).is_recoverable());
        assert!(!ClaudeSDKError::cli_not_found("not found").is_recoverable());
    }
}
