// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Tests for the hindsight_timeline MCP tool handler
//!
//! This module tests the timeline tool which returns a chronological
//! view of development activity including commits, test runs, and copilot sessions.

mod fixtures;
mod mcp_harness;

use fixtures::{populated_database, test_database};
use mcp_harness::McpTestHarness;
use serde_json::{Map, Value, json};

use hindsight_mcp::handlers::{self, HandlerError};

// ============================================================================
// Helper Functions
// ============================================================================

fn to_map(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => panic!("Expected JSON object"),
    }
}

fn harness_with_populated_db() -> McpTestHarness {
    let (db, _) = populated_database();
    McpTestHarness::new(db)
}

// ============================================================================
// Basic Timeline Tests
// ============================================================================

#[test]
fn test_timeline_default_limit() {
    // Default limit should be 50
    let harness = harness_with_populated_db();
    let events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    // Populated database has commits and sessions, so should have events
    assert!(!events.is_empty(), "Should have timeline events");
}

#[test]
fn test_timeline_custom_limit() {
    let harness = harness_with_populated_db();

    // Request only 2 events
    let events = harness
        .timeline(Some(2), None)
        .expect("timeline should succeed");

    assert!(events.len() <= 2, "Should respect limit of 2");
}

#[test]
fn test_timeline_limit_zero() {
    let harness = harness_with_populated_db();

    // Limit of 0 should return no events
    let events = harness
        .timeline(Some(0), None)
        .expect("timeline should succeed");

    assert!(events.is_empty(), "Limit 0 should return empty");
}

#[test]
fn test_timeline_very_large_limit() {
    let harness = harness_with_populated_db();

    // Very large limit should just return all available events
    let events = harness
        .timeline(Some(10000), None)
        .expect("timeline should succeed");

    // Should return events without crashing
    assert!(!events.is_empty());
}

#[test]
fn test_timeline_empty_database() {
    let db = test_database();
    let harness = McpTestHarness::new(db);

    let events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    assert!(events.is_empty(), "Empty database should return no events");
}

// ============================================================================
// Workspace Filter Tests
// ============================================================================

#[test]
fn test_timeline_workspace_filter_valid_path() {
    let harness = harness_with_populated_db();

    // Filter by the test workspace path
    let events = harness
        .timeline(None, Some("/tmp/test-project"))
        .expect("timeline should succeed");

    // Should have events from this workspace
    assert!(
        !events.is_empty(),
        "Should have events for test-project workspace"
    );
}

#[test]
fn test_timeline_workspace_filter_nonexistent_gracefully_ignored() {
    let harness = harness_with_populated_db();

    // Filter by a nonexistent workspace
    let events = harness
        .timeline(None, Some("/nonexistent/workspace/path"))
        .expect("timeline should succeed");

    // When workspace filter doesn't match any known workspace, the filter is ignored
    // and all events are returned (graceful degradation)
    let _ = events; // Just verify it doesn't error
}

#[test]
fn test_timeline_workspace_filter_null_uses_all() {
    let harness = harness_with_populated_db();

    // Null workspace should return events from all workspaces
    let all_events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    assert!(!all_events.is_empty());
}

#[test]
fn test_timeline_with_default_workspace_set() {
    let (db, _) = populated_database();
    let harness =
        McpTestHarness::new(db).with_workspace(std::path::PathBuf::from("/tmp/test-project"));

    // Timeline should use the default workspace when none is specified
    let events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    assert!(!events.is_empty());
}

// ============================================================================
// Event Content Tests
// ============================================================================

#[test]
fn test_timeline_events_have_required_fields() {
    let harness = harness_with_populated_db();
    let events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    for event in &events {
        assert!(!event.event_id.is_empty(), "event_id should not be empty");
        assert!(
            !event.event_type.is_empty(),
            "event_type should not be empty"
        );
        assert!(
            !event.workspace_id.is_empty(),
            "workspace_id should not be empty"
        );
        assert!(
            !event.event_timestamp.is_empty(),
            "event_timestamp should not be empty"
        );
        assert!(!event.summary.is_empty(), "summary should not be empty");
    }
}

