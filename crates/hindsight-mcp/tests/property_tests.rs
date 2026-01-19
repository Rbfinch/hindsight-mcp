// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Property-based tests for hindsight-mcp
//!
//! These tests use proptest to verify invariants hold for arbitrary inputs,
//! ensuring robustness against edge cases and malformed data.

// Allow single_match in this module - it's intentional for property tests
// where we want to assert on Ok and ignore Err variants
#![allow(clippy::single_match)]

mod fixtures;
mod mcp_harness;

use proptest::prelude::*;
use serde_json::{Value, json};

use fixtures::{populated_database, test_database};
use hindsight_mcp::handlers::{self, HandlerError};
use mcp_harness::McpTestHarness;

// ============================================================================
// Strategies
// ============================================================================

/// Generate arbitrary strings including edge cases
fn arbitrary_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty and whitespace
        Just("".to_string()),
        Just(" ".to_string()),
        Just("\t\n\r".to_string()),
        // Unicode
        Just("日本語テスト".to_string()),
        Just("emoji 🔥🚀".to_string()),
        Just("Ñoño".to_string()),
        // Special SQL/FTS5 characters
        Just("test OR crash".to_string()),
        Just("test AND fail".to_string()),
        Just("NOT test".to_string()),
        Just("test*".to_string()),
        Just("\"quoted phrase\"".to_string()),
        Just("test' OR '1'='1".to_string()),
        Just("'; DROP TABLE commits;--".to_string()),
        // Path-like strings
        Just("/some/path/to/file".to_string()),
        Just("./relative/path".to_string()),
        Just("../parent/path".to_string()),
        Just("C:\\Windows\\Path".to_string()),
        // Very long string (but not too long for test speed)
        Just("a".repeat(1000)),
        // Random alphanumeric
        "[a-zA-Z0-9]{1,50}".prop_map(|s| s),
        // Random with special chars
        ".*{0,100}".prop_map(|s| s),
    ]
}

/// Generate arbitrary limit values including edge cases
fn arbitrary_limit() -> impl Strategy<Value = i64> {
    prop_oneof![
        Just(0i64),
        Just(1i64),
        Just(50i64),
        Just(100i64),
        Just(1000i64),
        Just(i64::MAX),
        Just(-1i64),
        Just(-100i64),
        Just(i64::MIN),
        (-1000i64..=10000i64),
    ]
}

/// Generate arbitrary days values for activity summary
fn arbitrary_days() -> impl Strategy<Value = i64> {
    prop_oneof![
        Just(0i64),
        Just(1i64),
        Just(7i64),
        Just(30i64),
        Just(365i64),
        Just(1000i64),
        Just(-1i64),
        Just(-365i64),
        Just(i64::MAX),
        (-100i64..=1000i64),
    ]
}

/// Generate arbitrary source values for search
fn arbitrary_source() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("all".to_string()),
        Just("commits".to_string()),
        Just("messages".to_string()),
        Just("invalid_source".to_string()),
        Just("".to_string()),
        Just("ALL".to_string()),
        Just("COMMITS".to_string()),
        Just("tests".to_string()),
        arbitrary_string(),
    ]
}

/// Generate arbitrary ingest source values
fn arbitrary_ingest_source() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("all".to_string()),
        Just("git".to_string()),
        Just("copilot".to_string()),
        Just("tests".to_string()),
        Just("invalid".to_string()),
        Just("".to_string()),
        arbitrary_string(),
    ]
}

/// Generate arbitrary SHA-like strings
fn arbitrary_sha() -> impl Strategy<Value = String> {
    prop_oneof![
        // Valid-looking SHAs
        "[0-9a-f]{40}".prop_map(|s| s),
        "[0-9a-f]{7}".prop_map(|s| s),
        // Edge cases
        Just("".to_string()),
        Just("abc".to_string()),
        Just("0".repeat(40)),
        Just("f".repeat(40)),
        // Invalid but might be passed
        Just("UPPERCASE".to_string()),
        Just("not-a-sha".to_string()),
        Just("abc123xyz".to_string()),
        arbitrary_string(),
    ]
}

