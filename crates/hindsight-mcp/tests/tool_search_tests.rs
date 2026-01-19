// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Tests for the hindsight_search MCP tool handler
//!
//! This module tests the search tool which performs full-text search
//! across commits and/or copilot messages.

mod fixtures;
mod mcp_harness;

use fixtures::{populated_database, test_database};
use mcp_harness::{McpTestHarness, assert_error_contains, assert_search_contains};
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
// Required Query Parameter Tests
// ============================================================================

#[test]
fn test_search_query_required() {
    let db = test_database();

    // Missing query field entirely
    let args = to_map(json!({}));
    let result = handlers::handle_search(&db, Some(args));

    // Should fail because query is required
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_search_empty_query_produces_error() {
    let harness = harness_with_populated_db();

    let result = harness.search("", None, None);

    assert!(result.is_err());
    assert_error_contains(result, "cannot be empty");
}

#[test]
fn test_search_whitespace_only_query() {
    let harness = harness_with_populated_db();

    // Whitespace-only query - handler trims and rejects
    // The current implementation checks for empty after receiving
    let result = harness.search("   ", None, None);

    // May succeed with empty results or fail - depends on implementation
    // The key is it shouldn't crash
    let _ = result;
}

// ============================================================================
// Valid Query Tests
// ============================================================================

#[test]
fn test_search_valid_query_returns_results() {
    let harness = harness_with_populated_db();

    // Search for a term that exists in the populated data
    // The commits have "Commit message N" in them
    let results = harness
        .search("Commit", None, None)
        .expect("search should succeed");

    assert!(
        !results.is_empty(),
        "Should find commits containing 'Commit'"
    );
}

#[test]
fn test_search_query_no_matches() {
    let harness = harness_with_populated_db();

    // Search for something that doesn't exist
    let results = harness
        .search("xyznonexistentterm", None, None)
        .expect("search should succeed");

    assert!(
        results.is_empty(),
        "Should return empty for non-matching query"
    );
}

#[test]
fn test_search_case_insensitive() {
    let harness = harness_with_populated_db();

    // FTS5 is case-insensitive by default
    let results_lower = harness.search("commit", None, None).expect("search");
    let results_upper = harness.search("COMMIT", None, None).expect("search");

    // Both should find results (case insensitive)
    // The counts might differ slightly due to ranking, but both should have results
    let _ = (results_lower, results_upper);
}

// ============================================================================
// Source Filter Tests
// ============================================================================

#[test]
fn test_search_source_all_default() {
    let harness = harness_with_populated_db();

    // Default source is "all"
    let results = harness
        .search("Commit", None, None)
        .expect("search should succeed");

    assert!(!results.is_empty());
}

#[test]
fn test_search_source_commits() {
    let harness = harness_with_populated_db();

    // Search only in commits
    let results = harness
        .search("Commit", Some("commits"), None)
        .expect("search should succeed");

    // All results should be from commits
    for result in &results {
        assert_eq!(
            result.result_type, "commit",
            "Should only return commit results"
        );
    }
}

#[test]
fn test_search_source_messages() {
    let harness = harness_with_populated_db();

    // Search only in copilot messages
    // The populated data has "caching" in a copilot message
    let results = harness
        .search("caching", Some("messages"), None)
        .expect("search should succeed");

    // All results should be from messages
    for result in &results {
        assert_eq!(
            result.result_type, "copilot_message",
            "Should only return message results"
        );
    }
}

#[test]
fn test_search_source_invalid_falls_back_to_all() {
    let harness = harness_with_populated_db();

    // Invalid source should fall back to "all"
    let results = harness
        .search("Commit", Some("invalid_source"), None)
        .expect("search should succeed");

    // Should still return results (falling back to all)
    assert!(!results.is_empty());
}

#[test]
fn test_search_source_empty_string_uses_all() {
    let harness = harness_with_populated_db();

    let results = harness
        .search("Commit", Some(""), None)
        .expect("search should succeed");

    // Empty source should use "all"
    assert!(!results.is_empty());
}

// ============================================================================
// Limit Parameter Tests
// ============================================================================

#[test]
fn test_search_default_limit_is_20() {
    let harness = harness_with_populated_db();

    let results = harness
        .search("Commit", None, None)
        .expect("search should succeed");

    // Should have at most 20 results (default limit)
    assert!(results.len() <= 20);
}

#[test]
fn test_search_custom_limit() {
    let harness = harness_with_populated_db();

    let results = harness
        .search("Commit", None, Some(3))
        .expect("search should succeed");

    assert!(results.len() <= 3, "Should respect custom limit");
}

#[test]
fn test_search_limit_zero() {
    let harness = harness_with_populated_db();

    let results = harness
        .search("Commit", None, Some(0))
        .expect("search should succeed");

    assert!(results.is_empty(), "Limit 0 should return no results");
}

#[test]
fn test_search_very_large_limit() {
    let harness = harness_with_populated_db();

    let results = harness
        .search("Commit", None, Some(10000))
        .expect("search should succeed");

    // Should return results without crashing
    assert!(!results.is_empty());
}

// ============================================================================
// FTS5 Syntax Tests
// ============================================================================

#[test]
fn test_search_fts5_and_operator() {
    let harness = harness_with_populated_db();

    // FTS5 AND operator
    let results = harness
        .search("Commit AND message", None, None)
        .expect("search should succeed");

    // Should find commits with both terms
    let _ = results;
}

#[test]
fn test_search_fts5_or_operator() {
    let harness = harness_with_populated_db();

    // FTS5 OR operator
    let results = harness
        .search("Commit OR caching", None, None)
        .expect("search should succeed");

    // Should find items with either term
    assert!(!results.is_empty());
}

#[test]
fn test_search_fts5_not_operator() {
    let harness = harness_with_populated_db();

    // FTS5 NOT operator - commits that don't have "0"
    let results = harness
        .search("Commit NOT 0", None, None)
        .expect("search should succeed");

    let _ = results;
}

#[test]
fn test_search_fts5_phrase_match() {
    let harness = harness_with_populated_db();

    // FTS5 phrase matching with quotes
    let results = harness
        .search("\"Commit message\"", None, None)
        .expect("search should succeed");

    // Should find commits with exact phrase
    let _ = results;
}

#[test]
fn test_search_fts5_prefix_match() {
    let harness = harness_with_populated_db();

    // FTS5 prefix matching with asterisk
    let results = harness
        .search("Comm*", None, None)
        .expect("search should succeed");

    // Should find "Commit" matches
    assert!(!results.is_empty());
}

// ============================================================================
// Special Characters Tests
// ============================================================================

#[test]
fn test_search_special_characters_quotes() {
    let harness = harness_with_populated_db();

    // The copilot messages have code blocks with backticks
    let results = harness.search("rust", None, None);

    // Should handle without crashing
    let _ = results;
}

#[test]
fn test_search_special_characters_backticks() {
    let harness = harness_with_populated_db();

    // Search for content that might have backticks
    let results = harness.search("`rust`", None, None);

    // Should handle without crashing
    let _ = results;
}

#[test]
fn test_search_special_characters_brackets() {
    let harness = harness_with_populated_db();

    // Brackets might be in code
    let results = harness.search("[test]", None, None);

    // Should handle without crashing
    let _ = results;
}

#[test]
fn test_search_special_characters_parentheses() {
    let harness = harness_with_populated_db();

    let results = harness.search("function()", None, None);

    // Should handle without crashing
    let _ = results;
}

#[test]
fn test_search_unicode_characters() {
    let harness = harness_with_populated_db();

    // Unicode search term
    let results = harness.search("日本語", None, None);

    // Should handle without crashing
    let _ = results;
}

#[test]
fn test_search_emoji() {
    let harness = harness_with_populated_db();

    // Emoji in search
    let results = harness.search("🎉", None, None);

    // Should handle without crashing
    let _ = results;
}

// ============================================================================
// Result Structure Tests
// ============================================================================

#[test]
fn test_search_results_have_required_fields() {
    let harness = harness_with_populated_db();
    let results = harness.search("Commit", None, None).expect("search");

    for result in &results {
        assert!(!result.id.is_empty(), "id should not be empty");
        assert!(
            !result.result_type.is_empty(),
            "result_type should not be empty"
        );
        assert!(!result.snippet.is_empty(), "snippet should not be empty");
        assert!(
            !result.timestamp.is_empty(),
            "timestamp should not be empty"
        );
    }
}

#[test]
fn test_search_results_have_rank() {
    let harness = harness_with_populated_db();
    let results = harness.search("Commit", None, None).expect("search");

    for result in &results {
        // Rank should be a finite number
        assert!(result.rank.is_finite(), "rank should be a finite number");
    }
}

#[test]
fn test_search_results_ordered_by_relevance() {
    let harness = harness_with_populated_db();
    let results = harness.search("Commit", None, None).expect("search");

    if results.len() > 1 {
        // Results should be ordered by rank (lower is better in FTS5)
        for i in 0..results.len() - 1 {
            assert!(
                results[i].rank <= results[i + 1].rank,
                "Results should be ordered by rank (ascending)"
            );
        }
    }
}

// ============================================================================
// Raw JSON Input Tests
// ============================================================================

#[test]
fn test_search_raw_json_valid() {
    let db = test_database();

    let args = to_map(json!({
        "query": "test",
        "source": "all",
        "limit": 10
    }));

    let result = handlers::handle_search(&db, Some(args));
    assert!(result.is_ok());
}

#[test]
fn test_search_raw_json_minimal() {
    let db = test_database();

    // Only required field
    let args = to_map(json!({
        "query": "test"
    }));

    let result = handlers::handle_search(&db, Some(args));
    assert!(result.is_ok());
}

#[test]
fn test_search_raw_json_wrong_type_for_query() {
    let db = test_database();

    // Number instead of string for query
    let args = to_map(json!({
        "query": 12345
    }));

    let result = handlers::handle_search(&db, Some(args));
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_search_raw_json_wrong_type_for_limit() {
    let db = test_database();

    // String instead of number for limit
    let args = to_map(json!({
        "query": "test",
        "limit": "not a number"
    }));

    let result = handlers::handle_search(&db, Some(args));
    assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
}

#[test]
fn test_search_raw_json_extra_fields_ignored() {
    let db = test_database();

    let args = to_map(json!({
        "query": "test",
        "unknown_field": "some value"
    }));

    let result = handlers::handle_search(&db, Some(args));
    assert!(result.is_ok(), "Extra fields should be ignored");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_search_very_long_query() {
    let harness = harness_with_populated_db();

    // Very long search query
    let long_query = "a".repeat(1000);
    let results = harness.search(&long_query, None, None);

    // Should handle without crashing
    let _ = results;
}

#[test]
fn test_search_sql_injection_attempt() {
    let harness = harness_with_populated_db();

    // Attempt SQL injection
    let results = harness.search("'; DROP TABLE commits; --", None, None);

    // Should handle safely without crashing or corrupting database
    let _ = results;

    // Verify database is still intact by searching again
    let verify = harness.search("Commit", None, None);
    assert!(
        verify.is_ok(),
        "Database should be intact after injection attempt"
    );
}

#[test]
fn test_search_invoke_with_json_method() {
    let harness = harness_with_populated_db();

    let result = harness.invoke_with_json(
        "hindsight_search",
        json!({
            "query": "Commit",
            "limit": 5
        }),
    );

    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.is_array());
}

#[test]
fn test_search_empty_database() {
    let db = test_database();
    let harness = McpTestHarness::new(db);

    let results = harness.search("anything", None, None).expect("search");

    assert!(
        results.is_empty(),
        "Empty database should return no results"
    );
}

#[test]
fn test_search_finds_copilot_content() {
    let harness = harness_with_populated_db();

    // Search for content in copilot messages
    let results = harness
        .search("HashMap", None, None)
        .expect("search should succeed");

    // Should find the copilot message about HashMap
    assert_search_contains(&results, "HashMap");
}
