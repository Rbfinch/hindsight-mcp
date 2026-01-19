// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! MCP test harness for hindsight-mcp integration tests
//!
//! This module provides utilities for testing MCP tool handlers directly,
//! without going through the full MCP protocol stack.

use std::path::PathBuf;

use serde_json::{Map, Value, json};

use hindsight_mcp::db::Database;
use hindsight_mcp::handlers::{self, HandlerError};
use hindsight_mcp::queries::{
    ActivitySummary, CommitWithTests, FailingTest, SearchResult, TimelineEvent,
};

// ============================================================================
// MCP Test Harness
// ============================================================================

/// Test harness for invoking MCP tool handlers
///
/// This provides a simple interface for testing tool handlers with
/// typed inputs and outputs, avoiding the JSON serialization layer.
pub struct McpTestHarness {
    db: Database,
    workspace: Option<PathBuf>,
}

impl McpTestHarness {
    /// Create a new test harness with the given database
    pub fn new(db: Database) -> Self {
        Self {
            db,
            workspace: None,
        }
    }

    /// Set the default workspace for tool invocations
    #[allow(dead_code)]
    pub fn with_workspace(mut self, workspace: PathBuf) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// Get a reference to the database
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// Get the default workspace
    #[allow(dead_code)]
    pub fn workspace(&self) -> Option<&PathBuf> {
        self.workspace.as_ref()
    }

    // ========================================================================
    // Typed Tool Invocations
    // ========================================================================

    /// Invoke the hindsight_timeline tool
    pub fn timeline(
        &self,
        limit: Option<usize>,
        workspace: Option<&str>,
    ) -> Result<Vec<TimelineEvent>, HandlerError> {
        let args = build_args(json!({
            "limit": limit.unwrap_or(50),
            "workspace": workspace
        }));
        handlers::handle_timeline(&self.db, Some(args), self.workspace.as_ref())
    }

