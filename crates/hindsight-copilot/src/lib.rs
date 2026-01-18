// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! hindsight-copilot: GitHub Copilot log processing for hindsight-mcp
//!
//! This library crate provides functionality to parse and process GitHub Copilot
//! logs and chat sessions for consumption by the hindsight-mcp server.

#![warn(missing_docs)]

//! ## Log Locations
//!
//! VS Code stores Copilot chat history in local SQLite databases and JSON files:
//! - **macOS:** `~/Library/Application Support/Code/User/workspaceStorage/<workspace-id>/chatSessions/`
//! - **Windows:** `%APPDATA%\Code\User\workspaceStorage\<workspace-id>\chatSessions\`
//! - **Linux:** `~/.config/Code/User/workspaceStorage/<workspace-id>/chatSessions/`
//!
//! ## Log Format
//!
//! Copilot logs follow JSON Stream / LSP Trace format when log level is set to `Trace`.
//!
//! ## Session Discovery
//!
//! Use [`SessionDiscovery`] to find chat sessions across all workspaces:
//!
//! ```rust,no_run
//! use hindsight_copilot::session::{SessionDiscovery, parse_session_file};
//!
//! let discovery = SessionDiscovery::new().expect("find storage");
//! for session in discovery.discover_sessions().expect("enumerate") {
//!     let parsed = parse_session_file(&session.path, &session.workspace_storage_id);
//!     println!("{:?}", parsed);
//! }
//! ```

pub mod error;
pub mod lsp;
pub mod parser;
pub mod session;

pub use error::CopilotError;

// Re-export session discovery types at crate level for convenience
pub use session::{
    ChatMessage, ChatSession, DiscoveredSession, MessageRole, SessionDiscovery, Variable,
    WorkspaceInfo, default_chat_sessions_dir, parse_session_file, parse_session_json,
};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::error::CopilotError;
    pub use crate::session::{
        ChatMessage, ChatSession, DiscoveredSession, MessageRole, SessionDiscovery, Variable,
        WorkspaceInfo,
    };
}
