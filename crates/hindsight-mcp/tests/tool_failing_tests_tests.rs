// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Tests for the hindsight_failing_tests MCP tool handler
//!
//! This module tests the failing_tests tool which returns currently
//! failing tests from the most recent test runs.

mod fixtures;
mod mcp_harness;

use fixtures::{populated_database, sample_test_result, sample_test_run, test_database};
use mcp_harness::McpTestHarness;
use serde_json::{Map, Value, json};

use hindsight_mcp::db::{Database, TestRunRecord, WorkspaceRecord};
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

/// Create a database with specific failing tests for targeted testing
fn database_with_failing_tests() -> (Database, String, String) {
    let mut db = test_database();

    let workspace = WorkspaceRecord::new(
        "fail-test-project".to_string(),
        "/tmp/fail-test-project".to_string(),
    );
    db.insert_workspace(&workspace).expect("insert workspace");

    // Create a commit
    let commit_sha = "abc123def456789012345678901234567890abcd".to_string();

    // Create a test run with failures
    let mut run = TestRunRecord::new(workspace.id.clone());
    run = run.with_commit(&commit_sha);
    run = run.finished(5, 3, 0); // 5 passed, 3 failed
    db.insert_test_run(&run).expect("insert run");

    // Create failing test results
    let results = vec![
        sample_test_result(
            &run.id,
            "test_authentication_fails",
            "failed",
            Some("Expected user to be authenticated"),
        ),
        sample_test_result(
            &run.id,
            "test_database_connection",
            "failed",
            Some("Connection timeout after 30s"),
        ),
        sample_test_result(
            &run.id,
            "test_api_response",
            "failed",
            Some("Status code 500"),
        ),
        sample_test_result(&run.id, "test_passes_1", "passed", None),
        sample_test_result(&run.id, "test_passes_2", "passed", None),
    ];

    db.insert_test_results_batch(&results)
        .expect("insert results");

    (db, workspace.id, commit_sha)
}

// ============================================================================
// Basic Failing Tests Queries
// ============================================================================

#[test]
fn test_failing_tests_default_limit() {
    let harness = harness_with_populated_db();

    // Default limit should be 50
    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    assert!(tests.len() <= 50);
}

#[test]
fn test_failing_tests_returns_failures() {
    let harness = harness_with_populated_db();

    // The populated database has 2 failing tests
    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    assert!(!tests.is_empty(), "Should return failing tests");
}

#[test]
fn test_failing_tests_empty_database() {
    let db = test_database();
    let harness = McpTestHarness::new(db);

    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    assert!(
        tests.is_empty(),
        "Empty database should return no failing tests"
    );
}

#[test]
fn test_failing_tests_no_failures_in_runs() {
    let db = test_database();

    // Create workspace and a passing test run
    let workspace = WorkspaceRecord::new(
        "passing-project".to_string(),
        "/tmp/passing-project".to_string(),
    );
    db.insert_workspace(&workspace).expect("insert workspace");

    let run = sample_test_run(&workspace.id, None, 10, 0, 0); // All passed
    db.insert_test_run(&run).expect("insert run");

    let harness = McpTestHarness::new(db);
    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    assert!(tests.is_empty(), "No failures should return empty");
}

// ============================================================================
// Limit Parameter Tests
// ============================================================================

#[test]
fn test_failing_tests_custom_limit() {
    let (db, _, _) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    let tests = harness
        .failing_tests(Some(2), None, None)
        .expect("failing_tests should succeed");

    assert!(tests.len() <= 2, "Should respect limit of 2");
}

#[test]
fn test_failing_tests_limit_zero() {
    let harness = harness_with_populated_db();

    let tests = harness
        .failing_tests(Some(0), None, None)
        .expect("failing_tests should succeed");

    assert!(tests.is_empty(), "Limit 0 should return no results");
}

#[test]
fn test_failing_tests_very_large_limit() {
    let harness = harness_with_populated_db();

    let tests = harness
        .failing_tests(Some(10000), None, None)
        .expect("failing_tests should succeed");

    // Should return results without crashing
    let _ = tests;
}

// ============================================================================
// Workspace Filter Tests
// ============================================================================

#[test]
fn test_failing_tests_workspace_filter_valid() {
    let harness = harness_with_populated_db();

    // Filter by the test workspace path
    let tests = harness
        .failing_tests(None, Some("/tmp/test-project"), None)
        .expect("failing_tests should succeed");

    // Should have failing tests from this workspace
    assert!(!tests.is_empty());
}

