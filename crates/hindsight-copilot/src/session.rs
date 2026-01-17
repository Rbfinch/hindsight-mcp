//! Chat session types, discovery, and parsing
//!
//! This module provides:
//! - [`ChatSession`] and [`ChatMessage`] types for representing chat data
//! - [`SessionDiscovery`] for finding VS Code chat session files
//! - [`parse_session_file`] for parsing session JSON into domain types
//! - [`WorkspaceInfo`] for correlating workspaces with their storage IDs

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::CopilotError;

// ============================================================================
// Raw JSON Types (for deserializing VS Code's format)
// ============================================================================

/// Raw session file structure from VS Code chatSessions/*.json
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Fields used for deserialization
struct RawSession {
    version: u32,
    #[serde(default)]
    requester_username: Option<String>,
    #[serde(default)]
    responder_username: Option<String>,
    session_id: String,
    #[serde(default)]
    creation_date: Option<i64>,
    #[serde(default)]
    last_message_date: Option<i64>,
    #[serde(default)]
    requests: Vec<RawRequest>,
    #[serde(default)]
    mode: Option<RawMode>,
    #[serde(default)]
    selected_model: Option<RawSelectedModel>,
}

/// Raw request from the session
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Fields used for deserialization
struct RawRequest {
    request_id: String,
    message: Option<RawMessage>,
    #[serde(default)]
    variable_data: Option<RawVariableData>,
    #[serde(default)]
    response: Vec<RawResponsePart>,
    #[serde(default)]
    agent: Option<RawAgent>,
    timestamp: Option<i64>,
    #[serde(default)]
    model_id: Option<String>,
}

/// Raw message structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields used for deserialization
struct RawMessage {
    text: String,
    #[serde(default)]
    parts: Vec<RawMessagePart>,
}

/// Raw message part
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields used for deserialization
struct RawMessagePart {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    kind: Option<String>,
}

/// Raw variable data (file references, workspace info, etc.)
#[derive(Debug, Clone, Deserialize)]
struct RawVariableData {
    #[serde(default)]
    variables: Vec<RawVariable>,
}

/// Raw variable entry
#[derive(Debug, Clone, Deserialize)]
struct RawVariable {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    // Value can be a string or an object with URI info
    #[serde(default)]
    value: Option<serde_json::Value>,
}

/// Raw response part
#[derive(Debug, Clone, Deserialize)]
struct RawResponsePart {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    value: Option<serde_json::Value>,
}

/// Raw agent info
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Fields used for deserialization
struct RawAgent {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    full_name: Option<String>,
}

/// Raw mode info
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields used for deserialization
struct RawMode {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    kind: Option<String>,
}

/// Raw selected model info
#[derive(Debug, Clone, Deserialize)]
struct RawSelectedModel {
    #[serde(default)]
    identifier: Option<String>,
}

// ============================================================================
// Domain Types
// ============================================================================

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
    /// Model used for this session (e.g., "copilot/claude-opus-4.5")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Session mode (e.g., "agent", "ask")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
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
            model: None,
            mode: None,
        }
    }

    /// Create a session with model and mode information
    #[must_use]
    pub fn with_metadata(
        id: String,
        workspace_id: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        model: Option<String>,
        mode: Option<String>,
    ) -> Self {
        Self {
            id,
            workspace_id,
            created_at,
            updated_at,
            messages: Vec::new(),
            model,
            mode,
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
    /// Variables/attachments referenced in this message
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
}

/// A variable/attachment referenced in a chat message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    /// Variable kind (e.g., "file", "workspace", "promptFile")
    pub kind: String,
    /// Variable name (display name)
    pub name: String,
    /// Variable value (file path, content, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
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
            variables: Vec::new(),
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
            variables: Vec::new(),
        }
    }

    /// Set the agent for this message
    #[must_use]
    pub fn with_agent(mut self, agent: String) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Add variables to this message
    #[must_use]
    pub fn with_variables(mut self, variables: Vec<Variable>) -> Self {
        self.variables = variables;
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

// ============================================================================
// Session Discovery
// ============================================================================

/// Information about a VS Code workspace from workspace.json
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// The workspace storage ID (directory name hash)
    pub storage_id: String,
    /// The workspace folder path (from "folder" field)
    pub folder_path: Option<PathBuf>,
    /// The workspace file path (from "workspace" field, for multi-root)
    pub workspace_file: Option<PathBuf>,
}

