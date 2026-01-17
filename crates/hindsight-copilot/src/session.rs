//! Chat session types and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a Copilot chat session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl ChatSession {
    /// Create a new empty session
    #[must_use]
    pub fn new(id: String, workspace_id: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            id,
            workspace_id,
            created_at: timestamp,
            updated_at: timestamp,
            messages: Vec::new(),
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, message: ChatMessage) {
        self.updated_at = message.timestamp;
        self.messages.push(message);
    }

    /// Get the number of messages in the session
    #[must_use]
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get all user messages
    #[must_use]
    pub fn user_messages(&self) -> Vec<&ChatMessage> {
        self.messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .collect()
    }

    /// Get all assistant messages
    #[must_use]
    pub fn assistant_messages(&self) -> Vec<&ChatMessage> {
        self.messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .collect()
    }

    /// Check if session is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

/// Represents a message in a chat session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl ChatMessage {
    /// Create a new user message
    #[must_use]
    pub fn user(content: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            role: MessageRole::User,
            content,
            timestamp,
            agent: None,
        }
    }

    /// Create a new assistant message
    #[must_use]
    pub fn assistant(content: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
            timestamp,
            agent: None,
        }
    }

    /// Set the agent for this message
    #[must_use]
    pub fn with_agent(mut self, agent: String) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Get the content length
    #[must_use]
    pub fn content_len(&self) -> usize {
        self.content.len()
    }

    /// Check if message has an associated agent
    #[must_use]
    pub fn has_agent(&self) -> bool {
        self.agent.is_some()
    }
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

impl MessageRole {
    /// Get the display name for this role
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::User => "User",
            Self::Assistant => "Copilot",
            Self::System => "System",
        }
    }
}

/// Get the default Copilot chat sessions directory for the current OS
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use similar_asserts::assert_eq;

    fn sample_timestamp() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap()
    }

    fn sample_session() -> ChatSession {
        let ts = sample_timestamp();
        let mut session =
            ChatSession::new("session-123".to_string(), "workspace-456".to_string(), ts);
        session.add_message(ChatMessage::user("Hello".to_string(), ts));
        session.add_message(ChatMessage::assistant("Hi there!".to_string(), ts));
        session
    }

    #[test]
    fn test_session_serialization_roundtrip() {
        let session = sample_session();
        let json = serde_json::to_string(&session).expect("serialize");
        let deserialized: ChatSession = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(session, deserialized);
    }

    #[test]
    fn test_session_new() {
        let ts = sample_timestamp();
        let session = ChatSession::new("id".to_string(), "ws".to_string(), ts);
        assert_eq!(session.id, "id");
        assert_eq!(session.workspace_id, "ws");
        assert!(session.is_empty());
        assert_eq!(session.message_count(), 0);
    }

    #[test]
    fn test_session_add_message_updates_timestamp() {
        let ts1 = sample_timestamp();
        let ts2 = Utc.with_ymd_and_hms(2026, 1, 17, 3, 0, 0).unwrap();

        let mut session = ChatSession::new("id".to_string(), "ws".to_string(), ts1);
        assert_eq!(session.updated_at, ts1);

        session.add_message(ChatMessage::user("test".to_string(), ts2));
        assert_eq!(session.updated_at, ts2);
    }

    #[test]
    fn test_session_user_messages() {
        let session = sample_session();
        let user_msgs = session.user_messages();
        assert_eq!(user_msgs.len(), 1);
        assert_eq!(user_msgs[0].content, "Hello");
    }

    #[test]
    fn test_session_assistant_messages() {
        let session = sample_session();
        let assistant_msgs = session.assistant_messages();
        assert_eq!(assistant_msgs.len(), 1);
        assert_eq!(assistant_msgs[0].content, "Hi there!");
    }

    #[test]
    fn test_message_serialization_roundtrip() {
        let msg = ChatMessage::user("Test message".to_string(), sample_timestamp());
        let json = serde_json::to_string(&msg).expect("serialize");
        let deserialized: ChatMessage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_message_with_agent() {
        let msg = ChatMessage::user("Test".to_string(), sample_timestamp())
            .with_agent("@workspace".to_string());

        assert!(msg.has_agent());
        assert_eq!(msg.agent, Some("@workspace".to_string()));
    }

    #[test]
    fn test_message_agent_skipped_when_none() {
        let msg = ChatMessage::user("Test".to_string(), sample_timestamp());
        let json = serde_json::to_string(&msg).expect("serialize");
        // agent field should be omitted when None
        assert!(!json.contains("agent"));
    }

    #[test]
    fn test_message_content_len() {
        let msg = ChatMessage::user("Hello, World!".to_string(), sample_timestamp());
        assert_eq!(msg.content_len(), 13);
    }

    #[test]
    fn test_message_role_serialization() {
        let roles = vec![
            (MessageRole::User, "\"user\""),
            (MessageRole::Assistant, "\"assistant\""),
            (MessageRole::System, "\"system\""),
        ];

        for (role, expected) in roles {
            let json = serde_json::to_string(&role).expect("serialize");
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_message_role_display_name() {
        assert_eq!(MessageRole::User.display_name(), "User");
        assert_eq!(MessageRole::Assistant.display_name(), "Copilot");
        assert_eq!(MessageRole::System.display_name(), "System");
    }

    #[test]
    fn test_default_chat_sessions_dir_returns_path() {
        // This test verifies the function doesn't panic and returns a valid path
        // on supported platforms
        let path = default_chat_sessions_dir();

        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        {
            assert!(path.is_some());
            let p = path.unwrap();
            assert!(p.to_string_lossy().contains("workspaceStorage"));
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            assert!(path.is_none());
        }
    }
}
