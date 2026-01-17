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

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy to generate valid MessageRole values
    fn role_strategy() -> impl Strategy<Value = MessageRole> {
        prop_oneof![
            Just(MessageRole::User),
            Just(MessageRole::Assistant),
            Just(MessageRole::System),
        ]
    }

    /// Strategy to generate arbitrary ChatMessage values
    fn message_strategy() -> impl Strategy<Value = ChatMessage> {
        (
            role_strategy(),
            ".*",                           // content
            0i64..2_000_000_000i64,         // timestamp as unix seconds
            proptest::option::of("@[a-z]+"), // agent
        )
            .prop_map(|(role, content, ts, agent)| {
                let timestamp = DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now());
                ChatMessage {
                    role,
                    content,
                    timestamp,
                    agent,
                }
            })
    }

    /// Strategy to generate session IDs
    fn session_id_strategy() -> impl Strategy<Value = String> {
        "[a-z0-9-]{8,36}".prop_map(|s| s.to_string())
    }

    /// Strategy to generate arbitrary ChatSession values
    fn session_strategy() -> impl Strategy<Value = ChatSession> {
        (
            session_id_strategy(),
            session_id_strategy(),
            0i64..2_000_000_000i64,                   // created_at timestamp
            proptest::collection::vec(message_strategy(), 0..10), // messages
        )
            .prop_map(|(id, workspace_id, ts, messages)| {
                let created_at = DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now());
                let mut session = ChatSession::new(id, workspace_id, created_at);
                for msg in messages {
                    session.add_message(msg);
                }
                session
            })
    }

    proptest! {
        /// Property: Round-trip JSON serialization preserves ChatMessage
        #[test]
        fn prop_message_roundtrip_serialization(msg in message_strategy()) {
            let json = serde_json::to_string(&msg).expect("serialize");
            let deserialized: ChatMessage = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(msg, deserialized);
        }

        /// Property: Round-trip JSON serialization preserves ChatSession
        #[test]
        fn prop_session_roundtrip_serialization(session in session_strategy()) {
            let json = serde_json::to_string(&session).expect("serialize");
            let deserialized: ChatSession = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(session, deserialized);
        }

        /// Property: content_len equals content.len()
        #[test]
        fn prop_content_len_matches(msg in message_strategy()) {
            prop_assert_eq!(msg.content_len(), msg.content.len());
        }

        /// Property: has_agent is true iff agent is Some
        #[test]
        fn prop_has_agent_consistency(msg in message_strategy()) {
            prop_assert_eq!(msg.has_agent(), msg.agent.is_some());
        }

        /// Property: message_count equals messages.len()
        #[test]
        fn prop_message_count_matches(session in session_strategy()) {
            prop_assert_eq!(session.message_count(), session.messages.len());
        }

        /// Property: is_empty is true iff messages is empty
        #[test]
        fn prop_is_empty_consistency(session in session_strategy()) {
            prop_assert_eq!(session.is_empty(), session.messages.is_empty());
        }

        /// Property: user_messages returns only User role messages
        #[test]
        fn prop_user_messages_role(session in session_strategy()) {
            for msg in session.user_messages() {
                prop_assert_eq!(msg.role, MessageRole::User);
            }
        }

        /// Property: assistant_messages returns only Assistant role messages
        #[test]
        fn prop_assistant_messages_role(session in session_strategy()) {
            for msg in session.assistant_messages() {
                prop_assert_eq!(msg.role, MessageRole::Assistant);
            }
        }

        /// Property: user_messages + assistant_messages count <= total messages
        #[test]
        fn prop_message_filter_counts(session in session_strategy()) {
            let user_count = session.user_messages().len();
            let assistant_count = session.assistant_messages().len();
            prop_assert!(user_count + assistant_count <= session.message_count());
        }

        /// Property: MessageRole serializes to lowercase
        #[test]
        fn prop_role_serialization_lowercase(role in role_strategy()) {
            let json = serde_json::to_string(&role).expect("serialize");
            let value = json.trim_matches('"');
            prop_assert_eq!(value, value.to_lowercase());
        }

        /// Property: display_name returns non-empty string
        #[test]
        fn prop_display_name_non_empty(role in role_strategy()) {
            prop_assert!(!role.display_name().is_empty());
        }

        /// Property: with_agent sets agent correctly
        #[test]
        fn prop_with_agent_sets_agent(content in ".*", agent in "@[a-z]+") {
            let ts = Utc::now();
            let msg = ChatMessage::user(content, ts).with_agent(agent.clone());
            prop_assert!(msg.has_agent());
            prop_assert_eq!(msg.agent, Some(agent));
        }

        /// Property: ChatMessage::user creates User role
        #[test]
        fn prop_user_message_has_user_role(content in ".*") {
            let msg = ChatMessage::user(content, Utc::now());
            prop_assert_eq!(msg.role, MessageRole::User);
            prop_assert!(!msg.has_agent());
        }

        /// Property: ChatMessage::assistant creates Assistant role
        #[test]
        fn prop_assistant_message_has_assistant_role(content in ".*") {
            let msg = ChatMessage::assistant(content, Utc::now());
            prop_assert_eq!(msg.role, MessageRole::Assistant);
            prop_assert!(!msg.has_agent());
        }
    }
}
