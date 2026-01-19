// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! End-to-end workflow tests for hindsight-mcp
//!
//! These tests verify complete real-world workflows from CLI to MCP tools,
//! ensuring all components work together correctly.

mod fixtures;
mod mcp_harness;
mod test_utils;

use chrono::Utc;

use fixtures::{
    days_ago, hours_ago, sample_copilot_message, sample_copilot_session, sample_test_result,
    sample_test_run, test_database,
};
use hindsight_mcp::ingest::{IngestOptions, Ingestor};
use mcp_harness::McpTestHarness;
use test_utils::TestGitRepo;

// ============================================================================
// Workflow 1: Fresh Setup to First Query
// ============================================================================

/// Complete workflow: Create database, ingest git history, query timeline
#[test]
fn e2e_fresh_setup_to_first_query() {
    // Step 1: Create a temporary git repository with commits
    let mut repo = TestGitRepo::new("e2e_fresh_setup");
    repo.init();

    let _sha1 = repo.create_and_commit("README.md", "# My Project", "Initial commit");
    let _sha2 = repo.create_and_commit("src/main.rs", "fn main() {}", "Add main.rs");
    let _sha3 =
        repo.create_and_commit("Cargo.toml", "[package]\nname = \"test\"", "Add Cargo.toml");

    // Step 2: Create a fresh database
    let db = test_database();

    // Step 3: Ingest git history
    let mut ingestor = Ingestor::new(db);
    let stats = ingestor
        .ingest_git(repo.path(), &IngestOptions::incremental())
        .expect("git ingestion should succeed");

    assert_eq!(stats.commits_inserted, 3);

    // Step 4: Query timeline via MCP tool handler
    let db = ingestor.into_database();
    let harness = McpTestHarness::new(db);

    let timeline = harness
        .timeline(Some(10), Some(repo.path().to_str().unwrap()))
        .expect("timeline query should succeed");

    // Should have 3 events (commits)
    assert_eq!(timeline.len(), 3);

    // Verify all commit summaries are present (order may vary based on git timing)
    let summaries: Vec<&str> = timeline.iter().map(|e| e.summary.as_str()).collect();
    assert!(summaries.iter().any(|s| s.contains("Initial commit")));
    assert!(summaries.iter().any(|s| s.contains("Add main.rs")));
    assert!(summaries.iter().any(|s| s.contains("Add Cargo.toml")));
}

/// Fresh setup with empty repository
#[test]
fn e2e_fresh_setup_empty_repo() {
    // Create empty git repo (no commits)
    let mut repo = TestGitRepo::new("e2e_fresh_empty");
    repo.init();

    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    // Empty repos may fail with "reference not found" since there's no HEAD
    // This is expected behavior - just verify we handle it gracefully
    let result = ingestor.ingest_git(repo.path(), &IngestOptions::incremental());

    // Either it succeeds with 0 commits or fails gracefully
    match result {
        Ok(stats) => assert_eq!(stats.commits_inserted, 0),
        Err(_) => {
            // Expected - empty repos have no main/HEAD reference
        }
    }

    let db = ingestor.into_database();
    let harness = McpTestHarness::new(db);

    let timeline = harness
        .timeline(Some(10), None)
        .expect("timeline should work on empty db");

    assert_eq!(timeline.len(), 0);
}

// ============================================================================
// Workflow 2: Ingest Git History and Search Commits
// ============================================================================

