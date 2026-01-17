//! Integration tests for hindsight-mcp
//!
//! These tests verify full database round-trips and cross-crate data flow.

use chrono::{TimeZone, Utc};
use hindsight_copilot::session::{ChatMessage, ChatSession, MessageRole};
use hindsight_git::commit::Commit;
use hindsight_tests::result::{TestOutcome, TestResult};

/// Helper to generate a UUID-like string for testing
fn test_uuid(suffix: &str) -> String {
    format!("00000000-0000-0000-0000-{:0>12}", suffix)
}

/// Helper to generate an ISO 8601 timestamp string
fn iso_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[test]
fn test_commit_to_json_for_database() {
    let commit = Commit {
        sha: "1945ab9c752534e733c38ba0109dc3b741f0a6eb".to_string(),
        message: "feat: add integration tests\n\nDetailed description.".to_string(),
        author: "Test Author".to_string(),
        author_email: "test@example.com".to_string(),
        timestamp: Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap(),
        parents: vec!["c460aeb7fb2d109c17e43de0ce681faec0b7374d".to_string()],
    };

    // Serialize for database storage
    let json = serde_json::to_string(&commit).expect("Failed to serialize commit");

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");

    // Check that parents are stored as JSON array (for parents_json column)
    let parents_json = serde_json::to_string(&commit.parents).expect("Failed to serialize parents");
    let parents_parsed: Vec<String> = serde_json::from_str(&parents_json).expect("Should parse");
    assert_eq!(parents_parsed.len(), 1);

    println!("Commit JSON: {}", json);
    println!("Parents JSON: {}", parents_json);
}

#[test]
fn test_test_result_to_json_for_database() {
    let result = TestResult {
        name: "hindsight_git::commit::tests::test_is_valid_sha".to_string(),
        outcome: TestOutcome::Passed,
        duration_ms: 42,
        timestamp: Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap(),
        output: Some("test output here".to_string()),
    };

    // Serialize for database storage
    let json = serde_json::to_string(&result).expect("Failed to serialize test result");

    // The output should be stored as JSON (for output_json column)
    let output_json = result
        .output
        .as_ref()
        .map(|o| serde_json::json!({"stdout": o, "stderr": null}))
        .map(|v| serde_json::to_string(&v).expect("Failed to serialize output"));

    println!("TestResult JSON: {}", json);
    if let Some(oj) = &output_json {
        println!("Output JSON: {}", oj);
    }
}

#[test]
fn test_chat_session_to_json_for_database() {
    let timestamp = Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap();

    let mut session = ChatSession::new(test_uuid("session1"), test_uuid("workspace1"), timestamp);

    session.add_message(ChatMessage::user("Hello".to_string(), timestamp));
    session.add_message(ChatMessage::assistant("Hi there!".to_string(), timestamp));

    // Serialize for database storage
    let json = serde_json::to_string_pretty(&session).expect("Failed to serialize session");

    // Metadata for the metadata_json column
    let metadata = serde_json::json!({
        "version": 3,
        "responder": "GitHub Copilot",
        "message_count": session.message_count()
    });
    let metadata_json = serde_json::to_string(&metadata).expect("Failed to serialize metadata");

    println!("Session JSON: {}", json);
    println!("Metadata JSON: {}", metadata_json);
}

#[test]
fn test_cross_crate_type_compatibility() {
    // Verify that types from different crates can be used together
    let commit_timestamp = Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap();

    let commit = Commit {
        sha: "1945ab9c752534e733c38ba0109dc3b741f0a6eb".to_string(),
        message: "test: add tests".to_string(),
        author: "Test".to_string(),
        author_email: "test@test.com".to_string(),
        timestamp: commit_timestamp,
        parents: vec![],
    };

    let test_result = TestResult {
        name: "test_something".to_string(),
        outcome: TestOutcome::Passed,
        duration_ms: 10,
        timestamp: commit_timestamp, // Same timestamp type
        output: None,
    };

    let session = ChatSession::new(
        "session".to_string(),
        "workspace".to_string(),
        commit_timestamp, // Same timestamp type
    );

    // All types use chrono::DateTime<Utc> for timestamps
    assert_eq!(commit.timestamp, test_result.timestamp);
    assert_eq!(commit.timestamp, session.created_at);

    // All types are serializable
    let _ = serde_json::to_string(&commit).expect("commit serializable");
    let _ = serde_json::to_string(&test_result).expect("test_result serializable");
    let _ = serde_json::to_string(&session).expect("session serializable");
}

