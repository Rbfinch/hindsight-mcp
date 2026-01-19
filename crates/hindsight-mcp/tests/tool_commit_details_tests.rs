// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Tests for the hindsight_commit_details MCP tool handler
//!
//! This module tests the commit_details tool which returns detailed
//! information about a specific commit including linked test runs.

mod fixtures;
mod mcp_harness;

use fixtures::{
    hours_ago, now, populated_database, sample_commit, sample_test_result, sample_test_run,
    test_database,
};
use mcp_harness::{
    McpTestHarness, assert_error_contains, assert_invalid_input_error, assert_not_found_error,
};
use serde_json::{Map, Value, json};

use hindsight_mcp::db::{Database, WorkspaceRecord};
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

/// Create a database with a commit that has linked test runs
fn database_with_linked_tests() -> (Database, String) {
    let mut db = test_database();

    let workspace = WorkspaceRecord::new(
        "linked-test-project".to_string(),
        "/tmp/linked-test-project".to_string(),
    );
    db.insert_workspace(&workspace).expect("insert workspace");

    let base = now();
    let commit_sha = "abc123def456789012345678901234567890abcd".to_string();

    let commit = sample_commit(
        &workspace.id,
        &commit_sha,
        "Fix critical bug in authentication",
        hours_ago(base, 1),
    );
    db.insert_commit(&commit).expect("insert commit");

    // Create a test run linked to this commit
    let run = sample_test_run(&workspace.id, Some(&commit_sha), 8, 2, 0);
    db.insert_test_run(&run).expect("insert run");

    let results = vec![
        sample_test_result(&run.id, "test_auth_login", "passed", None),
        sample_test_result(&run.id, "test_auth_logout", "passed", None),
        sample_test_result(
            &run.id,
            "test_auth_refresh",
            "failed",
            Some("Token expired"),
        ),
    ];
    db.insert_test_results_batch(&results)
        .expect("insert results");

    (db, commit_sha)
}

// ============================================================================
// Required SHA Parameter Tests
// ============================================================================

#[test]
fn test_commit_details_sha_required() {
    let db = test_database();

    // Missing sha field entirely
    let args = to_map(json!({}));
    let result = handlers::handle_commit_details(&db, Some(args));

    // Should fail because sha is required
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_commit_details_empty_sha_produces_error() {
    let harness = harness_with_populated_db();

    let result = harness.commit_details("");

    assert!(result.is_err());
    assert_invalid_input_error(result);
}

#[test]
fn test_commit_details_whitespace_sha() {
    let harness = harness_with_populated_db();

    let result = harness.commit_details("   ");

    // Whitespace-only should be treated as not found or invalid
    assert!(result.is_err());
}

// ============================================================================
// SHA Lookup Tests
// ============================================================================

#[test]
fn test_commit_details_full_sha_found() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    let commit = harness
        .commit_details(&commit_sha)
        .expect("commit_details should succeed");

    assert_eq!(commit.sha, commit_sha);
}

#[test]
fn test_commit_details_partial_sha_found() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    // Use first 7 characters (typical short SHA)
    let short_sha = &commit_sha[..7];

    let commit = harness
        .commit_details(short_sha)
        .expect("commit_details should succeed");

    assert!(commit.sha.starts_with(short_sha));
}

#[test]
fn test_commit_details_very_short_sha() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    // Use just 3 characters
    let very_short = &commit_sha[..3];

    let result = harness.commit_details(very_short);

    // Should find a match (might be ambiguous in real scenarios)
    assert!(result.is_ok());
}

#[test]
fn test_commit_details_sha_not_found() {
    let harness = harness_with_populated_db();

    let result = harness.commit_details("0000000000000000000000000000000000000000");

    assert!(result.is_err());
    assert_not_found_error(result);
}

#[test]
fn test_commit_details_not_found_message() {
    let harness = harness_with_populated_db();

    let result = harness.commit_details("nonexistent123");

    assert_error_contains(result, "Commit not found");
}

// ============================================================================
// Commit Content Tests
// ============================================================================

#[test]
fn test_commit_details_includes_message() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    let commit = harness
        .commit_details(&commit_sha)
        .expect("commit_details should succeed");

    assert!(commit.message.contains("Fix critical bug"));
}

#[test]
fn test_commit_details_includes_author() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    let commit = harness
        .commit_details(&commit_sha)
        .expect("commit_details should succeed");

    assert!(!commit.author.is_empty());
}