#[test]
fn test_failing_tests_workspace_filter_nonexistent() {
    let harness = harness_with_populated_db();

    let tests = harness
        .failing_tests(None, Some("/nonexistent/workspace"), None)
        .expect("failing_tests should succeed");

    // When workspace filter doesn't match any known workspace, the filter is ignored
    // and all failing tests are returned (graceful degradation)
    // This is the actual behavior - the query falls back to no workspace filter
    let _ = tests; // Just verify it doesn't error
}

#[test]
fn test_failing_tests_workspace_filter_empty_string() {
    let harness = harness_with_populated_db();

    let tests = harness
        .failing_tests(None, Some(""), None)
        .expect("failing_tests should succeed");

    // Empty string workspace filter doesn't match any workspace,
    // so the filter is ignored and all failing tests are returned
    let _ = tests; // Just verify it doesn't error
}

#[test]
fn test_failing_tests_with_default_workspace_set() {
    let (db, _, _) = database_with_failing_tests();
    let harness =
        McpTestHarness::new(db).with_workspace(std::path::PathBuf::from("/tmp/fail-test-project"));

    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    // Should use default workspace and find failures
    assert!(!tests.is_empty());
}

// ============================================================================
// Commit Filter Tests
// ============================================================================

#[test]
fn test_failing_tests_commit_filter_full_sha() {
    let (db, _, commit_sha) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    let tests = harness
        .failing_tests(None, None, Some(&commit_sha))
        .expect("failing_tests should succeed");

    // Should find failing tests for this commit
    assert!(!tests.is_empty());

    // All results should be linked to this commit
    for test in &tests {
        if let Some(sha) = &test.commit_sha {
            assert!(sha.starts_with("abc123"));
        }
    }
}

#[test]
fn test_failing_tests_commit_filter_partial_sha() {
    let (db, _, _) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    // Use partial SHA
    let tests = harness
        .failing_tests(None, None, Some("abc123"))
        .expect("failing_tests should succeed");

    // Should find failing tests with partial SHA match
    assert!(!tests.is_empty());
}

#[test]
fn test_failing_tests_commit_filter_short_sha() {
    let (db, _, _) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    // Very short SHA prefix
    let tests = harness
        .failing_tests(None, None, Some("abc"))
        .expect("failing_tests should succeed");

    // Should still find matches
    assert!(!tests.is_empty());
}

#[test]
fn test_failing_tests_commit_filter_nonexistent() {
    let harness = harness_with_populated_db();

    let tests = harness
        .failing_tests(None, None, Some("0000000000000000000000000000000000000000"))
        .expect("failing_tests should succeed");

    assert!(
        tests.is_empty(),
        "Nonexistent commit should return no results"
    );
}

#[test]
fn test_failing_tests_commit_filter_empty_string() {
    let harness = harness_with_populated_db();

    let tests = harness
        .failing_tests(None, None, Some(""))
        .expect("failing_tests should succeed");

    // Empty string commit filter - behavior depends on implementation
    let _ = tests;
}

// ============================================================================
// Combined Filters Tests
// ============================================================================

#[test]
fn test_failing_tests_workspace_and_commit_filter() {
    let (db, _, commit_sha) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    let tests = harness
        .failing_tests(None, Some("/tmp/fail-test-project"), Some(&commit_sha))
        .expect("failing_tests should succeed");

    assert!(!tests.is_empty());
}

#[test]
fn test_failing_tests_all_filters() {
    let (db, _, commit_sha) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    let tests = harness
        .failing_tests(Some(1), Some("/tmp/fail-test-project"), Some(&commit_sha))
        .expect("failing_tests should succeed");

    assert!(tests.len() <= 1, "Should respect limit");
}

// ============================================================================
// Result Structure Tests
// ============================================================================

#[test]
fn test_failing_tests_results_have_required_fields() {
    let (db, _, _) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    for test in &tests {
        assert!(!test.test_name.is_empty(), "test_name should not be empty");
        assert!(
            !test.suite_name.is_empty(),
            "suite_name should not be empty"
        );
        assert!(!test.full_name.is_empty(), "full_name should not be empty");
        assert!(!test.run_id.is_empty(), "run_id should not be empty");
        assert!(
            !test.started_at.is_empty(),
            "started_at should not be empty"
        );
    }
}

#[test]
fn test_failing_tests_include_output() {
    let (db, _, _) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    // At least some failing tests should have output
    let has_output = tests.iter().any(|t| t.output_json.is_some());
    assert!(has_output, "Some failing tests should have output");
}

#[test]
fn test_failing_tests_include_duration() {
    let (db, _, _) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    // Tests should have duration
    for test in &tests {
        if let Some(duration) = test.duration_ms {
            assert!(duration >= 0, "Duration should be non-negative");
        }
    }
}

