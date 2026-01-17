//! Error types for hindsight-copilot

use thiserror::Error;

/// Errors that can occur during Copilot log processing
#[derive(Debug, Error)]
pub enum CopilotError {
    /// Error parsing JSON
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Error reading log file
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid LSP message format
    #[error("Invalid LSP message: {message}")]
    InvalidLspMessage { message: String },

    /// Workspace storage not found
    #[error("Workspace storage not found: {path}")]
    WorkspaceStorageNotFound { path: String },

    /// Chat session not found
    #[error("Chat session not found: {session_id}")]
    SessionNotFound { session_id: String },
}
