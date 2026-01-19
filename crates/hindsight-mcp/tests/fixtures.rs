// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Test fixtures for hindsight-mcp integration tests
//!
//! This module provides helper functions for creating test databases,
//! generating sample data, and ensuring test isolation.

use chrono::{DateTime, TimeZone, Utc};
use hindsight_git::commit::Commit;
use hindsight_mcp::db::{
    CommitRecord, CopilotMessageRecord, CopilotSessionRecord, Database, TestResultRecord,
    TestRunRecord, WorkspaceRecord,
};

// ============================================================================
// Database Fixtures
// ============================================================================

/// Create an initialized in-memory database for testing
///
/// This is the primary helper for creating test databases.
/// The database is fully initialized with the schema applied.
pub fn test_database() -> Database {
    let db = Database::in_memory().expect("Failed to create in-memory database");
    db.initialize().expect("Failed to initialize database");
    db
}

/// Create a database pre-populated with sample data
///
/// This creates:
/// - 1 workspace
/// - 5 commits
/// - 2 test runs (1 with failures)
/// - 10 test results
/// - 1 copilot session with 4 messages
pub fn populated_database() -> (Database, TestData) {
    let mut db = test_database();
    let data = TestData::default();

    // Insert workspace
    db.insert_workspace(&data.workspace)
        .expect("Failed to insert workspace");

    // Insert commits
    for commit in &data.commits {
        db.insert_commit(commit).expect("Failed to insert commit");
    }

    // Insert test runs and results
    for run in &data.test_runs {
        db.insert_test_run(run).expect("Failed to insert test run");
    }

    db.insert_test_results_batch(&data.test_results)
        .expect("Failed to insert test results");

    // Insert copilot session and messages
    db.insert_copilot_session(&data.copilot_session)
        .expect("Failed to insert copilot session");

    db.insert_copilot_messages_batch(&data.copilot_messages)
        .expect("Failed to insert copilot messages");

    (db, data)
}

// ============================================================================
// Test Data Structures
// ============================================================================

/// Container for all test data inserted into a populated database
#[derive(Debug, Clone)]
pub struct TestData {
    pub workspace: WorkspaceRecord,
    pub commits: Vec<CommitRecord>,
    pub test_runs: Vec<TestRunRecord>,
    pub test_results: Vec<TestResultRecord>,
    pub copilot_session: CopilotSessionRecord,
    pub copilot_messages: Vec<CopilotMessageRecord>,
}

impl Default for TestData {
    fn default() -> Self {
        let base_time = Utc.with_ymd_and_hms(2026, 1, 17, 12, 0, 0).unwrap();
        let _workspace_id = test_uuid("workspace1");

        let workspace =
            WorkspaceRecord::new("test-project".to_string(), "/tmp/test-project".to_string());
        let workspace_id_actual = workspace.id.clone();

        // Generate 5 commits
        let commits: Vec<CommitRecord> = (0..5)
            .map(|i| {
                sample_commit(
                    &workspace_id_actual,
                    &format!("{:0>40}", format!("abc{}", i)),
                    &format!("Commit message {}", i),
                    hours_ago(base_time, (4 - i) as i64),
                )
            })
            .collect();

        // Generate 2 test runs
        let test_runs = vec![
            sample_test_run(&workspace_id_actual, Some(&commits[3].sha), 10, 0, 1),
            sample_test_run(&workspace_id_actual, Some(&commits[4].sha), 8, 2, 0),
        ];

        // Generate test results (passing and failing)
        let mut test_results = Vec::new();
        for (i, run) in test_runs.iter().enumerate() {
            let passed = if i == 0 { 10 } else { 8 };
            let failed = if i == 0 { 0 } else { 2 };

            for j in 0..passed {
                test_results.push(sample_test_result(
                    &run.id,
                    &format!("test_passes_{}", j),
                    "passed",
                    None,
                ));
            }
            for j in 0..failed {
                test_results.push(sample_test_result(
                    &run.id,
                    &format!("test_fails_{}", j),
                    "failed",
                    Some("assertion failed: expected true, got false"),
                ));
            }
        }

        // Generate copilot session with messages
        let copilot_session = sample_copilot_session(&workspace_id_actual, hours_ago(base_time, 1));

        let copilot_messages = vec![
            sample_copilot_message(
                &copilot_session.id,
                "user",
                "How do I implement caching?",
                0,
            ),
            sample_copilot_message(
                &copilot_session.id,
                "assistant",
                "You can use a HashMap with an LRU eviction policy.",
                1,
            ),
            sample_copilot_message(&copilot_session.id, "user", "Show me an example in Rust", 2),
            sample_copilot_message(
                &copilot_session.id,
                "assistant",
                "```rust\nuse std::collections::HashMap;\n```",
                3,
            ),
        ];

        Self {
            workspace: WorkspaceRecord {
                id: workspace_id_actual,
                ..workspace
            },
            commits,
            test_runs,
            test_results,
            copilot_session,
            copilot_messages,
        }
    }
}

// ============================================================================
// Sample Data Generators
// ============================================================================

/// Generate a UUID-like string for testing
///
/// Format: `00000000-0000-0000-0000-{suffix padded to 12 chars}`
pub fn test_uuid(suffix: &str) -> String {
    format!("00000000-0000-0000-0000-{:0>12}", suffix)
}

/// Generate a sample commit record
pub fn sample_commit(
    workspace_id: &str,
    sha: &str,
    message: &str,
    timestamp: DateTime<Utc>,
) -> CommitRecord {
    CommitRecord::new(
        workspace_id.to_string(),
        sha.to_string(),
        "Test Author".to_string(),
        Some("test@example.com".to_string()),
        message.to_string(),
        timestamp,
    )
}