/// Complete workflow: Ingest commits and search for specific content
#[test]
fn e2e_ingest_git_search_commits() {
    // Step 1: Create repository with specific commit messages
    let mut repo = TestGitRepo::new("e2e_search_commits");
    repo.init();

    repo.create_and_commit(
        "file1.txt",
        "content1",
        "feat: implement user authentication",
    );
    repo.create_and_commit(
        "file2.txt",
        "content2",
        "fix: resolve database connection issue",
    );
    repo.create_and_commit("file3.txt", "content3", "docs: update README with examples");
    repo.create_and_commit(
        "file4.txt",
        "content4",
        "feat: add caching layer for performance",
    );

    // Step 2: Ingest into database
    let db = test_database();
    let mut ingestor = Ingestor::new(db);
    ingestor
        .ingest_git(repo.path(), &IngestOptions::incremental())
        .expect("ingestion should succeed");

    let db = ingestor.into_database();
    let harness = McpTestHarness::new(db);

    // Step 3: Search for "authentication"
    let auth_results = harness
        .search("authentication", Some("commits"), Some(10))
        .expect("search should succeed");

    assert_eq!(auth_results.len(), 1);
    assert!(auth_results[0].snippet.contains("authentication"));

    // Step 4: Search for "database"
    let db_results = harness
        .search("database", Some("commits"), Some(10))
        .expect("search should succeed");

    assert_eq!(db_results.len(), 1);
    assert!(db_results[0].snippet.contains("database"));

    // Step 5: Search for "feat" (should find 2 commits)
    let feat_results = harness
        .search("feat", Some("commits"), Some(10))
        .expect("search should succeed");

    assert_eq!(feat_results.len(), 2);
}

/// Search with no matching results
#[test]
fn e2e_search_no_results() {
    let mut repo = TestGitRepo::new("e2e_search_empty");
    repo.init();
    repo.create_and_commit("file.txt", "data", "simple commit message");

    let db = test_database();
    let mut ingestor = Ingestor::new(db);
    ingestor
        .ingest_git(repo.path(), &IngestOptions::incremental())
        .unwrap();

    let db = ingestor.into_database();
    let harness = McpTestHarness::new(db);

    let results = harness
        .search("nonexistent_keyword_xyz", Some("all"), Some(10))
        .expect("search should succeed");

    assert_eq!(results.len(), 0);
}

/// Search across both commits and messages
#[test]
fn e2e_search_all_sources() {
    let db = test_database();
    let mut db = db;

    // Create workspace
    let _workspace_id = "test-ws";
    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test".to_string(),
    ))
    .unwrap();

    // Insert a commit with "caching" in the message
    let commit = fixtures::sample_commit(
        &db.list_workspaces().unwrap()[0].id,
        "abc123def456789",
        "feat: add caching layer",
        Utc::now(),
    );
    db.insert_commit(&commit).unwrap();

    // Insert a copilot session with "caching" in a message
    let ws_id = &db.list_workspaces().unwrap()[0].id;
    let session = sample_copilot_session(ws_id, Utc::now());
    db.insert_copilot_session(&session).unwrap();

    let message = sample_copilot_message(&session.id, "user", "How do I implement caching?", 0);
    db.insert_copilot_messages_batch(&[message]).unwrap();

    let harness = McpTestHarness::new(db);

    // Search "all" should find both
    let results = harness
        .search("caching", Some("all"), Some(10))
        .expect("search should succeed");

    assert!(results.len() >= 2); // At least commit + message
}

// ============================================================================
// Workflow 3: Test Run with Failures, Query Failing Tests
// ============================================================================

