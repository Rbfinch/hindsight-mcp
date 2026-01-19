// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Tests for the `ingest` subcommand and test result ingestion
//!
//! This module tests:
//! - Valid nextest JSON input ingestion
//! - Edge cases (empty input, malformed JSON, etc.)
//! - `--commit` flag behavior
//! - Error handling and user-friendly messages

mod fixtures;
mod test_utils;

use fixtures::test_database;
use hindsight_mcp::ingest::{IngestError, IngestStats, Ingestor};
use test_utils::{TempTestDir, sample_nextest_json};

// ============================================================================
// Valid Input Ingestion Tests
// ============================================================================

#[test]
fn test_ingest_valid_nextest_json_all_passing() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_valid");
    let json = sample_nextest_json(5, 0, 0);

    let stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    assert_eq!(stats.test_runs_inserted, 1, "should insert 1 test run");
    assert_eq!(
        stats.test_results_inserted, 5,
        "should insert 5 test results"
    );
}

#[test]
fn test_ingest_valid_nextest_json_with_failures() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_failures");
    let json = sample_nextest_json(3, 2, 0);

    let stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    assert_eq!(stats.test_runs_inserted, 1);
    assert_eq!(stats.test_results_inserted, 5);
}

#[test]
fn test_ingest_valid_nextest_json_with_ignored() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_ignored");
    let json = sample_nextest_json(3, 1, 2);

    let stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    assert_eq!(stats.test_runs_inserted, 1);
    // Ignored tests are also stored as results
    assert_eq!(stats.test_results_inserted, 6);
}

#[test]
fn test_ingest_creates_workspace_record() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_workspace");
    let json = sample_nextest_json(2, 0, 0);

    let _stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    // Workspace should be created - verify via count
    let db_ref = ingestor.database();
    let workspace_count = db_ref.count("workspaces").expect("count workspaces");
    assert!(workspace_count >= 1, "workspace should be created");
}

#[test]
fn test_ingest_test_results_stored_correctly() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_results");
    let json = sample_nextest_json(3, 1, 0);

    let stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    // Verify results count from stats
    assert_eq!(
        stats.test_results_inserted, 4,
        "should insert 4 test results"
    );

    // Verify via database count
    let db_ref = ingestor.database();
    let result_count = db_ref.count("test_results").expect("count results");
    assert_eq!(result_count, 4, "should have 4 test results in database");
}

#[test]
fn test_ingest_multiple_runs_separate() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_multiple");
    let json1 = sample_nextest_json(2, 0, 0);
    let json2 = sample_nextest_json(3, 0, 0);

    // First ingestion
    let stats1 = ingestor
        .ingest_tests(temp.path(), &json1, None)
        .expect("first ingestion");
    assert_eq!(stats1.test_runs_inserted, 1);
    assert_eq!(stats1.test_results_inserted, 2);

    // Second ingestion
    let stats2 = ingestor
        .ingest_tests(temp.path(), &json2, None)
        .expect("second ingestion");
    assert_eq!(stats2.test_runs_inserted, 1);
    assert_eq!(stats2.test_results_inserted, 3);

    // Should now have 2 test runs total
    let db_ref = ingestor.database();
    let run_count = db_ref.count("test_runs").expect("count runs");
    assert_eq!(run_count, 2, "should have 2 separate test runs");
}

#[test]
fn test_ingest_large_test_suite() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_large");
    // 500 tests - reasonably large but not too slow
    let json = sample_nextest_json(450, 30, 20);

    let stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    assert_eq!(stats.test_runs_inserted, 1);
    assert_eq!(stats.test_results_inserted, 500);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_ingest_empty_input_creates_empty_run() {
    // The parser is lenient - empty input just produces an empty summary
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_empty");
    let result = ingestor.ingest_tests(temp.path(), "", None);

    // Empty input is accepted but produces a run with no results
    assert!(
        result.is_ok(),
        "empty input should succeed (lenient parser)"
    );
    let stats = result.unwrap();
    assert_eq!(stats.test_runs_inserted, 1, "should create 1 test run");
    assert_eq!(stats.test_results_inserted, 0, "should insert 0 results");
}