/// Raw workspace.json structure
#[derive(Debug, Clone, Deserialize)]
struct RawWorkspaceJson {
    /// Single folder workspace
    #[serde(default)]
    folder: Option<String>,
    /// Multi-root workspace file
    #[serde(default)]
    workspace: Option<String>,
}

impl WorkspaceInfo {
    /// Parse workspace.json from a workspace storage directory
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_storage_dir(storage_dir: &Path) -> Result<Self, CopilotError> {
        let storage_id = storage_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let workspace_json_path = storage_dir.join("workspace.json");
        if !workspace_json_path.exists() {
            return Ok(Self {
                storage_id,
                folder_path: None,
                workspace_file: None,
            });
        }

        let content = fs::read_to_string(&workspace_json_path)?;
        let raw: RawWorkspaceJson = serde_json::from_str(&content)?;

        let folder_path = raw.folder.and_then(|f| parse_file_uri(&f));
        let workspace_file = raw.workspace.and_then(|w| parse_file_uri(&w));

        Ok(Self {
            storage_id,
            folder_path,
            workspace_file,
        })
    }

    /// Get the effective workspace path (folder or workspace file)
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.folder_path
            .as_deref()
            .or(self.workspace_file.as_deref())
    }
}

/// Parse a file:// URI to a PathBuf
fn parse_file_uri(uri: &str) -> Option<PathBuf> {
    if let Some(path) = uri.strip_prefix("file://") {
        // Handle URL-encoded paths
        let decoded = urlencoding_decode(path);
        Some(PathBuf::from(decoded))
    } else {
        // Not a file URI, treat as raw path
        Some(PathBuf::from(uri))
    }
}

/// Simple URL decoding for file paths (handles %20, etc.)
fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }
    result
}

/// Discovered session file with metadata
#[derive(Debug, Clone)]
pub struct DiscoveredSession {
    /// Path to the session JSON file
    pub path: PathBuf,
    /// Session ID (from filename)
    pub session_id: String,
    /// Workspace storage ID
    pub workspace_storage_id: String,
}

/// Session discovery engine for finding VS Code chat sessions
#[derive(Debug)]
pub struct SessionDiscovery {
    /// Root directory for workspace storage
    storage_root: PathBuf,
}

impl SessionDiscovery {
    /// Create a new session discovery using the default storage location
    ///
    /// # Errors
    ///
    /// Returns an error if the default storage directory cannot be determined.
    pub fn new() -> Result<Self, CopilotError> {
        let storage_root =
            default_chat_sessions_dir().ok_or_else(|| CopilotError::WorkspaceStorageNotFound {
                path: "default location not available".to_string(),
            })?;
        Ok(Self { storage_root })
    }

    /// Create a session discovery with a custom storage root
    #[must_use]
    pub fn with_root(storage_root: PathBuf) -> Self {
        Self { storage_root }
    }