/// Complete workflow: Ingest test results with failures, query failing tests
#[test]
fn e2e_test_run_query_failures() {
    let db = test_database();
    let mut db = db;

    // Create workspace
    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test-project".to_string(),
    ))
    .unwrap();

    let ws_id = db.list_workspaces().unwrap()[0].id.clone();

    // Create a test run with some failures
    let test_run = sample_test_run(&ws_id, Some("commit123"), 8, 2, 0);
    db.insert_test_run(&test_run).unwrap();

    // Insert test results - 8 passing, 2 failing
    let mut results = Vec::new();
    for i in 0..8 {
        results.push(sample_test_result(
            &test_run.id,
            &format!("test_passes_{}", i),
            "passed",
            None,
        ));
    }
    results.push(sample_test_result(
        &test_run.id,
        "test_authentication_fails",
        "failed",
        Some("assertion failed: expected 200, got 401"),
    ));
    results.push(sample_test_result(
        &test_run.id,
        "test_database_connection_fails",
        "failed",
        Some("connection timeout: unable to reach database"),
    ));

    db.insert_test_results_batch(&results).unwrap();

    // Query failing tests
    let harness = McpTestHarness::new(db);

    let failing = harness
        .failing_tests(Some(50), None, None)
        .expect("failing tests query should succeed");

    assert_eq!(failing.len(), 2);

    // Check that error output is included
    // Note: full_name contains the actual test name (test_name in view is the UUID)
    let auth_failure = failing
        .iter()
        .find(|f| f.full_name.contains("authentication"));
    assert!(auth_failure.is_some());
    assert!(
        auth_failure
            .unwrap()
            .output_json
            .as_ref()
            .is_some_and(|o| o.contains("401"))
    );

    let db_failure = failing.iter().find(|f| f.full_name.contains("database"));
    assert!(db_failure.is_some());
    assert!(
        db_failure
            .unwrap()
            .output_json
            .as_ref()
            .is_some_and(|o| o.contains("timeout"))
    );
}

/// Query failing tests with commit filter
#[test]
fn e2e_failing_tests_by_commit() {
    let db = test_database();
    let mut db = db;

    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test".to_string(),
    ))
    .unwrap();

    let ws_id = db.list_workspaces().unwrap()[0].id.clone();

    // Create two test runs with different commits
    let run1 = sample_test_run(&ws_id, Some("aaaa1111"), 9, 1, 0);
    let run2 = sample_test_run(&ws_id, Some("bbbb2222"), 8, 2, 0);

    db.insert_test_run(&run1).unwrap();
    db.insert_test_run(&run2).unwrap();

    // Run 1: 1 failure
    db.insert_test_results_batch(&[sample_test_result(
        &run1.id,
        "test_from_run1_fails",
        "failed",
        Some("error in run 1"),
    )])
    .unwrap();

    // Run 2: 2 failures
    db.insert_test_results_batch(&[
        sample_test_result(
            &run2.id,
            "test_from_run2_fails_a",
            "failed",
            Some("error in run 2a"),
        ),
        sample_test_result(
            &run2.id,
            "test_from_run2_fails_b",
            "failed",
            Some("error in run 2b"),
        ),
    ])
    .unwrap();

    let harness = McpTestHarness::new(db);

    // Filter by commit aaaa1111 - should get 1 failure
    let failing_run1 = harness
        .failing_tests(Some(50), None, Some("aaaa1111"))
        .expect("query should succeed");

    assert_eq!(failing_run1.len(), 1);
    assert!(failing_run1[0].full_name.contains("run1"));

    // Filter by commit bbbb2222 - should get 2 failures
    let failing_run2 = harness
        .failing_tests(Some(50), None, Some("bbbb2222"))
        .expect("query should succeed");

    assert_eq!(failing_run2.len(), 2);
}

/// No failing tests in a passing run
#[test]
fn e2e_all_tests_pass() {
    let db = test_database();
    let mut db = db;

    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test".to_string(),
    ))
    .unwrap();

    let ws_id = db.list_workspaces().unwrap()[0].id.clone();
    let run = sample_test_run(&ws_id, None, 10, 0, 0);
    db.insert_test_run(&run).unwrap();

    for i in 0..10 {
        db.insert_test_results_batch(&[sample_test_result(
            &run.id,
            &format!("test_passes_{}", i),
            "passed",
            None,
        )])
        .unwrap();
    }

    let harness = McpTestHarness::new(db);
    let failing = harness
        .failing_tests(Some(50), None, None)
        .expect("query should succeed");

    assert_eq!(failing.len(), 0);
}

// ============================================================================
// Workflow 4: Activity Summary Across Time Periods
// ============================================================================