/// Generate a sample test run record
pub fn sample_test_run(
    workspace_id: &str,
    commit_sha: Option<&str>,
    passed: i32,
    failed: i32,
    ignored: i32,
) -> TestRunRecord {
    let mut run = TestRunRecord::new(workspace_id.to_string());
    if let Some(sha) = commit_sha {
        run = run.with_commit(sha);
    }
    run.finished(passed, failed, ignored)
}

/// Generate a sample test result record
pub fn sample_test_result(
    run_id: &str,
    test_name: &str,
    outcome: &str,
    error_output: Option<&str>,
) -> TestResultRecord {
    let mut result = TestResultRecord::new(
        run_id.to_string(),
        "test_suite".to_string(),
        test_name.to_string(),
        outcome.to_string(),
        Some(42),
    );
    if let Some(output) = error_output {
        result = result.with_output(Some(output), None);
    }
    result
}

/// Generate a sample Copilot session record
pub fn sample_copilot_session(
    workspace_id: &str,
    created_at: DateTime<Utc>,
) -> CopilotSessionRecord {
    CopilotSessionRecord {
        id: test_uuid("session1"),
        workspace_id: workspace_id.to_string(),
        vscode_session_id: "vscode-session-12345".to_string(),
        created_at,
        updated_at: created_at,
        metadata_json: Some(r#"{"version":3}"#.to_string()),
    }
}

/// Generate a sample Copilot message record
pub fn sample_copilot_message(
    session_id: &str,
    role: &str,
    content: &str,
    _turn: i32,
) -> CopilotMessageRecord {
    CopilotMessageRecord::new(
        session_id.to_string(),
        role.to_string(),
        content.to_string(),
        Utc::now(),
    )
}

/// Generate a sample git Commit (from hindsight-git crate)
#[allow(dead_code)]
pub fn sample_git_commit(sha: &str, message: &str, timestamp: DateTime<Utc>) -> Commit {
    Commit {
        sha: sha.to_string(),
        message: message.to_string(),
        author: "Test Author".to_string(),
        author_email: "test@example.com".to_string(),
        timestamp,
        parents: vec![],
    }
}

// ============================================================================
// Time Helpers
// ============================================================================

/// Get a timestamp N hours ago from a base time
pub fn hours_ago(base: DateTime<Utc>, hours: i64) -> DateTime<Utc> {
    base - chrono::Duration::hours(hours)
}

/// Get a timestamp N days ago from a base time
pub fn days_ago(base: DateTime<Utc>, days: i64) -> DateTime<Utc> {
    base - chrono::Duration::days(days)
}

/// Get the current time as a base for relative timestamps
#[allow(dead_code)]
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

// ============================================================================
// Assertion Helpers
// ============================================================================

/// Assert that the database has the expected number of records in a table
pub fn assert_table_count(db: &Database, table: &str, expected: usize) {
    let count: i64 = db
        .connection()
        .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
            row.get(0)
        })
        .expect("Failed to query table count");

    assert_eq!(
        count as usize, expected,
        "Expected {} records in {}, got {}",
        expected, table, count
    );
}

/// Assert that a workspace exists with the given path
#[allow(dead_code)]
pub fn assert_workspace_exists(db: &Database, path: &str) {
    let count: i64 = db
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM workspaces WHERE path = ?",
            [path],
            |row| row.get(0),
        )
        .expect("Failed to query workspace");

    assert!(
        count > 0,
        "Expected workspace with path '{}' to exist",
        path
    );
}

/// Assert that a commit exists with the given SHA
#[allow(dead_code)]
pub fn assert_commit_exists(db: &Database, sha: &str) {
    let count: i64 = db
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM commits WHERE sha LIKE ?",
            [format!("{}%", sha)],
            |row| row.get(0),
        )
        .expect("Failed to query commit");

    assert!(count > 0, "Expected commit with SHA '{}' to exist", sha);
}

// ============================================================================
// Unit Tests for Fixtures
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_test_database_creates_valid_db() {
        let db = test_database();
        assert!(db.is_initialized());
    }

    #[test]
    fn test_fixtures_populated_database_has_data() {
        let (db, data) = populated_database();

        assert!(db.is_initialized());
        assert!(!data.workspace.id.is_empty());
        assert_eq!(data.commits.len(), 5);
        assert_eq!(data.test_runs.len(), 2);
        assert!(!data.test_results.is_empty());
        assert!(!data.copilot_messages.is_empty());
    }

    #[test]
    fn test_fixtures_test_uuid_format() {
        let uuid = test_uuid("abc");
        assert_eq!(uuid.len(), 36);
        assert!(uuid.starts_with("00000000-0000-0000-0000-"));
        assert!(uuid.ends_with("abc"));
    }

    #[test]
    fn test_fixtures_time_helpers() {
        let base = Utc.with_ymd_and_hms(2026, 1, 17, 12, 0, 0).unwrap();

        let one_hour_ago = hours_ago(base, 1);
        assert_eq!(
            one_hour_ago,
            Utc.with_ymd_and_hms(2026, 1, 17, 11, 0, 0).unwrap()
        );

        let one_day_ago = days_ago(base, 1);
        assert_eq!(
            one_day_ago,
            Utc.with_ymd_and_hms(2026, 1, 16, 12, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_fixtures_assert_table_count() {
        let (db, _) = populated_database();
        assert_table_count(&db, "workspaces", 1);
        assert_table_count(&db, "commits", 5);
    }
}