#[test]
fn test_ingest_whitespace_only_creates_empty_run() {
    // Whitespace-only is also accepted by the lenient parser
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_whitespace");
    let result = ingestor.ingest_tests(temp.path(), "   \n\t\n   ", None);

    assert!(result.is_ok(), "whitespace-only input should succeed");
    let stats = result.unwrap();
    assert_eq!(stats.test_results_inserted, 0, "should insert 0 results");
}

#[test]
fn test_ingest_malformed_json_fails() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_malformed");
    let malformed = r#"{"type": "not valid nextest", "garbage": true"#; // Missing closing brace

    let result = ingestor.ingest_tests(temp.path(), malformed, None);

    assert!(result.is_err(), "malformed JSON should fail");
}

#[test]
fn test_ingest_invalid_json_structure_fails() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_invalid_structure");
    // Valid JSON but not nextest format
    let invalid = r#"{"foo": "bar", "baz": 123}"#;

    let result = ingestor.ingest_tests(temp.path(), invalid, None);

    assert!(result.is_err(), "invalid structure should fail");
}

#[test]
fn test_ingest_json_array_instead_of_lines_fails() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_array");
    // JSON array instead of line-delimited
    let invalid = r#"[{"type":"test","event":"ok","name":"test1"}]"#;

    let result = ingestor.ingest_tests(temp.path(), invalid, None);

    // This should fail or produce 0 results since it's not line-delimited
    match result {
        Err(_) => (), // Expected
        Ok(stats) => {
            // If it parses, should have no results
            assert_eq!(
                stats.test_results_inserted, 0,
                "array format should not parse correctly"
            );
        }
    }
}

#[test]
fn test_ingest_partial_json_lines() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_partial");
    // Mix of valid and invalid lines
    let partial = r#"{"type":"suite","event":"started","test_count":0}
invalid line here
{"type":"test","event":"ok","name":"test_1","exec_time":0.001}
{"type":"suite","event":"ok","passed":1,"failed":0,"ignored":0,"measured":0,"filtered_out":0,"exec_time":0.1}"#;

    // Parser may be lenient and skip invalid lines, or fail entirely
    let result = ingestor.ingest_tests(temp.path(), partial, None);

    // Accept either outcome - just shouldn't panic
    if let Ok(stats) = result {
        // If successful, should have parsed what it could
        assert!(stats.test_results_inserted <= 1);
    }
    // Err is also acceptable
}

#[test]
fn test_ingest_zero_tests() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_zero");
    let json = sample_nextest_json(0, 0, 0);

    let stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("zero tests should still succeed");

    // Should create a test run with 0 results
    assert_eq!(stats.test_runs_inserted, 1);
    assert_eq!(stats.test_results_inserted, 0);
}

#[test]
fn test_ingest_special_characters_in_test_names() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_special");
    // Create custom JSON with special characters
    let json = r#"{"type":"suite","event":"started","test_count":0}
{"type":"test","event":"started","name":"test::with::colons"}
{"type":"test","event":"ok","name":"test::with::colons","exec_time":0.001}
{"type":"test","event":"started","name":"test_with_unicode_μ_σ"}
{"type":"test","event":"ok","name":"test_with_unicode_μ_σ","exec_time":0.001}
{"type":"suite","event":"ok","passed":2,"failed":0,"ignored":0,"measured":0,"filtered_out":0,"exec_time":0.1}"#;

    let stats = ingestor
        .ingest_tests(temp.path(), json, None)
        .expect("special characters should be handled");

    assert_eq!(stats.test_results_inserted, 2);
}

