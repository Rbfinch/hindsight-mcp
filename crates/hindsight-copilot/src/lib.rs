//! hindsight-copilot: GitHub Copilot log processing for hindsight-mcp
//!
//! This library crate provides functionality to parse and process GitHub Copilot
//! logs and chat sessions for consumption by the hindsight-mcp server.
//!
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

pub mod error;
pub mod lsp;
pub mod parser;
pub mod session;

pub use error::CopilotError;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::error::CopilotError;
    pub use crate::session::ChatSession;
}