/// Complete workflow: Create commits at various timestamps, verify activity summary
#[test]
fn e2e_activity_summary_time_periods() {
    let db = test_database();
    let db = db;

    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test".to_string(),
    ))
    .unwrap();

    let ws_id = db.list_workspaces().unwrap()[0].id.clone();
    let now = Utc::now();

    // Create commits at different times:
    // - 3 commits today
    // - 2 commits 3 days ago
    // - 1 commit 10 days ago
    // - 1 commit 40 days ago

    let commits = vec![
        fixtures::sample_commit(&ws_id, "today1", "commit today 1", hours_ago(now, 1)),
        fixtures::sample_commit(&ws_id, "today2", "commit today 2", hours_ago(now, 2)),
        fixtures::sample_commit(&ws_id, "today3", "commit today 3", hours_ago(now, 3)),
        fixtures::sample_commit(&ws_id, "3daysago1", "commit 3 days ago 1", days_ago(now, 3)),
        fixtures::sample_commit(&ws_id, "3daysago2", "commit 3 days ago 2", days_ago(now, 3)),
        fixtures::sample_commit(&ws_id, "10daysago", "commit 10 days ago", days_ago(now, 10)),
        fixtures::sample_commit(&ws_id, "40daysago", "commit 40 days ago", days_ago(now, 40)),
    ];

    for commit in &commits {
        db.insert_commit(commit).unwrap();
    }

    let harness = McpTestHarness::new(db);

    // Last 1 day: 3 commits
    let summary_1d = harness
        .activity_summary(Some(1))
        .expect("summary should succeed");
    assert_eq!(summary_1d.commits, 3);

    // Last 7 days: 5 commits (3 today + 2 from 3 days ago)
    let summary_7d = harness
        .activity_summary(Some(7))
        .expect("summary should succeed");
    assert_eq!(summary_7d.commits, 5);

    // Last 30 days: 6 commits (5 + 1 from 10 days ago)
    let summary_30d = harness
        .activity_summary(Some(30))
        .expect("summary should succeed");
    assert_eq!(summary_30d.commits, 6);

    // Last 365 days: 7 commits (all)
    let summary_year = harness
        .activity_summary(Some(365))
        .expect("summary should succeed");
    assert_eq!(summary_year.commits, 7);
}

/// Activity summary includes test runs
#[test]
fn e2e_activity_summary_with_tests() {
    let db = test_database();
    let db = db;

    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test".to_string(),
    ))
    .unwrap();

    let ws_id = db.list_workspaces().unwrap()[0].id.clone();

    // Create test runs
    let run1 = sample_test_run(&ws_id, None, 10, 0, 0);
    let run2 = sample_test_run(&ws_id, None, 8, 2, 0);

    db.insert_test_run(&run1).unwrap();
    db.insert_test_run(&run2).unwrap();

    let harness = McpTestHarness::new(db);

    let summary = harness
        .activity_summary(Some(7))
        .expect("summary should succeed");

    assert_eq!(summary.test_runs, 2);
}

/// Activity summary with copilot sessions
#[test]
fn e2e_activity_summary_with_copilot() {
    let db = test_database();
    let db = db;

    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test".to_string(),
    ))
    .unwrap();

    let ws_id = db.list_workspaces().unwrap()[0].id.clone();
    let now = Utc::now();

    // Create copilot sessions
    let session1 = sample_copilot_session(&ws_id, hours_ago(now, 1));
    let session2 = hindsight_mcp::db::CopilotSessionRecord {
        id: fixtures::test_uuid("session2"),
        workspace_id: ws_id.clone(),
        vscode_session_id: "vscode-session-67890".to_string(),
        created_at: hours_ago(now, 2),
        updated_at: hours_ago(now, 2),
        metadata_json: None,
    };

    db.insert_copilot_session(&session1).unwrap();
    db.insert_copilot_session(&session2).unwrap();

    let harness = McpTestHarness::new(db);

    let summary = harness
        .activity_summary(Some(7))
        .expect("summary should succeed");

    assert_eq!(summary.copilot_sessions, 2);
}