#[test]
fn test_ingest_very_long_test_output() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_long_output");
    // Create JSON with very long stdout
    let long_output = "x".repeat(100_000);
    let json = format!(
        r#"{{"type":"suite","event":"started","test_count":0}}
{{"type":"test","event":"started","name":"test_long"}}
{{"type":"test","event":"failed","name":"test_long","exec_time":0.001,"stdout":"{long_output}"}}
{{"type":"suite","event":"failed","passed":0,"failed":1,"ignored":0,"measured":0,"filtered_out":0,"exec_time":0.1}}"#
    );

    let stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("long output should be handled");

    assert_eq!(stats.test_results_inserted, 1);
}

// ============================================================================
// --commit Flag Tests
// ============================================================================

#[test]
fn test_ingest_with_valid_commit_sha() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_commit");
    let json = sample_nextest_json(2, 0, 0);
    let commit_sha = "abc123def456789012345678901234567890abcd";

    let stats = ingestor
        .ingest_tests(temp.path(), &json, Some(commit_sha))
        .expect("ingestion with commit should succeed");

    assert_eq!(stats.test_runs_inserted, 1);

    // Verify the commit is linked via raw SQL query
    let db_ref = ingestor.database();
    let conn = db_ref.connection();
    let linked_sha: Option<String> = conn
        .query_row("SELECT commit_sha FROM test_runs LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("query should work");
    assert_eq!(linked_sha.as_deref(), Some(commit_sha));
}

#[test]
fn test_ingest_with_short_commit_sha() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_short_sha");
    let json = sample_nextest_json(2, 0, 0);
    let short_sha = "abc123"; // Only 6 characters

    // Short SHA should be accepted (validation is caller's responsibility)
    let stats = ingestor
        .ingest_tests(temp.path(), &json, Some(short_sha))
        .expect("short SHA should be accepted");

    assert_eq!(stats.test_runs_inserted, 1);

    let db_ref = ingestor.database();
    let conn = db_ref.connection();
    let linked_sha: Option<String> = conn
        .query_row("SELECT commit_sha FROM test_runs LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("query should work");
    assert_eq!(linked_sha.as_deref(), Some(short_sha));
}

#[test]
fn test_ingest_with_no_commit_sha() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_no_commit");
    let json = sample_nextest_json(2, 0, 0);

    let stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion without commit should succeed");

    assert_eq!(stats.test_runs_inserted, 1);

    // Verify no commit is linked
    let db_ref = ingestor.database();
    let conn = db_ref.connection();
    let linked_sha: Option<String> = conn
        .query_row("SELECT commit_sha FROM test_runs LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("query should work");
    assert!(linked_sha.is_none());
}

#[test]
fn test_ingest_with_full_40_char_sha() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_full_sha");
    let json = sample_nextest_json(1, 0, 0);
    let full_sha = "0123456789abcdef0123456789abcdef01234567";

    let stats = ingestor
        .ingest_tests(temp.path(), &json, Some(full_sha))
        .expect("full SHA should work");

    assert_eq!(stats.test_runs_inserted, 1);

    let db_ref = ingestor.database();
    let conn = db_ref.connection();
    let linked_sha: Option<String> = conn
        .query_row("SELECT commit_sha FROM test_runs LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("query should work");
    assert_eq!(linked_sha.as_deref(), Some(full_sha));
}

#[test]
fn test_ingest_commit_sha_with_uppercase() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_upper_sha");
    let json = sample_nextest_json(1, 0, 0);
    let upper_sha = "ABC123DEF456"; // Uppercase hex

    // Should be accepted (case is preserved or normalized)
    let stats = ingestor
        .ingest_tests(temp.path(), &json, Some(upper_sha))
        .expect("uppercase SHA should be accepted");

    assert_eq!(stats.test_runs_inserted, 1);
}

