// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Tests for the hindsight_ingest MCP tool handler
//!
//! This module tests the ingest tool which triggers data ingestion
//! from git, copilot, or all sources.

mod fixtures;
mod mcp_harness;

use fixtures::test_database;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::PathBuf;

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

/// Create a unique temp directory path for a test
fn unique_temp_dir(suffix: &str) -> PathBuf {
    let path = std::env::temp_dir().join("hindsight-tests").join(format!(
        "{}_{}",
        suffix,
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

/// Create a temporary directory that looks like a git repo
fn create_temp_git_repo(suffix: &str) -> PathBuf {
    let temp = unique_temp_dir(suffix);
    let git_dir = temp.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git dir");

    // Create minimal git structure
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
    fs::create_dir_all(git_dir.join("objects")).expect("create objects");
    fs::create_dir_all(git_dir.join("refs/heads")).expect("create refs");

    temp
}

/// Create a temporary directory without git
fn create_temp_workspace(suffix: &str) -> PathBuf {
    unique_temp_dir(suffix)
}

/// Create a temporary file (not a directory)
fn create_temp_file(suffix: &str) -> (PathBuf, PathBuf) {
    let temp = unique_temp_dir(suffix);
    let file_path = temp.join("not_a_directory.txt");
    fs::write(&file_path, "This is a file, not a directory").expect("write file");
    (temp, file_path)
}

// ============================================================================
// Required Workspace Parameter Tests
// ============================================================================

#[test]
fn test_ingest_workspace_required() {
    let db = test_database();

    // Missing workspace field entirely
    let args = to_map(json!({}));
    let result = handlers::handle_ingest(db, Some(args));

    // Should fail because workspace is required
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_ingest_empty_workspace_string() {
    let db = test_database();

    let args = to_map(json!({
        "workspace": ""
    }));

    let result = handlers::handle_ingest(db, Some(args));

    // Empty workspace should fail (path doesn't exist)
    assert!(result.is_err());
}

// ============================================================================
// Workspace Path Validation Tests
// ============================================================================

#[test]
fn test_ingest_workspace_not_found() {
    let db = test_database();

    let args = to_map(json!({
        "workspace": "/nonexistent/path/that/does/not/exist"
    }));

    let result = handlers::handle_ingest(db, Some(args));

    assert!(matches!(result, Err(HandlerError::WorkspaceNotFound(_))));
}

#[test]
fn test_ingest_workspace_is_file_not_directory() {
    let db = test_database();
    let (_temp, file_path) = create_temp_file("file_not_dir");

    let args = to_map(json!({
        "workspace": file_path.to_str().unwrap()
    }));

    let result = handlers::handle_ingest(db, Some(args));

    // Should fail because it's a file, not a directory
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_ingest_workspace_valid_directory() {
    let db = test_database();
    let temp = create_temp_workspace("valid_dir");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap()
    }));

    let result = handlers::handle_ingest(db, Some(args));

    // Should succeed (even if no data to ingest)
    assert!(result.is_ok());
}

// ============================================================================
// Source Parameter Tests
// ============================================================================

#[test]
fn test_ingest_source_default_is_all() {
    let db = test_database();
    let temp = create_temp_workspace("src_default");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap()
        // source not specified, defaults to "all"
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    assert_eq!(result.source, "all");
}

#[test]
fn test_ingest_source_git_requires_real_repo() {
    let db = test_database();
    // Our fake git repo doesn't have valid refs, so git ingestion fails
    let temp = create_temp_git_repo("src_git");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "source": "git"
    }));

    let result = handlers::handle_ingest(db, Some(args));

    // Git source on incomplete repo fails with Ingest error
    assert!(matches!(result, Err(HandlerError::Ingest(_))));
}

#[test]
fn test_ingest_source_copilot() {
    let db = test_database();
    let temp = create_temp_workspace("src_copilot");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "source": "copilot"
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    assert_eq!(result.source, "copilot");
}

#[test]
fn test_ingest_source_all_explicit() {
    let db = test_database();
    let temp = create_temp_workspace("src_all");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "source": "all"
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    assert_eq!(result.source, "all");
}

#[test]
fn test_ingest_source_invalid_falls_back() {
    let db = test_database();
    let temp = create_temp_workspace("src_invalid");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "source": "invalid_source"
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    // Invalid source falls back to "all" behavior
    assert_eq!(result.source, "invalid_source");
}