// ============================================================================
// Workflow 5: Copilot Session Ingestion and Search
// ============================================================================

/// Complete workflow: Ingest copilot sessions and search for message content
#[test]
fn e2e_copilot_session_search() {
    let db = test_database();
    let mut db = db;

    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test".to_string(),
    ))
    .unwrap();

    let ws_id = db.list_workspaces().unwrap()[0].id.clone();

    // Create a copilot session with messages
    let session = sample_copilot_session(&ws_id, Utc::now());
    db.insert_copilot_session(&session).unwrap();

    let messages = vec![
        sample_copilot_message(
            &session.id,
            "user",
            "How do I implement error handling in Rust?",
            0,
        ),
        sample_copilot_message(
            &session.id,
            "assistant",
            "In Rust, you can use the Result type for error handling. Here's an example using the ? operator.",
            1,
        ),
        sample_copilot_message(&session.id, "user", "What about custom error types?", 2),
        sample_copilot_message(
            &session.id,
            "assistant",
            "You can create custom error types by implementing the std::error::Error trait or using thiserror crate.",
            3,
        ),
    ];

    db.insert_copilot_messages_batch(&messages).unwrap();

    let harness = McpTestHarness::new(db);

    // Search for "error handling"
    let results = harness
        .search("error handling", Some("messages"), Some(10))
        .expect("search should succeed");

    assert!(!results.is_empty());
    assert!(
        results
            .iter()
            .any(|r| r.snippet.contains("error") || r.snippet.contains("Error"))
    );

    // Search for "Result type"
    let results = harness
        .search("Result", Some("messages"), Some(10))
        .expect("search should succeed");

    assert!(!results.is_empty());

    // Search for "thiserror"
    let results = harness
        .search("thiserror", Some("messages"), Some(10))
        .expect("search should succeed");

    assert!(!results.is_empty());
}

/// Search across multiple copilot sessions
#[test]
fn e2e_multiple_copilot_sessions() {
    let db = test_database();
    let mut db = db;

    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test".to_string(),
    ))
    .unwrap();

    let ws_id = db.list_workspaces().unwrap()[0].id.clone();
    let now = Utc::now();

    // Session 1: About testing
    let session1 = sample_copilot_session(&ws_id, hours_ago(now, 2));
    db.insert_copilot_session(&session1).unwrap();
    db.insert_copilot_messages_batch(&[
        sample_copilot_message(&session1.id, "user", "How do I write unit tests?", 0),
        sample_copilot_message(
            &session1.id,
            "assistant",
            "You can use the #[test] attribute in Rust.",
            1,
        ),
    ])
    .unwrap();

    // Session 2: About documentation
    let session2 = hindsight_mcp::db::CopilotSessionRecord {
        id: fixtures::test_uuid("session2"),
        workspace_id: ws_id.clone(),
        vscode_session_id: "vscode-session-docs".to_string(),
        created_at: hours_ago(now, 1),
        updated_at: hours_ago(now, 1),
        metadata_json: None,
    };
    db.insert_copilot_session(&session2).unwrap();
    db.insert_copilot_messages_batch(&[
        sample_copilot_message(&session2.id, "user", "How do I write documentation?", 0),
        sample_copilot_message(
            &session2.id,
            "assistant",
            "Use /// for doc comments in Rust.",
            1,
        ),
    ])
    .unwrap();

    let harness = McpTestHarness::new(db);

    // Search for "unit tests" - should find session 1
    let test_results = harness
        .search("unit tests", Some("messages"), Some(10))
        .expect("search should succeed");

    assert!(!test_results.is_empty());

    // Search for "documentation" - should find session 2
    let doc_results = harness
        .search("documentation", Some("messages"), Some(10))
        .expect("search should succeed");

    assert!(!doc_results.is_empty());

    // Search for "Rust" - should find messages from both sessions
    let rust_results = harness
        .search("Rust", Some("messages"), Some(10))
        .expect("search should succeed");

    assert!(rust_results.len() >= 2);
}