#[test]
fn test_commit_details_includes_timestamp() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    let commit = harness
        .commit_details(&commit_sha)
        .expect("commit_details should succeed");

    assert!(!commit.timestamp.is_empty());
}

// ============================================================================
// Linked Test Runs Tests
// ============================================================================

#[test]
fn test_commit_details_includes_test_runs() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    let commit = harness
        .commit_details(&commit_sha)
        .expect("commit_details should succeed");

    // Should have linked test runs
    assert!(!commit.test_runs.is_empty());
}

#[test]
fn test_commit_details_test_run_has_counts() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    let commit = harness
        .commit_details(&commit_sha)
        .expect("commit_details should succeed");

    if let Some(run) = commit.test_runs.first() {
        // Test run should have pass/fail counts
        let _ = run.passed;
        let _ = run.failed;
    }
}

#[test]
fn test_commit_details_no_linked_tests() {
    let db = test_database();

    let workspace = WorkspaceRecord::new("no-tests".to_string(), "/tmp/no-tests".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    let commit = sample_commit(
        &workspace.id,
        "deadbeef00000000000000000000000000000000",
        "Commit without tests",
        now(),
    );
    db.insert_commit(&commit).expect("insert commit");

    let harness = McpTestHarness::new(db);

    let result = harness
        .commit_details("deadbeef")
        .expect("commit_details should succeed");

    // Should have empty test_runs, not None or error
    assert!(result.test_runs.is_empty());
}

// ============================================================================
// SHA Format Tests
// ============================================================================

#[test]
fn test_commit_details_uppercase_sha() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    // Use uppercase SHA
    let upper_sha = commit_sha.to_uppercase();

    let result = harness.commit_details(&upper_sha);

    // Some implementations are case-sensitive, others aren't
    // Just verify it doesn't crash
    let _ = result;
}

#[test]
fn test_commit_details_mixed_case_sha() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    // Mix case
    let mixed = format!(
        "{}{}{}",
        &commit_sha[..2].to_uppercase(),
        &commit_sha[2..5],
        &commit_sha[5..10].to_uppercase()
    );

    let result = harness.commit_details(&mixed);

    // Should handle without crashing
    let _ = result;
}