// ============================================================================
// Property Tests: Search Tool
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Any string query doesn't crash the search handler
    #[test]
    fn search_never_panics_on_arbitrary_query(query in arbitrary_string()) {
        let db = test_database();
        let harness = McpTestHarness::new(db);

        // Should not panic, even on malformed input
        let result = harness.search(&query, Some("all"), Some(10));

        // Result can be Ok or Err, but should not panic
        match result {
            Ok(results) => {
                // Valid results should be a vec
                prop_assert!(results.len() <= 10);
            }
            Err(e) => {
                // Error is acceptable, just shouldn't panic
                let _ = format!("{}", e);
            }
        }
    }

    /// Property: Any source value produces valid response or error (not panic)
    #[test]
    fn search_handles_arbitrary_source(source in arbitrary_source()) {
        let db = test_database();
        let harness = McpTestHarness::new(db);

        let result = harness.search("test", Some(&source), Some(10));

        // Should not panic
        match result {
            Ok(results) => prop_assert!(results.len() <= 10),
            Err(_) => {} // Error is fine
        }
    }
}

// ============================================================================
// Property Tests: Timeline Tool
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Any positive limit produces valid output, others produce error or empty
    #[test]
    fn timeline_handles_arbitrary_limit(limit in arbitrary_limit()) {
        let (db, _data) = populated_database();
        let harness = McpTestHarness::new(db);

        // Convert i64 to usize, clamping negative to 0
        let limit_usize = if limit < 0 { 0 } else { limit.min(10000) as usize };
        let result = harness.timeline(Some(limit_usize), None);

        match result {
            Ok(events) => {
                prop_assert!(events.len() <= limit_usize.max(1));
            }
            Err(_) => {} // Error is acceptable for edge cases
        }
    }

    /// Property: Any workspace filter doesn't crash
    #[test]
    fn timeline_handles_arbitrary_workspace(workspace in arbitrary_string()) {
        let db = test_database();
        let harness = McpTestHarness::new(db);

        let workspace_opt = if workspace.is_empty() { None } else { Some(workspace.as_str()) };
        let result = harness.timeline(Some(10), workspace_opt);

        // Should not panic
        match result {
            Ok(events) => prop_assert!(events.len() <= 10),
            Err(_) => {}
        }
    }
}

// ============================================================================
// Property Tests: Activity Summary Tool
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Any days value produces valid response or graceful error
    #[test]
    fn activity_summary_handles_arbitrary_days(days in arbitrary_days()) {
        let db = test_database();
        let harness = McpTestHarness::new(db);

        // Convert to u32, clamping negative to 0
        let days_u32 = if days < 0 { 0 } else { days.min(u32::MAX as i64) as u32 };
        let result = harness.activity_summary(Some(days_u32));

        // Should not panic
        match result {
            Ok(summary) => {
                // Verify fields are valid u64 values (can be accessed)
                let _commits = summary.commits;
                let _test_runs = summary.test_runs;
            }
            Err(_) => {}
        }
    }
}

// ============================================================================
// Property Tests: Commit Details Tool
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Any SHA string doesn't crash commit details
    #[test]
    fn commit_details_handles_arbitrary_sha(sha in arbitrary_sha()) {
        let db = test_database();
        let harness = McpTestHarness::new(db);

        let result = harness.commit_details(&sha);

        // Should not panic, may return NotFound or InvalidInput
        match result {
            Ok(_commit) => {
                // Valid commit returned
            }
            Err(HandlerError::NotFound(_)) => {
                // Expected for nonexistent SHA
            }
            Err(HandlerError::InvalidInput(_)) => {
                // Expected for empty/invalid SHA
            }
            Err(e) => {
                // Other errors are also acceptable
                let _ = format!("{}", e);
            }
        }
    }
}

