// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! LSP types and parsing for Copilot logs
//!
//! GitHub Copilot operates as a Language Server, so its log data follows
//! the Language Server Protocol (LSP) format.

use lsp_types::{Position, Range};
use serde::{Deserialize, Serialize};

/// Represents an LSP-style message from Copilot logs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl LspMessage {
    /// Create a new request message
    #[must_use]
    pub fn request(id: impl Into<serde_json::Value>, method: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id.into()),
            method: Some(method.to_string()),
            params: None,
            result: None,
            error: None,
        }
    }

    /// Create a new notification message (no id)
    #[must_use]
    pub fn notification(method: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some(method.to_string()),
            params: None,
            result: None,
            error: None,
        }
    }

    /// Check if this is a request (has id and method)
    #[must_use]
    pub fn is_request(&self) -> bool {
        self.id.is_some() && self.method.is_some()
    }

    /// Check if this is a response (has id and result or error)
    #[must_use]
    pub fn is_response(&self) -> bool {
        self.id.is_some() && (self.result.is_some() || self.error.is_some())
    }

    /// Check if this is a notification (has method but no id)
    #[must_use]
    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }

    /// Check if this is an error response
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Add parameters to the message
    #[must_use]
    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }
}

/// Code context sent to Copilot
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeContext {
    /// The text document URI
    pub uri: String,
    /// Position in the document
    pub position: Position,
    /// Visible range in the editor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_range: Option<Range>,
}

impl CodeContext {
    /// Create a new code context
    #[must_use]
    pub fn new(uri: String, line: u32, character: u32) -> Self {
        Self {
            uri,
            position: Position { line, character },
            visible_range: None,
        }
    }

    /// Set the visible range
    #[must_use]
    pub fn with_visible_range(mut self, start: Position, end: Position) -> Self {
        self.visible_range = Some(Range { start, end });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;

    #[test]
    fn test_lsp_message_serialization_roundtrip() {
        let msg = LspMessage::request(1, "textDocument/completion")
            .with_params(serde_json::json!({"textDocument": {"uri": "file:///test.rs"}}));

        let json = serde_json::to_string(&msg).expect("serialize");
        let deserialized: LspMessage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_lsp_message_request() {
        let msg = LspMessage::request(42, "test/method");

        assert!(msg.is_request());
        assert!(!msg.is_response());
        assert!(!msg.is_notification());
        assert_eq!(msg.jsonrpc, "2.0");
        assert_eq!(msg.method, Some("test/method".to_string()));
    }

    #[test]
    fn test_lsp_message_notification() {
        let msg = LspMessage::notification("window/logMessage");

        assert!(msg.is_notification());
        assert!(!msg.is_request());
        assert!(!msg.is_response());
        assert!(msg.id.is_none());
    }

    #[test]
    fn test_lsp_message_response() {
        let msg = LspMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: None,
            params: None,
            result: Some(serde_json::json!({"completions": []})),
            error: None,
        };

        assert!(msg.is_response());
        assert!(!msg.is_request());
        assert!(!msg.is_error());
    }

    #[test]
    fn test_lsp_message_error_response() {
        let msg = LspMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: None,
            params: None,
            result: None,
            error: Some(serde_json::json!({"code": -32600, "message": "Invalid Request"})),
        };

        assert!(msg.is_response());
        assert!(msg.is_error());
    }

    #[test]
    fn test_lsp_message_skips_none_fields() {
        let msg = LspMessage::notification("test");
        let json = serde_json::to_string(&msg).expect("serialize");

        // None fields should be omitted
        assert!(!json.contains("\"id\""));
        assert!(!json.contains("\"params\""));
        assert!(!json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_code_context_serialization_roundtrip() {
        let ctx = CodeContext::new("file:///test.rs".to_string(), 10, 5);

        let json = serde_json::to_string(&ctx).expect("serialize");
        let deserialized: CodeContext = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ctx, deserialized);
    }

    #[test]
    fn test_code_context_with_visible_range() {
        let ctx = CodeContext::new("file:///test.rs".to_string(), 10, 5).with_visible_range(
            Position {
                line: 0,
                character: 0,
            },
            Position {
                line: 50,
                character: 0,
            },
        );

        assert!(ctx.visible_range.is_some());
        let range = ctx.visible_range.unwrap();
        assert_eq!(range.start.line, 0);
        assert_eq!(range.end.line, 50);
    }

    #[test]
    fn test_code_context_skips_none_visible_range() {
        let ctx = CodeContext::new("file:///test.rs".to_string(), 10, 5);
        let json = serde_json::to_string(&ctx).expect("serialize");

        assert!(!json.contains("visible_range"));
    }
}