#[test]
fn test_failing_tests_includes_commit_sha() {
    let (db, _, _) = database_with_failing_tests();
    let harness = McpTestHarness::new(db);

    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    // Tests should be linked to commits
    let has_commit = tests.iter().any(|t| t.commit_sha.is_some());
    assert!(has_commit, "Some failing tests should have commit_sha");
}

// ============================================================================
// Raw JSON Input Tests
// ============================================================================

#[test]
fn test_failing_tests_raw_json_valid() {
    let db = test_database();

    let args = to_map(json!({
        "limit": 25,
        "workspace": "/some/path",
        "commit": "abc123"
    }));

    let result = handlers::handle_failing_tests(&db, Some(args), None);
    assert!(result.is_ok());
}

#[test]
fn test_failing_tests_raw_json_empty_args() {
    let db = test_database();

    let args = to_map(json!({}));
    let result = handlers::handle_failing_tests(&db, Some(args), None);
    assert!(result.is_ok());
}

#[test]
fn test_failing_tests_raw_json_none_args() {
    let db = test_database();

    let result = handlers::handle_failing_tests(&db, None, None);
    assert!(result.is_ok());
}

#[test]
fn test_failing_tests_raw_json_extra_fields_ignored() {
    let db = test_database();

    let args = to_map(json!({
        "limit": 10,
        "unknown_field": "some value"
    }));

    let result = handlers::handle_failing_tests(&db, Some(args), None);
    assert!(result.is_ok(), "Extra fields should be ignored");
}

#[test]
fn test_failing_tests_raw_json_wrong_type_for_limit() {
    let db = test_database();

    let args = to_map(json!({
        "limit": "not a number"
    }));

    let result = handlers::handle_failing_tests(&db, Some(args), None);
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_failing_tests_test_name_with_special_chars() {
    let mut db = test_database();

    let workspace = WorkspaceRecord::new(
        "special-chars".to_string(),
        "/tmp/special-chars".to_string(),
    );
    db.insert_workspace(&workspace).expect("insert workspace");

    let run = sample_test_run(&workspace.id, None, 0, 1, 0);
    db.insert_test_run(&run).expect("insert run");

    // Test with special characters in name
    let result = sample_test_result(&run.id, "test::module::submodule<Type>", "failed", None);
    db.insert_test_results_batch(&[result])
        .expect("insert result");

    let harness = McpTestHarness::new(db);
    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    assert!(!tests.is_empty());
}

#[test]
fn test_failing_tests_invoke_with_json_method() {
    let harness = harness_with_populated_db();

    let result = harness.invoke_with_json(
        "hindsight_failing_tests",
        json!({
            "limit": 5
        }),
    );

    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.is_array());
}

#[test]
fn test_failing_tests_multiple_runs_same_workspace() {
    let mut db = test_database();

    let workspace = WorkspaceRecord::new("multi-run".to_string(), "/tmp/multi-run".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    // Create multiple runs
    let run1 = sample_test_run(&workspace.id, Some("abc111"), 5, 2, 0);
    let run2 = sample_test_run(&workspace.id, Some("abc222"), 5, 1, 0);
    db.insert_test_run(&run1).expect("insert run1");
    db.insert_test_run(&run2).expect("insert run2");

    let results = vec![
        sample_test_result(&run1.id, "test_fail_1", "failed", Some("Error 1")),
        sample_test_result(&run1.id, "test_fail_2", "failed", Some("Error 2")),
        sample_test_result(&run2.id, "test_fail_3", "failed", Some("Error 3")),
    ];
    db.insert_test_results_batch(&results)
        .expect("insert results");

    let harness = McpTestHarness::new(db);
    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    // Should find failures from multiple runs
    assert!(tests.len() >= 2);
}

#[test]
fn test_failing_tests_long_error_output() {
    let mut db = test_database();

    let workspace = WorkspaceRecord::new("long-output".to_string(), "/tmp/long-output".to_string());
    db.insert_workspace(&workspace).expect("insert workspace");

    let run = sample_test_run(&workspace.id, None, 0, 1, 0);
    db.insert_test_run(&run).expect("insert run");

    // Very long error output
    let long_output = "Error: ".to_string() + &"x".repeat(10000);
    let result = sample_test_result(&run.id, "test_long_output", "failed", Some(&long_output));
    db.insert_test_results_batch(&[result])
        .expect("insert result");

    let harness = McpTestHarness::new(db);
    let tests = harness
        .failing_tests(None, None, None)
        .expect("failing_tests should succeed");

    assert!(!tests.is_empty());
}