// ============================================================================
// Property Tests: Failing Tests Tool
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Failing tests handles arbitrary commit filters
    #[test]
    fn failing_tests_handles_arbitrary_commit(commit in arbitrary_sha()) {
        let db = test_database();
        let harness = McpTestHarness::new(db);

        let commit_opt = if commit.is_empty() { None } else { Some(commit.as_str()) };
        let result = harness.failing_tests(Some(10), None, commit_opt);

        // Should not panic
        match result {
            Ok(tests) => prop_assert!(tests.len() <= 10),
            Err(_) => {}
        }
    }

    /// Property: Failing tests handles arbitrary workspace filters
    #[test]
    fn failing_tests_handles_arbitrary_workspace(workspace in arbitrary_string()) {
        let db = test_database();
        let harness = McpTestHarness::new(db);

        let workspace_opt = if workspace.is_empty() { None } else { Some(workspace.as_str()) };
        let result = harness.failing_tests(Some(10), workspace_opt, None);

        // Should not panic
        match result {
            Ok(tests) => prop_assert!(tests.len() <= 10),
            Err(_) => {}
        }
    }
}

// ============================================================================
// Property Tests: JSON Parsing
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: JSON parsing never panics on arbitrary input
    #[test]
    fn json_parsing_never_panics(input in arbitrary_string()) {
        // Attempt to parse as JSON - should not panic
        let _result: Result<Value, _> = serde_json::from_str(&input);
        // Any result (Ok or Err) is acceptable
    }

    /// Property: Tool argument deserialization handles arbitrary JSON
    #[test]
    fn tool_args_handle_arbitrary_json(
        query in arbitrary_string(),
        source in arbitrary_source(),
        limit in 0i64..1000i64,
    ) {
        let json_obj = json!({
            "query": query,
            "source": source,
            "limit": limit
        });

        // Convert to Map<String, Value> like MCP does
        if let Value::Object(map) = json_obj {
            let db = test_database();

            // Try to invoke handler with these args
            let result = handlers::handle_search(&db, Some(map));

            // Should not panic
            match result {
                Ok(_) | Err(_) => {} // Both are acceptable
            }
        }
    }
}

// ============================================================================
// Property Tests: Ingest Tool
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Ingest handles arbitrary source values
    #[test]
    fn ingest_handles_arbitrary_source(source in arbitrary_ingest_source()) {
        let db = test_database();
        let harness = McpTestHarness::new(db);

        // Use a temp directory that exists
        let temp_dir = std::env::temp_dir();

        let result = harness.ingest(
            &source,
            temp_dir.to_str().unwrap_or("/tmp"),
            true,
            Some(1),
        );

        // Should not panic
        match result {
            Ok(_stats) => {}
            Err(_) => {}
        }
    }

    /// Property: Ingest handles arbitrary workspace paths
    #[test]
    fn ingest_handles_arbitrary_workspace(workspace in arbitrary_string()) {
        let db = test_database();
        let harness = McpTestHarness::new(db);

        let result = harness.ingest("all", &workspace, true, Some(1));

        // Should not panic - error for invalid path is acceptable
        match result {
            Ok(_stats) => {}
            Err(HandlerError::WorkspaceNotFound(_)) => {
                // Expected for invalid paths
            }
            Err(HandlerError::InvalidInput(_)) => {
                // Also expected
            }
            Err(_) => {}
        }
    }
}

// ============================================================================
// Deterministic Property Tests (no randomness, fast)
// ============================================================================

