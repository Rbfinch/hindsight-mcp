//! Integration tests for hindsight-copilot
//!
//! These tests verify parsing of actual Copilot chat session JSON files.

use chrono::Utc;
use hindsight_copilot::session::{
    ChatMessage, ChatSession, MessageRole, default_chat_sessions_dir,
};
use std::path::Path;

/// Get the fixtures directory for test data
fn fixtures_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    Path::new(&manifest_dir).join("tests/fixtures")
}

#[test]
fn test_parse_sample_chat_session() {
    let fixture_path = fixtures_dir().join("chat-session-sample.json");
    let content = std::fs::read_to_string(&fixture_path)
        .expect("Failed to read chat-session-sample.json fixture");

    // Parse as generic JSON to verify structure
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Failed to parse chat session JSON");

    // Verify expected structure
    assert_eq!(json["version"], 3, "Should be version 3");
    assert_eq!(json["responderUsername"], "GitHub Copilot");
    assert_eq!(json["initialLocation"], "panel");

    let requests = json["requests"]
        .as_array()
        .expect("Should have requests array");
    assert_eq!(requests.len(), 2, "Should have 2 requests");

    // Check first request
    let first_request = &requests[0];
    assert!(first_request["requestId"].as_str().is_some());
    assert!(
        first_request["message"]["text"]
            .as_str()
            .unwrap()
            .contains("trait")
    );

    println!("Parsed {} requests from chat session", requests.len());
}

#[test]
fn test_extract_messages_from_chat_session() {
    let fixture_path = fixtures_dir().join("chat-session-sample.json");
    let content = std::fs::read_to_string(&fixture_path)
        .expect("Failed to read chat-session-sample.json fixture");

    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Failed to parse chat session JSON");

    let requests = json["requests"]
        .as_array()
        .expect("Should have requests array");
    let timestamp = Utc::now();

    let mut session = ChatSession::new(
        "test-session-123".to_string(),
        "workspace-456".to_string(),
        timestamp,
    );

    for request in requests {
        // Extract user message
        if let Some(message_text) = request["message"]["text"].as_str() {
            let user_msg = ChatMessage::user(message_text.to_string(), timestamp);
            session.add_message(user_msg);
        }

        // Extract assistant response
        if let Some(responses) = request["response"].as_array() {
            for response in responses {
                if let Some(value) = response["value"].as_str() {
                    let assistant_msg = ChatMessage::assistant(value.to_string(), timestamp);
                    session.add_message(assistant_msg);
                }
            }
        }
    }

    assert_eq!(
        session.message_count(),
        4,
        "Should have 4 messages (2 user + 2 assistant)"
    );
    assert_eq!(
        session.user_messages().len(),
        2,
        "Should have 2 user messages"
    );
    assert_eq!(
        session.assistant_messages().len(),
        2,
        "Should have 2 assistant messages"
    );

    // Verify message content
    let user_msgs = session.user_messages();
    assert!(user_msgs[0].content.contains("trait"));
    assert!(user_msgs[1].content.contains("&str"));

    println!(
        "Extracted {} messages from session",
        session.message_count()
    );
}

#[test]
fn test_extract_variables_from_requests() {
    let fixture_path = fixtures_dir().join("chat-session-sample.json");
    let content = std::fs::read_to_string(&fixture_path)
        .expect("Failed to read chat-session-sample.json fixture");

    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Failed to parse chat session JSON");

    let requests = json["requests"]
        .as_array()
        .expect("Should have requests array");

    let mut files_referenced: Vec<String> = Vec::new();

    for request in requests {
        if let Some(variables) = request["variableData"]["variables"].as_array() {
            for var in variables {
                if var["kind"] == "file" {
                    if let Some(name) = var["name"].as_str() {
                        files_referenced.push(name.to_string());
                    }
                }
            }
        }
    }

    assert_eq!(files_referenced.len(), 1, "Should have 1 file referenced");
    assert_eq!(files_referenced[0], "main.rs");

    println!("Found {} file references", files_referenced.len());
}