/// Timeline includes copilot sessions
#[test]
fn e2e_timeline_with_copilot() {
    let db = test_database();
    let mut db = db;

    db.insert_workspace(&hindsight_mcp::db::WorkspaceRecord::new(
        "test-project".to_string(),
        "/tmp/test".to_string(),
    ))
    .unwrap();

    let ws_id = db.list_workspaces().unwrap()[0].id.clone();
    let now = Utc::now();

    // Create a commit
    let commit = fixtures::sample_commit(&ws_id, "abc123", "Add feature", hours_ago(now, 2));
    db.insert_commit(&commit).unwrap();

    // Create a copilot session with a message (messages appear in timeline, not sessions)
    let session = sample_copilot_session(&ws_id, hours_ago(now, 1));
    db.insert_copilot_session(&session).unwrap();
    let message = sample_copilot_message(&session.id, "user", "How do I add a feature?", 0);
    db.insert_copilot_messages_batch(&[message]).unwrap();

    // Create a test run
    let run = sample_test_run(&ws_id, None, 10, 0, 0);
    db.insert_test_run(&run).unwrap();

    let harness = McpTestHarness::new(db);

    let timeline = harness
        .timeline(Some(10), None)
        .expect("timeline should succeed");

    // Should have events for commit, copilot message, and test run
    assert!(timeline.len() >= 3);

    // Verify different event types exist
    let event_types: Vec<&str> = timeline.iter().map(|e| e.event_type.as_str()).collect();
    assert!(event_types.contains(&"commit"));
    assert!(event_types.contains(&"copilot_message"));
    assert!(event_types.contains(&"test_run"));
}

// ============================================================================
// Integration with Real Git Repos
// ============================================================================

/// Full workflow with real git repository and ingestion
#[test]
fn e2e_full_git_workflow() {
    let mut repo = TestGitRepo::new("e2e_full_git");
    repo.init();

    // Create a series of commits simulating development
    repo.create_and_commit(
        "README.md",
        "# Project\n\nA test project",
        "docs: add README",
    );
    repo.create_and_commit(
        "src/lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
        "feat: implement add function",
    );
    repo.create_and_commit(
        "src/lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub(a: i32, b: i32) -> i32 { a - b }",
        "feat: implement subtract function",
    );
    let last_sha = repo.create_and_commit(
        "tests/test_math.rs",
        "#[test] fn test_add() { assert_eq!(add(1, 2), 3); }",
        "test: add unit tests for math functions",
    );

    let db = test_database();
    let mut ingestor = Ingestor::new(db);
    let stats = ingestor
        .ingest_git(repo.path(), &IngestOptions::incremental())
        .expect("ingestion should succeed");

    assert_eq!(stats.commits_inserted, 4);

    let db = ingestor.into_database();
    let harness = McpTestHarness::new(db);

    // Verify timeline
    let timeline = harness.timeline(Some(10), None).expect("timeline");
    assert_eq!(timeline.len(), 4);

    // Verify search
    let search_results = harness
        .search("implement", Some("commits"), Some(10))
        .expect("search");
    assert_eq!(search_results.len(), 2); // "implement add" and "implement subtract"

    // Verify commit details
    let details = harness
        .commit_details(&last_sha[..7])
        .expect("commit details");
    assert!(details.message.contains("unit tests"));

    // Verify activity summary
    let summary = harness.activity_summary(Some(7)).expect("summary");
    assert_eq!(summary.commits, 4);
}

// ============================================================================
// Module Tests
// ============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn test_e2e_test_count() {
        // Placeholder test - the e2e tests above are the actual tests
        // This module exists to group any helper tests if needed
    }
}