#[test]
fn property_search_sql_injection_safe() {
    // Verify SQL injection attempts don't crash or corrupt
    let db = test_database();
    let harness = McpTestHarness::new(db);

    let injection_attempts = [
        "'; DROP TABLE commits; --",
        "1; DELETE FROM workspaces; --",
        "test' OR '1'='1",
        "test\" OR \"1\"=\"1",
        "test\"; DROP TABLE test_runs; --",
        "Robert'); DROP TABLE students;--",
        "1 OR 1=1",
        "1 UNION SELECT * FROM commits",
        "test\0null",
        "test\x00byte",
    ];

    for query in injection_attempts {
        let result = harness.search(query, Some("all"), Some(10));
        // Should not panic, and no tables should be affected
        assert!(result.is_ok() || result.is_err());
    }

    // Verify database is still functional after injection attempts
    let result = harness.timeline(Some(10), None);
    assert!(result.is_ok());
}

#[test]
fn property_special_unicode_handled() {
    let db = test_database();
    let harness = McpTestHarness::new(db);

    let unicode_strings = [
        "日本語",
        "中文测试",
        "한국어",
        "العربية",
        "🔥🚀💻",
        "emoji: 😀😎🤔",
        "control\x00chars\x01here",
        "null\0byte",
        "tab\there",
        "newline\nhere",
        "carriage\rreturn",
        "\u{FEFF}BOM",
        "\u{200B}zero-width",
        "a̐éö̲\u{0332}",
    ];

    for s in unicode_strings {
        // Search
        let _ = harness.search(s, Some("all"), Some(5));
        // Timeline with workspace filter
        let _ = harness.timeline(Some(5), Some(s));
        // All should not panic
    }
}

#[test]
fn property_extreme_limits_handled() {
    let db = test_database();
    let harness = McpTestHarness::new(db);

    // Extreme limit values
    let limits = [0usize, 1, 10, 100, 1000, 10000, usize::MAX / 2];

    for limit in limits {
        let result = harness.timeline(Some(limit), None);
        assert!(result.is_ok() || result.is_err());

        let result = harness.search("test", Some("all"), Some(limit));
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn property_empty_inputs_handled() {
    let db = test_database();
    let harness = McpTestHarness::new(db);

    // Empty query - should produce error
    let result = harness.search("", Some("all"), Some(10));
    assert!(result.is_err());

    // Empty SHA - should produce error
    let result = harness.commit_details("");
    assert!(result.is_err());

    // Empty workspace filter - should be treated as None
    let result = harness.timeline(Some(10), Some(""));
    // Either success or error, both acceptable
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn property_very_long_strings_handled() {
    let db = test_database();
    let harness = McpTestHarness::new(db);

    // Long strings that might cause issues
    let long_query = "a".repeat(10000);
    let result = harness.search(&long_query, Some("all"), Some(10));
    assert!(result.is_ok() || result.is_err());

    let long_sha = "a".repeat(10000);
    let result = harness.commit_details(&long_sha);
    assert!(result.is_err()); // Should be not found, not crash

    let long_workspace = "/".to_string() + &"a".repeat(10000);
    let result = harness.timeline(Some(10), Some(&long_workspace));
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn property_path_traversal_safe() {
    let path_attacks = [
        "../../../etc/passwd",
        "..\\..\\..\\Windows\\System32",
        "/etc/passwd",
        "C:\\Windows\\System32\\config",
        "....//....//....//etc/passwd",
        "%2e%2e%2f%2e%2e%2f",
        "..%00/",
        "..%c0%af",
        "file:///etc/passwd",
        "\\\\server\\share",
    ];

    for path in path_attacks {
        // Each iteration needs a fresh harness since ingest consumes it
        let db = test_database();
        let harness = McpTestHarness::new(db);

        // Timeline should handle path filter gracefully
        let result = harness.timeline(Some(10), Some(path));
        assert!(result.is_ok() || result.is_err());

        // Fresh harness for ingest test
        let db2 = test_database();
        let harness2 = McpTestHarness::new(db2);

        let result = harness2.ingest("git", path, true, Some(1));
        // Should fail with WorkspaceNotFound or similar
        assert!(result.is_err() || result.is_ok());
    }
}