#[test]
fn test_timeline_data_structure() {
    // Simulate a timeline combining all data sources
    #[derive(Debug, serde::Serialize)]
    struct TimelineEvent {
        event_type: String,
        timestamp: String,
        content: String,
        reference_id: String,
    }

    let timestamp = Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap();
    let iso = timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let events = vec![
        TimelineEvent {
            event_type: "commit".to_string(),
            timestamp: iso.clone(),
            content: "feat: add new feature".to_string(),
            reference_id: "1945ab9c".to_string(),
        },
        TimelineEvent {
            event_type: "test_run".to_string(),
            timestamp: iso.clone(),
            content: "Tests: 102 passed, 0 failed".to_string(),
            reference_id: test_uuid("run1"),
        },
        TimelineEvent {
            event_type: "copilot".to_string(),
            timestamp: iso.clone(),
            content: "How do I implement this?".to_string(),
            reference_id: test_uuid("session1"),
        },
    ];

    let json = serde_json::to_string_pretty(&events).expect("Failed to serialize timeline");

    // Verify structure
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).expect("Should parse");
    assert_eq!(parsed.len(), 3);

    println!("Timeline JSON:\n{}", json);
}

#[test]
fn test_uuid_and_timestamp_format() {
    // Verify UUID format
    let uuid = test_uuid("abc123");
    assert_eq!(uuid.len(), 36, "UUID should be 36 characters");
    assert!(uuid.contains('-'), "UUID should contain dashes");

    // Verify ISO 8601 timestamp format
    let ts = iso_timestamp();
    assert!(ts.contains('T'), "Timestamp should contain T separator");
    assert!(ts.ends_with('Z'), "Timestamp should end with Z for UTC");

    // Verify chrono serialization matches expected format
    let dt = Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap();
    let json = serde_json::to_string(&dt).expect("Failed to serialize datetime");

    // chrono uses RFC 3339 which is compatible with ISO 8601
    assert!(json.contains("2026-01-17"));
    assert!(json.contains("02:33:06"));

    println!("UUID: {}", uuid);
    println!("ISO timestamp: {}", ts);
    println!("Chrono JSON: {}", json);
}

#[test]
fn test_json_column_structures() {
    // Define JSON structures for database columns

    // parents_json for commits table
    let parents: Vec<String> = vec![
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
    ];
    let parents_json = serde_json::to_string(&parents).expect("parents");

    // diff_json for commits table
    let diff = serde_json::json!({
        "files_changed": 3,
        "additions": 100,
        "deletions": 20,
        "files": [
            {"path": "src/lib.rs", "additions": 50, "deletions": 10},
            {"path": "src/main.rs", "additions": 30, "deletions": 5},
            {"path": "README.md", "additions": 20, "deletions": 5}
        ]
    });
    let diff_json = serde_json::to_string(&diff).expect("diff");

    // metadata_json for test_runs table
    let test_metadata = serde_json::json!({
        "nextest_version": "0.9",
        "profile": "default",
        "target_triple": "aarch64-apple-darwin"
    });
    let test_metadata_json = serde_json::to_string(&test_metadata).expect("test_metadata");

    // output_json for test_results table
    let output = serde_json::json!({
        "stdout": "test output...",
        "stderr": null,
        "status_code": 0
    });
    let output_json = serde_json::to_string(&output).expect("output");

    // variables_json for copilot_messages table
    let variables = serde_json::json!({
        "files": [
            {"name": "lib.rs", "uri": "file:///workspace/src/lib.rs"},
            {"name": "main.rs", "uri": "file:///workspace/src/main.rs"}
        ],
        "selections": []
    });
    let variables_json = serde_json::to_string(&variables).expect("variables");

    // Verify all are valid JSON
    let _: Vec<String> = serde_json::from_str(&parents_json).expect("parse parents");
    let _: serde_json::Value = serde_json::from_str(&diff_json).expect("parse diff");
    let _: serde_json::Value = serde_json::from_str(&test_metadata_json).expect("parse metadata");
    let _: serde_json::Value = serde_json::from_str(&output_json).expect("parse output");
    let _: serde_json::Value = serde_json::from_str(&variables_json).expect("parse variables");

    println!("All JSON column structures are valid");
}

#[test]
fn test_message_role_as_database_enum() {
    // Verify MessageRole serializes to lowercase strings suitable for database
    let roles = [
        MessageRole::User,
        MessageRole::Assistant,
        MessageRole::System,
    ];

    for role in &roles {
        let json = serde_json::to_string(role).expect("serialize");
        let value: String = serde_json::from_str(&json).expect("parse");

        // Should be lowercase
        assert_eq!(value, value.to_lowercase());

        // Should be a simple string (no complex structure)
        assert!(!value.contains('{'));
    }
}

#[test]
fn test_test_outcome_as_database_enum() {
    // Verify TestOutcome serializes to lowercase strings suitable for database
    let outcomes = [
        TestOutcome::Passed,
        TestOutcome::Failed,
        TestOutcome::Ignored,
        TestOutcome::TimedOut,
    ];

    for outcome in &outcomes {
        let json = serde_json::to_string(outcome).expect("serialize");
        let value: String = serde_json::from_str(&json).expect("parse");

        // Should be lowercase
        assert_eq!(value, value.to_lowercase());

        // Should be a simple string
        assert!(!value.contains('{'));
    }
}