#[test]
fn test_chat_session_serialization_roundtrip() {
    let timestamp = Utc::now();

    let mut session = ChatSession::new(
        "session-abc123".to_string(),
        "workspace-xyz789".to_string(),
        timestamp,
    );

    session.add_message(
        ChatMessage::user("What is Rust?".to_string(), timestamp)
            .with_agent("@workspace".to_string()),
    );

    session.add_message(ChatMessage::assistant(
        "Rust is a systems programming language focused on safety and performance.".to_string(),
        timestamp,
    ));

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&session).expect("Failed to serialize session");

    // Verify structure
    assert!(json.contains("\"id\": \"session-abc123\""));
    assert!(json.contains("\"workspace_id\": \"workspace-xyz789\""));
    assert!(json.contains("@workspace"));
    assert!(json.contains("What is Rust?"));

    // Round-trip
    let deserialized: ChatSession =
        serde_json::from_str(&json).expect("Failed to deserialize session");

    assert_eq!(session, deserialized);

    println!("Session serialization roundtrip successful");
}

#[test]
fn test_message_role_serialization() {
    let roles = vec![
        (MessageRole::User, "user"),
        (MessageRole::Assistant, "assistant"),
        (MessageRole::System, "system"),
    ];

    for (role, expected_str) in roles {
        let json = serde_json::to_string(&role).expect("Failed to serialize role");
        assert_eq!(json, format!("\"{}\"", expected_str));

        // Round-trip
        let deserialized: MessageRole =
            serde_json::from_str(&json).expect("Failed to deserialize role");
        assert_eq!(role, deserialized);
    }
}

#[test]
fn test_default_chat_sessions_dir_exists_on_supported_platforms() {
    let path = default_chat_sessions_dir();

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        assert!(
            path.is_some(),
            "Should return a path on supported platforms"
        );
        let p = path.unwrap();

        // Path should contain expected components
        let path_str = p.to_string_lossy();
        assert!(
            path_str.contains("workspaceStorage"),
            "Path should contain workspaceStorage: {}",
            path_str
        );

        // On macOS, should be in Application Support
        #[cfg(target_os = "macos")]
        assert!(
            path_str.contains("Application Support") || path_str.contains("Library"),
            "macOS path should be in Library: {}",
            path_str
        );

        println!("Chat sessions directory: {}", path_str);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        assert!(
            path.is_none(),
            "Should return None on unsupported platforms"
        );
    }
}

#[test]
fn test_discover_real_chat_sessions() {
    // Try to discover real Copilot chat sessions on this system
    let sessions_dir = default_chat_sessions_dir();

    if let Some(dir) = sessions_dir {
        if dir.exists() {
            let mut session_count = 0;

            // Walk the workspaceStorage directory looking for chatSessions
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let chat_dir = entry.path().join("chatSessions");
                    if chat_dir.exists() {
                        if let Ok(files) = std::fs::read_dir(&chat_dir) {
                            for file in files.flatten() {
                                if file
                                    .path()
                                    .extension()
                                    .map(|e| e == "json")
                                    .unwrap_or(false)
                                {
                                    session_count += 1;
                                }
                            }
                        }
                    }
                }
            }

            println!(
                "Found {} chat session files in workspaceStorage",
                session_count
            );
        } else {
            println!(
                "workspaceStorage directory does not exist yet (VS Code may not have been used)"
            );
        }
    } else {
        println!("Skipping: Platform not supported for chat session discovery");
    }
}

#[test]
fn test_message_with_agent_extraction() {
    let timestamp = Utc::now();

    // Simulate extracting agent from request
    let agent_data = serde_json::json!({
        "id": "github.copilot.workspace",
        "name": "@workspace"
    });

    let agent_name = agent_data["name"].as_str().map(String::from);

    let mut msg = ChatMessage::user("Find all Rust files".to_string(), timestamp);
    if let Some(agent) = agent_name {
        msg = msg.with_agent(agent);
    }

    assert!(msg.has_agent());
    assert_eq!(msg.agent, Some("@workspace".to_string()));
}