    /// Get the storage root path
    #[must_use]
    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    /// Discover all workspace storage directories
    ///
    /// # Errors
    ///
    /// Returns an error if the storage root cannot be read.
    pub fn discover_workspaces(&self) -> Result<Vec<WorkspaceInfo>, CopilotError> {
        if !self.storage_root.exists() {
            return Err(CopilotError::WorkspaceStorageNotFound {
                path: self.storage_root.display().to_string(),
            });
        }

        let mut workspaces = Vec::new();

        for entry in fs::read_dir(&self.storage_root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip hidden directories
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('.'))
                {
                    continue;
                }

                match WorkspaceInfo::from_storage_dir(&path) {
                    Ok(info) => workspaces.push(info),
                    Err(e) => {
                        warn!("Failed to read workspace info from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(workspaces)
    }

    /// Discover all chat session files
    ///
    /// # Errors
    ///
    /// Returns an error if the storage directories cannot be read.
    pub fn discover_sessions(&self) -> Result<Vec<DiscoveredSession>, CopilotError> {
        if !self.storage_root.exists() {
            return Err(CopilotError::WorkspaceStorageNotFound {
                path: self.storage_root.display().to_string(),
            });
        }

        let mut sessions = Vec::new();

        for entry in fs::read_dir(&self.storage_root)? {
            let entry = entry?;
            let workspace_dir = entry.path();

            if !workspace_dir.is_dir() {
                continue;
            }

            let workspace_storage_id = workspace_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip hidden directories
            if workspace_storage_id.starts_with('.') {
                continue;
            }

            let chat_sessions_dir = workspace_dir.join("chatSessions");
            if !chat_sessions_dir.exists() {
                continue;
            }

            match fs::read_dir(&chat_sessions_dir) {
                Ok(entries) => {
                    for session_entry in entries.flatten() {
                        let session_path = session_entry.path();
                        if session_path.extension().is_some_and(|e| e == "json") {
                            let session_id = session_path
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string();

                            sessions.push(DiscoveredSession {
                                path: session_path,
                                session_id,
                                workspace_storage_id: workspace_storage_id.clone(),
                            });
                        }
                    }
                }
                Err(e) => {
                    debug!(
                        "Failed to read chat sessions from {:?}: {}",
                        chat_sessions_dir, e
                    );
                }
            }
        }

        Ok(sessions)
    }

    /// Discover sessions for a specific workspace folder path
    ///
    /// # Errors
    ///
    /// Returns an error if discovery fails.
    pub fn discover_sessions_for_workspace(
        &self,
        workspace_path: &Path,
    ) -> Result<Vec<DiscoveredSession>, CopilotError> {
        let all_sessions = self.discover_sessions()?;
        let workspaces = self.discover_workspaces()?;

        // Find workspace storage IDs that match the given path
        let matching_storage_ids: Vec<_> = workspaces
            .iter()
            .filter(|w| w.path().is_some_and(|p| p == workspace_path))
            .map(|w| &w.storage_id)
            .collect();

        let filtered: Vec<_> = all_sessions
            .into_iter()
            .filter(|s| matching_storage_ids.contains(&&s.workspace_storage_id))
            .collect();

        Ok(filtered)
    }
}

// ============================================================================
// Session Parsing
// ============================================================================

/// Parse a chat session from a JSON file
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn parse_session_file(path: &Path, workspace_id: &str) -> Result<ChatSession, CopilotError> {
    let content = fs::read_to_string(path)?;
    parse_session_json(&content, workspace_id)
}

/// Parse a chat session from JSON content
///
/// # Errors
///
/// Returns an error if the JSON is invalid or doesn't match the expected format.
pub fn parse_session_json(json: &str, workspace_id: &str) -> Result<ChatSession, CopilotError> {
    let raw: RawSession = serde_json::from_str(json)?;

    let created_at = raw
        .creation_date
        .and_then(DateTime::from_timestamp_millis)
        .unwrap_or_else(Utc::now);

    let updated_at = raw
        .last_message_date
        .and_then(DateTime::from_timestamp_millis)
        .unwrap_or(created_at);

    let model = raw.selected_model.and_then(|m| m.identifier);
    let mode = raw.mode.and_then(|m| m.id);

    let mut session = ChatSession::with_metadata(
        raw.session_id,
        workspace_id.to_string(),
        created_at,
        updated_at,
        model,
        mode,
    );

    // Parse each request/response pair
    for request in raw.requests {
        // Extract user message
        if let Some(msg) = &request.message {
            let timestamp = request
                .timestamp
                .and_then(DateTime::from_timestamp_millis)
                .unwrap_or(created_at);

            let variables = extract_variables(&request.variable_data);

            let agent_name = request
                .agent
                .as_ref()
                .and_then(|a| a.name.clone().or(a.full_name.clone()));

            let user_msg = ChatMessage::user(msg.text.clone(), timestamp).with_variables(variables);

            let user_msg = if let Some(agent) = agent_name.clone() {
                user_msg.with_agent(agent)
            } else {
                user_msg
            };

            session.add_message(user_msg);
        }

        // Extract assistant response
        let response_text = extract_response_text(&request.response);
        if !response_text.is_empty() {
            let timestamp = request
                .timestamp
                .and_then(DateTime::from_timestamp_millis)
                .unwrap_or(created_at);

            let agent_name = request
                .agent
                .as_ref()
                .and_then(|a| a.name.clone().or(a.full_name.clone()));

            let assistant_msg = ChatMessage::assistant(response_text, timestamp);
            let assistant_msg = if let Some(agent) = agent_name {
                assistant_msg.with_agent(agent)
            } else {
                assistant_msg
            };

            session.add_message(assistant_msg);
        }
    }

    Ok(session)
}

/// Extract variables from raw variable data
fn extract_variables(variable_data: &Option<RawVariableData>) -> Vec<Variable> {
    let Some(data) = variable_data else {
        return Vec::new();
    };

    data.variables
        .iter()
        .filter_map(|v| {
            let kind = v.kind.clone().unwrap_or_else(|| "unknown".to_string());
            let name = v.name.clone().unwrap_or_else(|| "unnamed".to_string());

            // Skip prompt instructions (they're internal)
            if kind == "promptText" || name.starts_with("prompt:instructions") {
                return None;
            }

            // Extract value as string
            let value = match &v.value {
                Some(serde_json::Value::String(s)) => Some(s.clone()),
                Some(serde_json::Value::Object(obj)) => {
                    // Try to extract path from URI object
                    obj.get("path")
                        .and_then(|p| p.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            obj.get("external")
                                .and_then(|e| e.as_str())
                                .map(|s| s.to_string())
                        })
                }
                _ => v.id.clone(),
            };

            Some(Variable { kind, name, value })
        })
        .collect()
}

/// Extract response text from raw response parts
fn extract_response_text(response_parts: &[RawResponsePart]) -> String {
    let mut text_parts = Vec::new();

    for part in response_parts {
        match part.kind.as_deref() {
            Some("thinking") => {
                // Include thinking content if it has meaningful text
                if let Some(serde_json::Value::String(s)) = &part.value {
                    if !s.is_empty() && s.len() < 500 {
                        // Skip encrypted/encoded thinking
                        text_parts.push(s.clone());
                    }
                }
            }
            Some("textEditGroup") | Some("codeblockUri") | Some("prepareToolInvocation") => {
                // Skip these - they're tool-related, not text
            }
            _ => {
                // Default: try to extract text from value
                if let Some(serde_json::Value::String(s)) = &part.value {
                    text_parts.push(s.clone());
                } else if let Some(serde_json::Value::Object(obj)) = &part.value {
                    if let Some(serde_json::Value::String(s)) = obj.get("value") {
                        text_parts.push(s.clone());
                    }
                }
            }
        }
    }

    text_parts.join("")
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

    // ========================================================================
    // Session parsing tests
    // ========================================================================

    #[test]
    fn test_parse_session_json_empty() {
        let json = r#"{
            "version": 3,
            "sessionId": "test-session-id",
            "creationDate": 1705500000000,
            "lastMessageDate": 1705500001000,
            "requests": []
        }"#;

        let session = parse_session_json(json, "workspace-123").expect("parse");
        assert_eq!(session.id, "test-session-id");
        assert_eq!(session.workspace_id, "workspace-123");
        assert!(session.is_empty());
    }

    #[test]
    fn test_parse_session_json_with_request() {
        let json = r#"{
            "version": 3,
            "sessionId": "session-with-request",
            "creationDate": 1705500000000,
            "lastMessageDate": 1705500001000,
            "requests": [
                {
                    "requestId": "request-1",
                    "message": {
                        "text": "Hello, Copilot!",
                        "parts": []
                    },
                    "timestamp": 1705500000500,
                    "response": [
                        {
                            "value": "Hello! How can I help you?",
                            "supportThemeIcons": false
                        }
                    ]
                }
            ]
        }"#;

        let session = parse_session_json(json, "ws").expect("parse");
        assert_eq!(session.message_count(), 2);

        let user_msgs = session.user_messages();
        assert_eq!(user_msgs.len(), 1);
        assert_eq!(user_msgs[0].content, "Hello, Copilot!");

        let assistant_msgs = session.assistant_messages();
        assert_eq!(assistant_msgs.len(), 1);
        assert_eq!(assistant_msgs[0].content, "Hello! How can I help you?");
    }

