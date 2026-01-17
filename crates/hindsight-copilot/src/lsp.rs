//! LSP types and parsing for Copilot logs
//!
//! GitHub Copilot operates as a Language Server, so its log data follows
//! the Language Server Protocol (LSP) format.

use lsp_types::{Position, Range};
use serde::{Deserialize, Serialize};

/// Represents an LSP-style message from Copilot logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspMessage {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Message ID (for request/response correlation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    /// Method name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Result (for responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (for error responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// Code context sent to Copilot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    /// The text document URI
    pub uri: String,
    /// Position in the document
    pub position: Position,
    /// Visible range in the editor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_range: Option<Range>,
}

// TODO: Implement LSP message parsing