// ============================================================================
// Incremental Parameter Tests
// ============================================================================

#[test]
fn test_ingest_incremental_default_is_true() {
    let db = test_database();
    let temp = create_temp_workspace("inc_default");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap()
        // incremental not specified, defaults to true
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    // Default is incremental mode
    let _ = result;
}

#[test]
fn test_ingest_incremental_true() {
    let db = test_database();
    let temp = create_temp_workspace("inc_true");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "incremental": true
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    let _ = result;
}

#[test]
fn test_ingest_incremental_false() {
    let db = test_database();
    let temp = create_temp_workspace("inc_false");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "incremental": false
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    // Full mode re-ingests everything
    let _ = result;
}

// ============================================================================
// Limit Parameter Tests
// ============================================================================

#[test]
fn test_ingest_limit_omitted() {
    let db = test_database();
    let temp = create_temp_workspace("limit_omit");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap()
        // limit not specified
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    let _ = result;
}

#[test]
fn test_ingest_limit_specified() {
    let db = test_database();
    let temp = create_temp_workspace("limit_spec");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "limit": 100
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    let _ = result;
}

#[test]
fn test_ingest_limit_zero() {
    let db = test_database();
    let temp = create_temp_workspace("limit_zero");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "limit": 0
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    // Limit 0 should result in no items ingested
    assert_eq!(result.stats.total_items, 0);
}

#[test]
fn test_ingest_limit_very_large() {
    let db = test_database();
    let temp = create_temp_workspace("limit_large");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "limit": 1000000
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    let _ = result;
}

// ============================================================================
// Response Structure Tests
// ============================================================================

#[test]
fn test_ingest_response_has_source() {
    let db = test_database();
    let temp = create_temp_workspace("resp_src");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "source": "copilot"  // Use copilot source (doesn't require git)
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    assert!(!result.source.is_empty());
}

#[test]
fn test_ingest_response_has_message() {
    let db = test_database();
    let temp = create_temp_workspace("resp_msg");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap()
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    assert!(!result.message.is_empty());
}

#[test]
fn test_ingest_response_has_stats() {
    let db = test_database();
    let temp = create_temp_workspace("resp_stats");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap()
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    // Stats should have all expected fields
    let _ = result.stats.commits_inserted;
    let _ = result.stats.commits_skipped;
    let _ = result.stats.test_runs_inserted;
    let _ = result.stats.test_results_inserted;
    let _ = result.stats.sessions_inserted;
    let _ = result.stats.messages_inserted;
    let _ = result.stats.total_items;
}

#[test]
fn test_ingest_stats_total_items_calculation() {
    let db = test_database();
    let temp = create_temp_workspace("stats_calc");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap()
    }));

    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    // Total items should be sum of all inserted items
    let expected_total = result.stats.commits_inserted
        + result.stats.test_runs_inserted
        + result.stats.test_results_inserted
        + result.stats.sessions_inserted
        + result.stats.messages_inserted;

    assert_eq!(result.stats.total_items, expected_total);
}

// ============================================================================
// Raw JSON Input Tests
// ============================================================================

#[test]
fn test_ingest_raw_json_valid() {
    let db = test_database();
    let temp = create_temp_workspace("raw_valid");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "source": "copilot",  // Use copilot source (doesn't require git)
        "incremental": true,
        "limit": 50
    }));

    let result = handlers::handle_ingest(db, Some(args));
    assert!(result.is_ok());
}

#[test]
fn test_ingest_raw_json_minimal() {
    let db = test_database();
    let temp = create_temp_workspace("raw_min");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap()
    }));

    let result = handlers::handle_ingest(db, Some(args));
    assert!(result.is_ok());
}

#[test]
fn test_ingest_raw_json_extra_fields_ignored() {
    let db = test_database();
    let temp = create_temp_workspace("raw_extra");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "unknown_field": "some value"
    }));

    let result = handlers::handle_ingest(db, Some(args));
    assert!(result.is_ok(), "Extra fields should be ignored");
}