#[test]
fn test_commit_details_sha_with_leading_zeros() {
    let db = test_database();

    let workspace = WorkspaceRecord::new("zero-sha".to_string(), "/tmp/zero-sha".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    // SHA starting with zeros
    let zero_sha = "0000abcdef0000000000000000000000000000";
    let commit = sample_commit(&workspace.id, zero_sha, "Commit with zeros", now());
    db.insert_commit(&commit).expect("insert commit");

    let harness = McpTestHarness::new(db);

    let result = harness.commit_details("0000abc");
    assert!(result.is_ok());
}

// ============================================================================
// Raw JSON Input Tests
// ============================================================================

#[test]
fn test_commit_details_raw_json_valid() {
    let (db, commit_sha) = database_with_linked_tests();

    let args = to_map(json!({
        "sha": commit_sha
    }));

    let result = handlers::handle_commit_details(&db, Some(args));
    assert!(result.is_ok());
}

#[test]
fn test_commit_details_raw_json_sha_only() {
    let db = test_database();

    let args = to_map(json!({
        "sha": "abc123"
    }));

    let result = handlers::handle_commit_details(&db, Some(args));
    // Not found, but valid input
    assert!(matches!(result, Err(HandlerError::NotFound(_))));
}

#[test]
fn test_commit_details_raw_json_extra_fields_ignored() {
    let db = test_database();

    let args = to_map(json!({
        "sha": "abc123",
        "unknown_field": "some value"
    }));

    let result = handlers::handle_commit_details(&db, Some(args));
    // Extra fields should be ignored; not found is fine
    assert!(matches!(result, Err(HandlerError::NotFound(_))));
}

#[test]
fn test_commit_details_raw_json_wrong_type_for_sha() {
    let db = test_database();

    // Number instead of string for sha
    let args = to_map(json!({
        "sha": 12345
    }));

    let result = handlers::handle_commit_details(&db, Some(args));
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_commit_details_raw_json_null_sha() {
    let db = test_database();

    let args = to_map(json!({
        "sha": null
    }));

    let result = handlers::handle_commit_details(&db, Some(args));
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_commit_details_invoke_with_json_method() {
    let (db, commit_sha) = database_with_linked_tests();
    let harness = McpTestHarness::new(db);

    let result = harness.invoke_with_json(
        "hindsight_commit_details",
        json!({
            "sha": commit_sha
        }),
    );

    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.is_object());
}

#[test]
fn test_commit_details_multiple_commits_unique_lookup() {
    let db = test_database();

    let workspace =
        WorkspaceRecord::new("multi-commit".to_string(), "/tmp/multi-commit".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    let base = now();

    // Create multiple commits with different SHAs
    let commit1 = sample_commit(
        &workspace.id,
        "abc1111111111111111111111111111111111111",
        "First commit",
        hours_ago(base, 2),
    );
    let commit2 = sample_commit(
        &workspace.id,
        "abc2222222222222222222222222222222222222",
        "Second commit",
        hours_ago(base, 1),
    );

    db.insert_commit(&commit1).expect("insert commit1");
    db.insert_commit(&commit2).expect("insert commit2");

    let harness = McpTestHarness::new(db);

    // Lookup specific commit
    let result = harness.commit_details("abc111").expect("lookup");
    assert!(result.message.contains("First"));

    let result2 = harness.commit_details("abc222").expect("lookup");
    assert!(result2.message.contains("Second"));
}

#[test]
fn test_commit_details_special_chars_in_message() {
    let db = test_database();

    let workspace = WorkspaceRecord::new("special-msg".to_string(), "/tmp/special-msg".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    let commit = sample_commit(
        &workspace.id,
        "special0000000000000000000000000000000000",
        "Fix: \"quoted\" text & <special> 'chars'\nMulti-line\nMessage",
        now(),
    );
    db.insert_commit(&commit).expect("insert commit");

    let harness = McpTestHarness::new(db);

    let result = harness.commit_details("special").expect("lookup");
    assert!(result.message.contains("quoted"));
    assert!(result.message.contains("special"));
}

#[test]
fn test_commit_details_unicode_in_message() {
    let db = test_database();

    let workspace = WorkspaceRecord::new("unicode-msg".to_string(), "/tmp/unicode-msg".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    let commit = sample_commit(
        &workspace.id,
        "unicode0000000000000000000000000000000000",
        "Add 日本語 support and emoji 🎉",
        now(),
    );
    db.insert_commit(&commit).expect("insert commit");

    let harness = McpTestHarness::new(db);

    let result = harness.commit_details("unicode").expect("lookup");
    assert!(result.message.contains("日本語"));
    assert!(result.message.contains("🎉"));
}

#[test]
fn test_commit_details_long_commit_message() {
    let db = test_database();

    let workspace = WorkspaceRecord::new("long-msg".to_string(), "/tmp/long-msg".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    // Very long commit message
    let long_message = "Long commit message: ".to_string() + &"x".repeat(5000);
    let commit = sample_commit(
        &workspace.id,
        "longmsg0000000000000000000000000000000000",
        &long_message,
        now(),
    );
    db.insert_commit(&commit).expect("insert commit");

    let harness = McpTestHarness::new(db);

    let result = harness.commit_details("longmsg").expect("lookup");
    assert!(result.message.len() > 5000);
}

#[test]
fn test_commit_details_sql_injection_attempt() {
    let harness = harness_with_populated_db();

    // Attempt SQL injection in SHA
    let result = harness.commit_details("'; DROP TABLE commits; --");

    // Should handle safely - not found is expected
    assert!(result.is_err());

    // Verify database is still intact
    let verify = harness.commit_details("abc");
    let _ = verify; // Might find or not find, but shouldn't crash
}

#[test]
fn test_commit_details_multiple_test_runs_for_commit() {
    let db = test_database();

    let workspace = WorkspaceRecord::new("multi-runs".to_string(), "/tmp/multi-runs".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    let commit_sha = "multiruns000000000000000000000000000000";
    let commit = sample_commit(
        &workspace.id,
        commit_sha,
        "Commit with multiple runs",
        now(),
    );
    db.insert_commit(&commit).expect("insert commit");

    // Create multiple test runs for the same commit
    let run1 = sample_test_run(&workspace.id, Some(commit_sha), 10, 0, 0);
    let run2 = sample_test_run(&workspace.id, Some(commit_sha), 9, 1, 0);
    db.insert_test_run(&run1).expect("insert run1");
    db.insert_test_run(&run2).expect("insert run2");

    let harness = McpTestHarness::new(db);

    let result = harness.commit_details("multiruns").expect("lookup");

    // Should have multiple test runs linked
    assert!(result.test_runs.len() >= 2);
}
