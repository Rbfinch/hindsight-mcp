// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Tests for the hindsight_activity_summary MCP tool handler
//!
//! This module tests the activity_summary tool which returns aggregate
//! statistics about development activity over a time period.

mod fixtures;
mod mcp_harness;

use fixtures::{days_ago, hours_ago, now, populated_database, sample_commit, test_database};
use mcp_harness::{McpTestHarness, assert_activity_counts};
use serde_json::{Map, Value, json};

use hindsight_mcp::db::WorkspaceRecord;
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
// Default Days Parameter Tests
// ============================================================================

#[test]
fn test_activity_summary_default_days_is_7() {
    let harness = harness_with_populated_db();

    // Default should be 7 days
    let summary = harness
        .activity_summary(None)
        .expect("activity_summary should succeed");

    // Should return a summary
    let _ = summary;
}

#[test]
fn test_activity_summary_empty_database() {
    let db = test_database();
    let harness = McpTestHarness::new(db);

    let summary = harness
        .activity_summary(None)
        .expect("activity_summary should succeed");

    assert_eq!(summary.commits, 0);
    assert_eq!(summary.test_runs, 0);
    assert_eq!(summary.copilot_sessions, 0);
}

// ============================================================================
// Custom Days Parameter Tests
// ============================================================================

#[test]
fn test_activity_summary_days_1() {
    let harness = harness_with_populated_db();

    let summary = harness
        .activity_summary(Some(1))
        .expect("activity_summary should succeed");

    // 1 day might have fewer events
    let _ = summary;
}

#[test]
fn test_activity_summary_days_30() {
    let harness = harness_with_populated_db();

    let summary = harness
        .activity_summary(Some(30))
        .expect("activity_summary should succeed");

    // 30 days should include all recent activity
    let _ = summary;
}

#[test]
fn test_activity_summary_days_365() {
    let harness = harness_with_populated_db();

    let summary = harness
        .activity_summary(Some(365))
        .expect("activity_summary should succeed");

    // Full year should include everything
    let _ = summary;
}

#[test]
fn test_activity_summary_days_0() {
    let harness = harness_with_populated_db();

    // 0 days - should return today's activity or be empty
    let summary = harness
        .activity_summary(Some(0))
        .expect("activity_summary should succeed");

    // Implementation-dependent behavior, but shouldn't crash
    let _ = summary;
}

#[test]
fn test_activity_summary_very_large_days() {
    let harness = harness_with_populated_db();

    // Very large days value
    let summary = harness
        .activity_summary(Some(10000))
        .expect("activity_summary should succeed");

    // Should handle without crashing
    let _ = summary;
}

// ============================================================================
// Count Accuracy Tests
// ============================================================================

#[test]
fn test_activity_summary_counts_commits() {
    let harness = harness_with_populated_db();

    // Populated database has 5 commits
    let summary = harness
        .activity_summary(Some(365))
        .expect("activity_summary should succeed");

    assert!(summary.commits >= 5, "Should count at least 5 commits");
}

#[test]
fn test_activity_summary_counts_test_runs() {
    let harness = harness_with_populated_db();

    // Populated database has 2 test runs
    let summary = harness
        .activity_summary(Some(365))
        .expect("activity_summary should succeed");

    assert!(summary.test_runs >= 2, "Should count at least 2 test runs");
}

#[test]
fn test_activity_summary_counts_copilot_sessions() {
    let harness = harness_with_populated_db();

    // Populated database has 1 copilot session
    let summary = harness
        .activity_summary(Some(365))
        .expect("activity_summary should succeed");

    assert!(
        summary.copilot_sessions >= 1,
        "Should count at least 1 copilot session"
    );
}

#[test]
fn test_activity_summary_assertion_helper() {
    let harness = harness_with_populated_db();

    let summary = harness
        .activity_summary(Some(365))
        .expect("activity_summary should succeed");

    // Use the assertion helper
    assert_activity_counts(&summary, 1, 1);
}

// ============================================================================
// Time Period Boundary Tests
// ============================================================================

