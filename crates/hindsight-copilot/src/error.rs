// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

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
    InvalidLspMessage {
        /// Description of why the LSP message was invalid
        message: String,
    },

    /// Workspace storage not found
    #[error("Workspace storage not found: {path}")]
    WorkspaceStorageNotFound {
        /// The path that was searched for workspace storage
        path: String,
    },

    /// Chat session not found
    #[error("Chat session not found: {session_id}")]
    SessionNotFound {
        /// The session ID that could not be found
        session_id: String,
    },
}
