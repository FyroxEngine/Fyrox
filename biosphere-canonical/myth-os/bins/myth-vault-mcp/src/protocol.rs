//! Minimal JSON-RPC 2.0 / MCP protocol types.
//! MCP over stdio: newline-delimited JSON objects on stdin/stdout.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ─── JSON-RPC ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id:      Option<Value>,
    pub method:  String,
    pub params:  Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id:      Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<RpcError>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code:    i32,
    pub message: String,
}

impl Response {
    pub fn ok(id: Value, result: impl Serialize) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
            error: None,
        }
    }

    pub fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(RpcError { code, message: message.into() }),
        }
    }
}

// ─── MCP types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name:    &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Capabilities {
    pub tools: ToolsCapability,
}

#[derive(Debug, Serialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct Tool {
    pub name:        &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<TextContent>,
    #[serde(rename = "isError")]
    pub is_error: bool,
}

#[derive(Debug, Serialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub text: String,
}

impl ToolResult {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            content: vec![TextContent { kind: "text", text: s.into() }],
            is_error: false,
        }
    }

    pub fn error(s: impl Into<String>) -> Self {
        Self {
            content: vec![TextContent { kind: "text", text: s.into() }],
            is_error: true,
        }
    }

    pub fn json(v: impl Serialize) -> Self {
        Self::text(serde_json::to_string_pretty(&v).unwrap_or_default())
    }
}