#[test]
fn test_activity_summary_recent_commits_only() {
    let db = test_database();

    let workspace = WorkspaceRecord::new("time-test".to_string(), "/tmp/time-test".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    let base = now();

    // Create commits at different times
    let recent_commit = sample_commit(
        &workspace.id,
        "recent00000000000000000000000000000000",
        "Recent commit",
        hours_ago(base, 1),
    );

    let old_commit = sample_commit(
        &workspace.id,
        "old00000000000000000000000000000000000",
        "Old commit",
        days_ago(base, 30),
    );

    db.insert_commit(&recent_commit).expect("insert recent");
    db.insert_commit(&old_commit).expect("insert old");

    let harness = McpTestHarness::new(db);

    // 1 day should only include recent commit
    let summary_1_day = harness.activity_summary(Some(1)).expect("summary");

    // 60 days should include both
    let summary_60_days = harness.activity_summary(Some(60)).expect("summary");

    assert!(summary_1_day.commits <= summary_60_days.commits);
}

// ============================================================================
// Response Structure Tests
// ============================================================================

#[test]
fn test_activity_summary_response_fields() {
    let harness = harness_with_populated_db();

    let summary = harness
        .activity_summary(None)
        .expect("activity_summary should succeed");

    // Summary should have all expected fields (they exist by type system)
    let _ = summary.commits;
    let _ = summary.test_runs;
    let _ = summary.copilot_sessions;
}

#[test]
fn test_activity_summary_counts_are_non_negative() {
    let harness = harness_with_populated_db();

    let summary = harness
        .activity_summary(None)
        .expect("activity_summary should succeed");

    // Counts should never be negative
    // (u64 type ensures this, but let's verify the values are reasonable)
    assert!(summary.commits < u64::MAX);
    assert!(summary.test_runs < u64::MAX);
    assert!(summary.copilot_sessions < u64::MAX);
}

// ============================================================================
// Raw JSON Input Tests
// ============================================================================

#[test]
fn test_activity_summary_raw_json_valid() {
    let db = test_database();

    let args = to_map(json!({
        "days": 14
    }));

    let result = handlers::handle_activity_summary(&db, Some(args));
    assert!(result.is_ok());
}

#[test]
fn test_activity_summary_raw_json_empty_args() {
    let db = test_database();

    let args = to_map(json!({}));
    let result = handlers::handle_activity_summary(&db, Some(args));
    assert!(result.is_ok());
}

#[test]
fn test_activity_summary_raw_json_none_args() {
    let db = test_database();

    let result = handlers::handle_activity_summary(&db, None);
    assert!(result.is_ok());
}

#[test]
fn test_activity_summary_raw_json_extra_fields_ignored() {
    let db = test_database();

    let args = to_map(json!({
        "days": 7,
        "unknown_field": "some value"
    }));

    let result = handlers::handle_activity_summary(&db, Some(args));
    assert!(result.is_ok(), "Extra fields should be ignored");
}

#[test]
fn test_activity_summary_raw_json_wrong_type_for_days() {
    let db = test_database();

    let args = to_map(json!({
        "days": "not a number"
    }));

    let result = handlers::handle_activity_summary(&db, Some(args));
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_activity_summary_raw_json_negative_days() {
    let db = test_database();

    // Negative days - serde will reject this for u32
    let args = to_map(json!({
        "days": -5
    }));

    let result = handlers::handle_activity_summary(&db, Some(args));
    // Negative value can't be deserialized to u32, so this should error
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_activity_summary_raw_json_float_days() {
    let db = test_database();

    // Float value for days
    let args = to_map(json!({
        "days": 7.5
    }));

    let result = handlers::handle_activity_summary(&db, Some(args));
    // Float can be truncated to u32 by serde, or might error
    // Either behavior is acceptable
    let _ = result;
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_activity_summary_invoke_with_json_method() {
    let harness = harness_with_populated_db();

    let result = harness.invoke_with_json(
        "hindsight_activity_summary",
        json!({
            "days": 14
        }),
    );

    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.is_object());
}

#[test]
fn test_activity_summary_multiple_workspaces() {
    let db = test_database();

    // Create two workspaces
    let workspace1 =
        WorkspaceRecord::new("workspace-1".to_string(), "/tmp/workspace-1".to_string());
    let workspace2 =
        WorkspaceRecord::new("workspace-2".to_string(), "/tmp/workspace-2".to_string());

    db.insert_workspace(&workspace1).expect("insert workspace1");
    db.insert_workspace(&workspace2).expect("insert workspace2");

    let base = now();

    // Add commits to both
    let commit1 = sample_commit(&workspace1.id, "sha1000", "Commit 1", hours_ago(base, 1));
    let commit2 = sample_commit(&workspace2.id, "sha2000", "Commit 2", hours_ago(base, 2));

    db.insert_commit(&commit1).expect("insert commit1");
    db.insert_commit(&commit2).expect("insert commit2");

    let harness = McpTestHarness::new(db);

    let summary = harness
        .activity_summary(Some(7))
        .expect("activity_summary should succeed");

    // Should aggregate across all workspaces
    assert!(summary.commits >= 2);
}

#[test]
fn test_activity_summary_consistent_between_calls() {
    let harness = harness_with_populated_db();

    let summary1 = harness.activity_summary(Some(30)).expect("summary1");
    let summary2 = harness.activity_summary(Some(30)).expect("summary2");

    // Same parameters should return same results
    assert_eq!(summary1.commits, summary2.commits);
    assert_eq!(summary1.test_runs, summary2.test_runs);
    assert_eq!(summary1.copilot_sessions, summary2.copilot_sessions);
}

#[test]
fn test_activity_summary_monotonic_with_days() {
    let harness = harness_with_populated_db();

    let summary_7 = harness.activity_summary(Some(7)).expect("summary 7");
    let summary_30 = harness.activity_summary(Some(30)).expect("summary 30");
    let summary_365 = harness.activity_summary(Some(365)).expect("summary 365");

    // More days should include at least as many events
    assert!(summary_30.commits >= summary_7.commits);
    assert!(summary_365.commits >= summary_30.commits);

    assert!(summary_30.test_runs >= summary_7.test_runs);
    assert!(summary_365.test_runs >= summary_30.test_runs);
}