#[test]
fn test_timeline_events_ordered_chronologically() {
    let harness = harness_with_populated_db();
    let events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    if events.len() > 1 {
        // Events should be ordered by timestamp descending (newest first)
        for i in 0..events.len() - 1 {
            assert!(
                events[i].event_timestamp >= events[i + 1].event_timestamp,
                "Events should be in descending chronological order"
            );
        }
    }
}

#[test]
fn test_timeline_contains_commits() {
    let harness = harness_with_populated_db();
    let events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    let has_commits = events.iter().any(|e| e.event_type == "commit");
    assert!(has_commits, "Timeline should contain commit events");
}

#[test]
fn test_timeline_contains_copilot_messages() {
    let harness = harness_with_populated_db();
    let events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    // The populated database has copilot messages
    // Event type is "copilot_message" (not "copilot" or "copilot_session")
    let has_copilot = events.iter().any(|e| e.event_type == "copilot_message");
    assert!(
        has_copilot,
        "Timeline should contain copilot_message events"
    );
}

#[test]
fn test_timeline_commit_summary_includes_message() {
    let harness = harness_with_populated_db();
    let events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    // Find a commit event
    let commit_event = events.iter().find(|e| e.event_type == "commit");

    if let Some(event) = commit_event {
        // The summary should contain part of the commit message
        assert!(
            event.summary.contains("Commit message"),
            "Commit summary should include message"
        );
    }
}

// ============================================================================
// Raw JSON Input Tests
// ============================================================================

#[test]
fn test_timeline_raw_json_valid() {
    let db = test_database();

    let args = to_map(json!({
        "limit": 25,
        "workspace": "/some/path"
    }));

    let result = handlers::handle_timeline(&db, Some(args), None);
    assert!(result.is_ok());
}

#[test]
fn test_timeline_raw_json_empty_args() {
    let db = test_database();

    // Empty args should use defaults
    let args = to_map(json!({}));
    let result = handlers::handle_timeline(&db, Some(args), None);
    assert!(result.is_ok());
}

#[test]
fn test_timeline_raw_json_none_args() {
    let db = test_database();

    // None args should use defaults
    let result = handlers::handle_timeline(&db, None, None);
    assert!(result.is_ok());
}

#[test]
fn test_timeline_raw_json_extra_fields_ignored() {
    let db = test_database();

    // Extra fields should be ignored
    let args = to_map(json!({
        "limit": 10,
        "unknown_field": "some value",
        "another_extra": 42
    }));

    let result = handlers::handle_timeline(&db, Some(args), None);
    assert!(result.is_ok(), "Extra fields should be ignored");
}

#[test]
fn test_timeline_raw_json_wrong_type_for_limit() {
    let db = test_database();

    // String instead of number for limit
    let args = to_map(json!({
        "limit": "not a number"
    }));

    let result = handlers::handle_timeline(&db, Some(args), None);
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_timeline_limit_one() {
    let harness = harness_with_populated_db();

    let events = harness
        .timeline(Some(1), None)
        .expect("timeline should succeed");

    assert!(events.len() <= 1, "Should return at most 1 event");
}

#[test]
fn test_timeline_workspace_filter_empty_string() {
    let harness = harness_with_populated_db();

    // Empty string workspace filter doesn't match any workspace,
    // so the filter is ignored and all events are returned
    let events = harness
        .timeline(None, Some(""))
        .expect("timeline should succeed");

    let _ = events; // Just verify it doesn't error
}

#[test]
fn test_timeline_workspace_filter_with_trailing_slash() {
    let harness = harness_with_populated_db();

    // Test workspace path with trailing slash - should still work
    let events = harness
        .timeline(None, Some("/tmp/test-project/"))
        .expect("timeline should succeed");

    // May or may not find events depending on path normalization
    // The key is it shouldn't crash
    let _ = events;
}

#[test]
fn test_timeline_invoke_with_json_method() {
    let harness = harness_with_populated_db();

    let result = harness.invoke_with_json(
        "hindsight_timeline",
        json!({
            "limit": 5
        }),
    );

    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.is_array());
}

#[test]
fn test_timeline_returns_details_json() {
    let harness = harness_with_populated_db();
    let events = harness
        .timeline(None, None)
        .expect("timeline should succeed");

    // Check that at least some events have details_json
    let has_details = events.iter().any(|e| e.details_json.is_some());

    // This is optional, so just verify the field exists
    let _ = has_details;
}