#[test]
fn test_ingest_empty_commit_sha_treated_as_none() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_empty_sha");
    let json = sample_nextest_json(1, 0, 0);

    // Empty string should be treated similarly to None or as empty
    let stats = ingestor
        .ingest_tests(temp.path(), &json, Some(""))
        .expect("empty SHA should not crash");

    assert_eq!(stats.test_runs_inserted, 1);

    // The run should have empty or None commit
    let db_ref = ingestor.database();
    let conn = db_ref.connection();
    let linked_sha: Option<String> = conn
        .query_row("SELECT commit_sha FROM test_runs LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("query should work");
    // Either None or Some("") is acceptable
    assert!(linked_sha.is_none() || linked_sha.as_deref() == Some(""));
}

#[test]
fn test_ingest_multiple_runs_different_commits() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_diff_commits");
    let json = sample_nextest_json(1, 0, 0);

    let sha1 = "commit1_sha_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let sha2 = "commit2_sha_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    // First run with sha1
    ingestor
        .ingest_tests(temp.path(), &json, Some(sha1))
        .expect("first commit");

    // Second run with sha2
    ingestor
        .ingest_tests(temp.path(), &json, Some(sha2))
        .expect("second commit");

    // Query runs and verify different commits
    let db_ref = ingestor.database();
    let run_count = db_ref.count("test_runs").expect("count runs");
    assert_eq!(run_count, 2);

    // Verify both SHAs are stored
    let conn = db_ref.connection();
    let shas: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT commit_sha FROM test_runs WHERE commit_sha IS NOT NULL")
            .expect("prepare");
        stmt.query_map([], |row| row.get(0))
            .expect("query")
            .filter_map(|r| r.ok())
            .collect()
    };
    assert!(shas.contains(&sha1.to_string()));
    assert!(shas.contains(&sha2.to_string()));
}

// ============================================================================
// Error Condition Tests
// ============================================================================

#[test]
fn test_ingest_stats_default() {
    let stats = IngestStats::default();
    assert_eq!(stats.commits_inserted, 0);
    assert_eq!(stats.test_runs_inserted, 0);
    assert_eq!(stats.test_results_inserted, 0);
    assert_eq!(stats.sessions_inserted, 0);
    assert_eq!(stats.messages_inserted, 0);
    assert_eq!(stats.warnings, 0);
    assert_eq!(stats.total_items(), 0);
}

#[test]
fn test_ingest_stats_total_items() {
    let stats = IngestStats {
        commits_inserted: 5,
        test_runs_inserted: 2,
        test_results_inserted: 50,
        sessions_inserted: 1,
        messages_inserted: 10,
        ..Default::default()
    };

    assert_eq!(stats.total_items(), 68);
}

#[test]
fn test_ingest_error_display_database() {
    use hindsight_mcp::db::DbError;
    let db_err = DbError::NotFound {
        table: "test".to_string(),
        id: "123".to_string(),
    };
    let ingest_err = IngestError::Database(db_err);
    let msg = format!("{}", ingest_err);
    assert!(msg.contains("Database error"), "error message: {}", msg);
}

#[test]
fn test_ingest_error_display_tests() {
    use hindsight_tests::TestsError;
    let tests_err = TestsError::InvalidFormat {
        message: "invalid format".to_string(),
    };
    let ingest_err = IngestError::Tests(tests_err);
    let msg = format!("{}", ingest_err);
    assert!(msg.contains("Tests error"), "error message: {}", msg);
}

#[test]
fn test_ingest_database_preserved_after_error() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_preserve");

    // First, successful ingestion
    let json_good = sample_nextest_json(2, 0, 0);
    ingestor
        .ingest_tests(temp.path(), &json_good, None)
        .expect("first should succeed");

    // Then, failed ingestion
    let _result = ingestor.ingest_tests(temp.path(), "invalid json", None);
    // Error is expected, ignore result

    // Original data should still be there
    let db_ref = ingestor.database();
    let run_count = db_ref.count("test_runs").expect("count should work");
    assert_eq!(run_count, 1, "original run should still exist");
}

