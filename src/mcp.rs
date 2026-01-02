//! MCP (Model Context Protocol) tool support.
//!
//! This module provides functionality for defining SDK-managed tools that
//! run in-process rather than as external servers.
//!
//! # Feature Flag
//!
//! This module requires the `mcp` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! claude-agents-sdk = { version = "0.1", features = ["mcp"] }
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Content type for tool responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    /// Text content.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },
    /// Image content.
    #[serde(rename = "image")]
    Image {
        /// Base64-encoded image data.
        data: String,
        /// MIME type of the image.
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

impl ToolContent {
    /// Create a text content item.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an image content item.
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }
}

/// Result from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Content items returned by the tool.
    pub content: Vec<ToolContent>,
    /// Whether the result is an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    /// Create a successful text result.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: None,
        }
    }

    /// Create an error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(message)],
            is_error: Some(true),
        }
    }

    /// Create a result with multiple content items.
    pub fn with_content(content: Vec<ToolContent>) -> Self {
        Self {
            content,
            is_error: None,
        }
    }
}

/// Input schema for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    /// JSON Schema type (usually "object").
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Property definitions.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, Value>,
    /// Required property names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

impl ToolInputSchema {
    /// Create a new object schema.
    pub fn object() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: Vec::new(),
        }
    }

    /// Add a string property.
    pub fn string_property(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.properties.insert(
            name.into(),
            serde_json::json!({
                "type": "string",
                "description": description.into()
            }),
        );
        self
    }

    /// Add a number property.
    pub fn number_property(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.properties.insert(
            name.into(),
            serde_json::json!({
                "type": "number",
                "description": description.into()
            }),
        );
        self
    }

    /// Add a boolean property.
    pub fn boolean_property(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.properties.insert(
            name.into(),
            serde_json::json!({
                "type": "boolean",
                "description": description.into()
            }),
        );
        self
    }

    /// Add a required property name.
    pub fn required_property(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }
}

/// Type alias for tool handler functions.
pub type ToolHandler = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = ToolResult> + Send>> + Send + Sync,
>;

/// SDK MCP tool definition.
///
/// Represents a tool that can be registered with the SDK for in-process execution.
pub struct SdkMcpTool {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// Input schema.
    pub input_schema: ToolInputSchema,
    /// Handler function.
    pub handler: ToolHandler,
}

impl SdkMcpTool {
    /// Create a new tool.
    pub fn new<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: ToolInputSchema,
        handler: F,
    ) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ToolResult> + Send + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            handler: Arc::new(move |input| Box::pin(handler(input))),
        }
    }
}

impl std::fmt::Debug for SdkMcpTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdkMcpTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_schema", &self.input_schema)
            .finish()
    }
}

/// Configuration for an SDK MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSdkServerConfig {
    /// Server type (always "sdk").
    #[serde(rename = "type")]
    pub server_type: String,
    /// Server name.
    pub name: String,
    /// Server version.
    pub version: String,
}

/// Create an SDK MCP server configuration.
///
/// # Arguments
///
/// * `name` - Server name
/// * `version` - Server version
/// * `tools` - List of tools to register
///
/// # Returns
///
/// A tuple of (config, tools) where config can be added to options.mcp_servers
/// and tools should be stored for handling tool calls.
///
/// # Examples
///
/// ```rust,no_run
/// use claude_agents_sdk::mcp::{create_sdk_mcp_server, SdkMcpTool, ToolInputSchema, ToolResult};
///
/// let calculator = SdkMcpTool::new(
///     "add",
///     "Add two numbers",
///     ToolInputSchema::object()
///         .number_property("a", "First number")
///         .number_property("b", "Second number")
///         .required_property("a")
///         .required_property("b"),
///     |input| async move {
///         let a = input.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
///         let b = input.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
///         ToolResult::text(format!("{}", a + b))
///     },
/// );
///
/// let (config, tools) = create_sdk_mcp_server("calculator", "1.0.0", vec![calculator]);
/// ```
pub fn create_sdk_mcp_server(
    name: impl Into<String>,
    version: impl Into<String>,
    tools: Vec<SdkMcpTool>,
) -> (McpSdkServerConfig, Vec<SdkMcpTool>) {
    let config = McpSdkServerConfig {
        server_type: "sdk".to_string(),
        name: name.into(),
        version: version.into(),
    };

    (config, tools)
}

/// Macro for defining tools with a simpler syntax.
///
/// # Examples
///
/// ```rust,ignore
/// use claude_agents_sdk::tool;
///
/// tool! {
///     /// Add two numbers together.
///     fn add(a: f64, b: f64) -> ToolResult {
///         ToolResult::text(format!("{}", a + b))
///     }
/// }
/// ```
#[macro_export]
macro_rules! tool {
    (
        $(#[$meta:meta])*
        fn $name:ident($($arg:ident: $type:ty),*) -> $ret:ty $body:block
    ) => {
        {
            use $crate::mcp::{SdkMcpTool, ToolInputSchema, ToolResult};

            let mut schema = ToolInputSchema::object();
            $(
                schema = schema.string_property(stringify!($arg), "");
                schema = schema.required_property(stringify!($arg));
            )*

            SdkMcpTool::new(
                stringify!($name),
                "",
                schema,
                |input: serde_json::Value| async move {
                    $(
                        let $arg: $type = serde_json::from_value(
                            input.get(stringify!($arg)).cloned().unwrap_or_default()
                        ).unwrap_or_default();
                    )*
                    $body
                },
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_content_text() {
        let content = ToolContent::text("Hello");
        match content {
            ToolContent::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_tool_result_text() {
        let result = ToolResult::text("Success");
        assert_eq!(result.content.len(), 1);
        assert!(result.is_error.is_none());
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("Something went wrong");
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_input_schema_builder() {
        let schema = ToolInputSchema::object()
            .string_property("name", "The name")
            .number_property("age", "The age")
            .required_property("name");

        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.contains_key("name"));
        assert!(schema.properties.contains_key("age"));
        assert!(schema.required.contains(&"name".to_string()));
    }

    #[test]
    fn test_create_sdk_server() {
        let tool = SdkMcpTool::new(
            "test",
            "Test tool",
            ToolInputSchema::object(),
            |_| async { ToolResult::text("ok") },
        );

        let (config, tools) = create_sdk_mcp_server("test-server", "1.0.0", vec![tool]);

        assert_eq!(config.server_type, "sdk");
        assert_eq!(config.name, "test-server");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(tools.len(), 1);
    }
}