    #[test]
    fn test_parse_session_json_with_model() {
        let json = r#"{
            "version": 3,
            "sessionId": "session-with-model",
            "creationDate": 1705500000000,
            "lastMessageDate": 1705500001000,
            "requests": [],
            "selectedModel": {
                "identifier": "copilot/claude-opus-4.5"
            },
            "mode": {
                "id": "agent",
                "kind": "agent"
            }
        }"#;

        let session = parse_session_json(json, "ws").expect("parse");
        assert_eq!(session.model, Some("copilot/claude-opus-4.5".to_string()));
        assert_eq!(session.mode, Some("agent".to_string()));
    }

    #[test]
    fn test_parse_session_json_with_variables() {
        let json = r#"{
            "version": 3,
            "sessionId": "session-with-vars",
            "creationDate": 1705500000000,
            "lastMessageDate": 1705500001000,
            "requests": [
                {
                    "requestId": "request-1",
                    "message": {
                        "text": "Check this file",
                        "parts": []
                    },
                    "variableData": {
                        "variables": [
                            {
                                "kind": "file",
                                "name": "main.rs",
                                "value": {
                                    "path": "/project/src/main.rs",
                                    "scheme": "file"
                                }
                            },
                            {
                                "kind": "workspace",
                                "name": "myproject",
                                "value": "Repository info"
                            }
                        ]
                    },
                    "timestamp": 1705500000500,
                    "response": []
                }
            ]
        }"#;

        let session = parse_session_json(json, "ws").expect("parse");
        assert_eq!(session.message_count(), 1);

        let msg = &session.messages[0];
        assert_eq!(msg.variables.len(), 2);
        assert_eq!(msg.variables[0].kind, "file");
        assert_eq!(msg.variables[0].name, "main.rs");
        assert_eq!(
            msg.variables[0].value,
            Some("/project/src/main.rs".to_string())
        );
    }

    #[test]
    fn test_parse_file_uri() {
        assert_eq!(
            parse_file_uri("file:///Users/test/project"),
            Some(PathBuf::from("/Users/test/project"))
        );
        assert_eq!(
            parse_file_uri("file:///path/with%20spaces"),
            Some(PathBuf::from("/path/with spaces"))
        );
        assert_eq!(
            parse_file_uri("/raw/path"),
            Some(PathBuf::from("/raw/path"))
        );
    }

    #[test]
    fn test_urlencoding_decode() {
        assert_eq!(urlencoding_decode("hello%20world"), "hello world");
        assert_eq!(urlencoding_decode("no%2fslash"), "no/slash");
        assert_eq!(urlencoding_decode("plain"), "plain");
        assert_eq!(urlencoding_decode("%2F%2F"), "//");
    }

    #[test]
    fn test_variable_serialization() {
        let var = Variable {
            kind: "file".to_string(),
            name: "test.rs".to_string(),
            value: Some("/path/to/test.rs".to_string()),
        };
        let json = serde_json::to_string(&var).expect("serialize");
        let deserialized: Variable = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(var, deserialized);
    }

    #[test]
    fn test_message_with_variables() {
        let vars = vec![Variable {
            kind: "file".to_string(),
            name: "lib.rs".to_string(),
            value: Some("/src/lib.rs".to_string()),
        }];

        let msg = ChatMessage::user("Test".to_string(), sample_timestamp()).with_variables(vars);

        assert_eq!(msg.variables.len(), 1);
        assert_eq!(msg.variables[0].name, "lib.rs");
    }

    #[test]
    fn test_session_with_metadata() {
        let ts = sample_timestamp();
        let session = ChatSession::with_metadata(
            "id".to_string(),
            "ws".to_string(),
            ts,
            ts,
            Some("gpt-4".to_string()),
            Some("ask".to_string()),
        );

        assert_eq!(session.model, Some("gpt-4".to_string()));
        assert_eq!(session.mode, Some("ask".to_string()));
    }

    #[test]
    fn test_workspace_info_path() {
        let info = WorkspaceInfo {
            storage_id: "abc123".to_string(),
            folder_path: Some(PathBuf::from("/project")),
            workspace_file: None,
        };
        assert_eq!(info.path(), Some(Path::new("/project")));

        let info2 = WorkspaceInfo {
            storage_id: "xyz789".to_string(),
            folder_path: None,
            workspace_file: Some(PathBuf::from("/multi.code-workspace")),
        };
        assert_eq!(info2.path(), Some(Path::new("/multi.code-workspace")));

        let info3 = WorkspaceInfo {
            storage_id: "empty".to_string(),
            folder_path: None,
            workspace_file: None,
        };
        assert!(info3.path().is_none());
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

    /// Strategy to generate a Variable
    fn variable_strategy() -> impl Strategy<Value = Variable> {
        (
            prop_oneof![Just("file"), Just("workspace"), Just("selection")],
            "[a-z._-]{1,20}",
            proptest::option::of("[a-z/._-]{1,50}"),
        )
            .prop_map(|(kind, name, value)| Variable {
                kind: kind.to_string(),
                name,
                value,
            })
    }

    /// Strategy to generate arbitrary ChatMessage values
    fn message_strategy() -> impl Strategy<Value = ChatMessage> {
        (
            role_strategy(),
            ".*",                                                 // content
            0i64..2_000_000_000i64,                               // timestamp as unix seconds
            proptest::option::of("@[a-z]+"),                      // agent
            proptest::collection::vec(variable_strategy(), 0..3), // variables
        )
            .prop_map(|(role, content, ts, agent, variables)| {
                let timestamp = DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
                ChatMessage {
                    role,
                    content,
                    timestamp,
                    agent,
                    variables,
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
            0i64..2_000_000_000i64, // created_at timestamp
            proptest::collection::vec(message_strategy(), 0..10), // messages
        )
            .prop_map(|(id, workspace_id, ts, messages)| {
                let created_at = DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
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