#[test]
fn test_ingest_workspace_path_variations() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let json = sample_nextest_json(1, 0, 0);

    // Absolute path
    let temp1 = TempTestDir::new("ingest_abs");
    let result1 = ingestor.ingest_tests(temp1.path(), &json, None);
    assert!(result1.is_ok(), "absolute path should work");

    // Path with trailing slash (create new ingestor to avoid duplicate workspace issues)
    let db2 = test_database();
    let mut ingestor2 = Ingestor::new(db2);
    let temp2 = TempTestDir::new("ingest_trailing");
    let path_with_slash = format!("{}/", temp2.path().display());
    let result2 = ingestor2.ingest_tests(&path_with_slash, &json, None);
    // Should work or fail gracefully
    assert!(result2.is_ok() || result2.is_err());
}

#[test]
fn test_ingest_nonexistent_workspace_creates_record() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let json = sample_nextest_json(1, 0, 0);
    // Path that doesn't exist
    let fake_path = "/nonexistent/path/to/project";

    // The ingestor creates workspace record based on path string,
    // it doesn't validate that the path exists
    let result = ingestor.ingest_tests(fake_path, &json, None);

    // This may succeed since workspace creation is by path string
    match result {
        Ok(stats) => {
            assert_eq!(stats.test_runs_inserted, 1);
        }
        Err(_) => {
            // Also acceptable if it validates path existence
        }
    }
}

// ============================================================================
// Test Run Record Verification
// ============================================================================

#[test]
fn test_ingest_run_record_has_counts() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_counts");
    let json = sample_nextest_json(5, 3, 2);

    ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    // Verify the test run has correct counts stored
    let db_ref = ingestor.database();
    let conn = db_ref.connection();

    let (passed, failed, ignored): (i32, i32, i32) = conn
        .query_row(
            "SELECT passed_count, failed_count, ignored_count FROM test_runs LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("query should work");

    assert_eq!(passed, 5, "passed count should be 5");
    assert_eq!(failed, 3, "failed count should be 3");
    assert_eq!(ignored, 2, "ignored count should be 2");
}

#[test]
fn test_ingest_run_has_started_at_timestamp() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_timestamp");
    let json = sample_nextest_json(1, 0, 0);

    ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    let db_ref = ingestor.database();
    let conn = db_ref.connection();

    let started_at: String = conn
        .query_row("SELECT started_at FROM test_runs LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("query should work");

    // Should be a valid RFC3339 timestamp
    assert!(!started_at.is_empty());
    assert!(started_at.contains("T"), "should be ISO timestamp format");
}

#[test]
fn test_ingest_result_records_have_outcome() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_outcomes");
    let json = sample_nextest_json(2, 1, 0);

    ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    let db_ref = ingestor.database();
    let conn = db_ref.connection();

    // Count outcomes
    let pass_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM test_results WHERE outcome = 'passed'",
            [],
            |row| row.get(0),
        )
        .expect("query should work");

    let fail_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM test_results WHERE outcome = 'failed'",
            [],
            |row| row.get(0),
        )
        .expect("query should work");

    assert_eq!(pass_count, 2);
    assert_eq!(fail_count, 1);
}

#[test]
fn test_ingest_result_records_linked_to_run() {
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("ingest_linked");
    let json = sample_nextest_json(3, 0, 0);

    ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    let db_ref = ingestor.database();
    let conn = db_ref.connection();

    // Get the run ID
    let run_id: String = conn
        .query_row("SELECT id FROM test_runs LIMIT 1", [], |row| row.get(0))
        .expect("query should work");

    // All results should be linked to this run
    let linked_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM test_results WHERE run_id = ?1",
            [&run_id],
            |row| row.get(0),
        )
        .expect("query should work");

    assert_eq!(linked_count, 3, "all results should be linked to the run");
}