    /// Invoke the hindsight_search tool
    pub fn search(
        &self,
        query: &str,
        source: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>, HandlerError> {
        let args = build_args(json!({
            "query": query,
            "source": source.unwrap_or("all"),
            "limit": limit.unwrap_or(20)
        }));
        handlers::handle_search(&self.db, Some(args))
    }

    /// Invoke the hindsight_failing_tests tool
    #[allow(dead_code)]
    pub fn failing_tests(
        &self,
        limit: Option<usize>,
        workspace: Option<&str>,
        commit: Option<&str>,
    ) -> Result<Vec<FailingTest>, HandlerError> {
        let args = build_args(json!({
            "limit": limit.unwrap_or(50),
            "workspace": workspace,
            "commit": commit
        }));
        handlers::handle_failing_tests(&self.db, Some(args), self.workspace.as_ref())
    }

    /// Invoke the hindsight_activity_summary tool
    pub fn activity_summary(&self, days: Option<u32>) -> Result<ActivitySummary, HandlerError> {
        let args = build_args(json!({
            "days": days.unwrap_or(7)
        }));
        handlers::handle_activity_summary(&self.db, Some(args))
    }

    /// Invoke the hindsight_commit_details tool
    pub fn commit_details(&self, sha: &str) -> Result<CommitWithTests, HandlerError> {
        let args = build_args(json!({
            "sha": sha
        }));
        handlers::handle_commit_details(&self.db, Some(args))
    }

    /// Invoke the hindsight_ingest tool (consumes harness since ingest needs DB ownership)
    #[allow(dead_code)]
    pub fn ingest(
        self,
        source: &str,
        workspace: &str,
        incremental: bool,
        limit: Option<usize>,
    ) -> Result<handlers::IngestResponse, HandlerError> {
        let args = build_args(json!({
            "source": source,
            "workspace": workspace,
            "incremental": incremental,
            "limit": limit
        }));
        handlers::handle_ingest(self.db, Some(args))
    }

    // ========================================================================
    // Raw JSON Invocations (for edge case testing)
    // ========================================================================

    /// Invoke a tool with raw JSON arguments
    ///
    /// This is useful for testing malformed or edge-case inputs.
    pub fn invoke_raw(
        &self,
        tool_name: &str,
        args: Option<Map<String, Value>>,
    ) -> Result<Value, HandlerError> {
        match tool_name {
            "hindsight_timeline" => {
                let result = handlers::handle_timeline(&self.db, args, self.workspace.as_ref())?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "hindsight_search" => {
                let result = handlers::handle_search(&self.db, args)?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "hindsight_failing_tests" => {
                let result =
                    handlers::handle_failing_tests(&self.db, args, self.workspace.as_ref())?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "hindsight_activity_summary" => {
                let result = handlers::handle_activity_summary(&self.db, args)?;
                Ok(serde_json::to_value(result).unwrap())
            }
            "hindsight_commit_details" => {
                let result = handlers::handle_commit_details(&self.db, args)?;
                Ok(serde_json::to_value(result).unwrap())
            }
            _ => Err(HandlerError::InvalidInput(format!(
                "Unknown tool: {}",
                tool_name
            ))),
        }
    }

    /// Invoke a tool with a JSON value (convenience wrapper)
    #[allow(dead_code)]
    pub fn invoke_with_json(&self, tool_name: &str, args: Value) -> Result<Value, HandlerError> {
        let map = match args {
            Value::Object(m) => Some(m),
            Value::Null => None,
            _ => {
                return Err(HandlerError::InvalidInput(
                    "Args must be an object".to_string(),
                ));
            }
        };
        self.invoke_raw(tool_name, map)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a JSON value into a Map<String, Value>
fn build_args(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map.into_iter().filter(|(_, v)| !v.is_null()).collect(),
        _ => Map::new(),
    }
}

// ============================================================================
// Response Assertions
// ============================================================================

/// Assert that a timeline result contains an event with the given content
#[allow(dead_code)]
pub fn assert_timeline_contains(events: &[TimelineEvent], content_substring: &str) {
    let found = events.iter().any(|e| e.summary.contains(content_substring));
    assert!(
        found,
        "Expected timeline to contain event with summary '{}', but it was not found.\nEvents: {:?}",
        content_substring, events
    );
}

/// Assert that search results contain a result with the given content
#[allow(dead_code)]
pub fn assert_search_contains(results: &[SearchResult], content_substring: &str) {
    let found = results
        .iter()
        .any(|r| r.snippet.contains(content_substring));
    assert!(
        found,
        "Expected search results to contain '{}', but it was not found.\nResults: {:?}",
        content_substring, results
    );
}

/// Assert that failing tests include a test with the given name
#[allow(dead_code)]
pub fn assert_failing_test_exists(tests: &[FailingTest], test_name_substring: &str) {
    let found = tests
        .iter()
        .any(|t| t.test_name.contains(test_name_substring));
    assert!(
        found,
        "Expected failing tests to include '{}', but it was not found.\nTests: {:?}",
        test_name_substring, tests
    );
}

/// Assert that an activity summary has expected counts
#[allow(dead_code)]
pub fn assert_activity_counts(summary: &ActivitySummary, min_commits: u64, min_test_runs: u64) {
    assert!(
        summary.commits >= min_commits,
        "Expected at least {} commits, got {}",
        min_commits,
        summary.commits
    );
    assert!(
        summary.test_runs >= min_test_runs,
        "Expected at least {} test runs, got {}",
        min_test_runs,
        summary.test_runs
    );
}

// ============================================================================
// Error Assertions
// ============================================================================

/// Assert that a handler error is an InvalidInput error
#[allow(dead_code)]
pub fn assert_invalid_input_error(result: Result<impl std::fmt::Debug, HandlerError>) {
    match result {
        Err(HandlerError::InvalidInput(_)) => {}
        Err(other) => panic!("Expected InvalidInput error, got: {:?}", other),
        Ok(val) => panic!("Expected error, got success: {:?}", val),
    }
}

/// Assert that a handler error is a NotFound error
#[allow(dead_code)]
pub fn assert_not_found_error(result: Result<impl std::fmt::Debug, HandlerError>) {
    match result {
        Err(HandlerError::NotFound(_)) => {}
        Err(other) => panic!("Expected NotFound error, got: {:?}", other),
        Ok(val) => panic!("Expected error, got success: {:?}", val),
    }
}

/// Assert that a handler error contains a specific message
#[allow(dead_code)]
pub fn assert_error_contains(
    result: Result<impl std::fmt::Debug, HandlerError>,
    expected_substring: &str,
) {
    match result {
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains(expected_substring),
                "Expected error to contain '{}', got: {}",
                expected_substring,
                msg
            );
        }
        Ok(val) => panic!(
            "Expected error containing '{}', got success: {:?}",
            expected_substring, val
        ),
    }
}

// ============================================================================
// Unit Tests for Harness
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_harness() -> McpTestHarness {
        let db =
            hindsight_mcp::db::Database::in_memory().expect("Failed to create in-memory database");
        db.initialize().expect("Failed to initialize database");
        McpTestHarness::new(db)
    }

    #[test]
    fn test_harness_creation() {
        let harness = test_harness();
        assert!(harness.database().is_initialized());
    }

    #[test]
    fn test_harness_timeline_empty_db() {
        let harness = test_harness();
        let result = harness.timeline(None, None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_harness_search_empty_query() {
        let harness = test_harness();
        let result = harness.search("", None, None);
        // Empty query should produce an error
        assert!(result.is_err());
    }

    #[test]
    fn test_harness_activity_summary_empty_db() {
        let harness = test_harness();
        let result = harness.activity_summary(None);
        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.commits, 0);
    }

    #[test]
    fn test_harness_commit_details_not_found() {
        let harness = test_harness();
        let result = harness.commit_details("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_harness_invoke_raw_unknown_tool() {
        let harness = test_harness();
        let result = harness.invoke_raw("unknown_tool", None);
        assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
    }

    #[test]
    fn test_harness_build_args_filters_nulls() {
        let args = build_args(json!({
            "limit": 10,
            "workspace": null,
            "query": "test"
        }));

        assert!(args.contains_key("limit"));
        assert!(args.contains_key("query"));
        assert!(!args.contains_key("workspace"));
    }
}
