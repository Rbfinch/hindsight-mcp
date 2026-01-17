//! Chat session types and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a Copilot chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    /// Session ID
    pub id: String,
    /// Workspace ID this session belongs to
    pub workspace_id: String,
    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
    /// Session last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Messages in this session
    pub messages: Vec<ChatMessage>,
}

/// Represents a message in a chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message role (user, assistant, system)
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Associated agent (e.g., @workspace)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

/// Message role in a chat conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message
    User,
    /// Assistant (Copilot) response
    Assistant,
    /// System message
    System,
}

/// Get the default Copilot chat sessions directory for the current OS
pub fn default_chat_sessions_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| h.join("Library/Application Support/Code/User/workspaceStorage"))
    }
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir().map(|c| c.join("Code/User/workspaceStorage"))
    }
    #[cfg(target_os = "linux")]
    {
        dirs::config_dir().map(|c| c.join("Code/User/workspaceStorage"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        None
    }
}

// TODO: Implement session loading and parsing