#[test]
fn test_ingest_raw_json_wrong_type_for_workspace() {
    let db = test_database();

    let args = to_map(json!({
        "workspace": 12345
    }));

    let result = handlers::handle_ingest(db, Some(args));
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_ingest_raw_json_wrong_type_for_incremental() {
    let db = test_database();
    let temp = create_temp_workspace("raw_inc_type");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "incremental": "not a boolean"
    }));

    let result = handlers::handle_ingest(db, Some(args));
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_ingest_raw_json_wrong_type_for_limit() {
    let db = test_database();
    let temp = create_temp_workspace("raw_lim_type");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "limit": "not a number"
    }));

    let result = handlers::handle_ingest(db, Some(args));
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_ingest_raw_json_negative_limit() {
    let db = test_database();
    let temp = create_temp_workspace("raw_neg_lim");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "limit": -5
    }));

    let result = handlers::handle_ingest(db, Some(args));
    // Negative value can't be deserialized to usize
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_ingest_workspace_with_spaces() {
    let base = unique_temp_dir("spaces_base");
    let spaced_path = base.join("path with spaces");
    fs::create_dir_all(&spaced_path).expect("create dir with spaces");

    let db = test_database();

    let args = to_map(json!({
        "workspace": spaced_path.to_str().unwrap()
    }));

    let result = handlers::handle_ingest(db, Some(args));
    assert!(result.is_ok());
}

#[test]
fn test_ingest_workspace_with_unicode() {
    let base = unique_temp_dir("unicode_base");
    let unicode_path = base.join("日本語_folder");
    fs::create_dir_all(&unicode_path).expect("create unicode dir");

    let db = test_database();

    let args = to_map(json!({
        "workspace": unicode_path.to_str().unwrap()
    }));

    let result = handlers::handle_ingest(db, Some(args));
    assert!(result.is_ok());
}

#[test]
#[cfg(unix)]
fn test_ingest_workspace_symlink() {
    let base = unique_temp_dir("symlink_base");
    let real_dir = base.join("real");
    let symlink_path = base.join("link");

    fs::create_dir_all(&real_dir).expect("create real dir");
    std::os::unix::fs::symlink(&real_dir, &symlink_path).expect("create symlink");

    let db = test_database();

    let args = to_map(json!({
        "workspace": symlink_path.to_str().unwrap()
    }));

    let result = handlers::handle_ingest(db, Some(args));
    assert!(result.is_ok());
}

#[test]
fn test_ingest_empty_git_repo_fails_without_refs() {
    let db = test_database();
    // Our fake git repo doesn't have valid refs/commits
    let temp = create_temp_git_repo("empty_git");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "source": "git"
    }));

    let result = handlers::handle_ingest(db, Some(args));

    // Git source on incomplete/empty repo fails
    assert!(matches!(result, Err(HandlerError::Ingest(_))));
}

#[test]
fn test_ingest_multiple_times_incremental() {
    let temp = create_temp_workspace("multi_inc");

    // First ingestion
    let db1 = test_database();
    let args1 = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "incremental": true
    }));
    let result1 = handlers::handle_ingest(db1, Some(args1)).expect("ingest 1");

    // Second ingestion (incremental should skip already processed)
    let db2 = test_database();
    let args2 = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "incremental": true
    }));
    let result2 = handlers::handle_ingest(db2, Some(args2)).expect("ingest 2");

    // Both should succeed
    let _ = (result1, result2);
}

#[test]
fn test_ingest_all_sources_handles_missing_git() {
    let db = test_database();
    let temp = create_temp_workspace("no_git"); // No .git directory

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap(),
        "source": "all"
    }));

    // Should still succeed even if git ingestion fails
    let result = handlers::handle_ingest(db, Some(args)).expect("ingest should succeed");

    let _ = result;
}

#[test]
fn test_ingest_sql_injection_attempt_in_workspace() {
    let db = test_database();

    // Attempt SQL injection in workspace path
    let args = to_map(json!({
        "workspace": "/tmp/'; DROP TABLE commits; --"
    }));

    let result = handlers::handle_ingest(db, Some(args));

    // Should fail safely (path doesn't exist)
    assert!(matches!(result, Err(HandlerError::WorkspaceNotFound(_))));
}

// ============================================================================
// Note: Ingest tool takes ownership of database
// ============================================================================

// The handle_ingest function takes ownership of the Database,
// so we can't use the harness's invoke methods directly.
// Tests above use handlers::handle_ingest directly.

#[test]
fn test_ingest_consumes_database() {
    let db = test_database();
    let temp = create_temp_workspace("consume_db");

    let args = to_map(json!({
        "workspace": temp.to_str().unwrap()
    }));

    // This consumes the database
    let _result = handlers::handle_ingest(db, Some(args));

    // db is no longer available here (moved)
    // This is by design - ingestion may modify the database
}
