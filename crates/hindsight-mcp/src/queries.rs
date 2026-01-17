//! Query helper functions for hindsight-mcp database
//!
//! This module provides high-level query functions for searching and
//! retrieving development history data from the SQLite database.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Query errors
#[derive(Debug, Error)]
pub enum QueryError {
    /// SQLite error during query
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

/// A timeline event representing activity in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Event type: 'commit', 'test_run', or 'copilot_message'
    pub event_type: String,
    /// Unique identifier for the event (UUID as string)
    pub event_id: String,
    /// Workspace ID
    pub workspace_id: String,
    /// ISO 8601 timestamp of the event
    pub event_timestamp: String,
    /// Brief summary of the event
    pub summary: String,
    /// JSON details
    pub details_json: Option<String>,
}

/// A search result from full-text search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Type of result: 'commit' or 'copilot_message'
    pub result_type: String,
    /// Unique identifier (UUID as string)
    pub id: String,
    /// Matching content snippet
    pub snippet: String,
    /// Relevance rank (lower is better)
    pub rank: f64,
    /// ISO 8601 timestamp
    pub timestamp: String,
}

/// A failing test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailingTest {
    /// Test name (UUID)
    pub test_name: String,
    /// Suite name
    pub suite_name: String,
    /// Full test name
    pub full_name: String,
    /// Duration in milliseconds
    pub duration_ms: Option<i64>,
    /// Output JSON if available
    pub output_json: Option<String>,
    /// Test run ID
    pub run_id: String,
    /// Commit SHA
    pub commit_sha: Option<String>,
    /// ISO 8601 timestamp
    pub started_at: String,
}

/// Get workspace ID from a workspace path
///
/// The workspace filter can be either a workspace ID (UUID) or a filesystem path.
/// This function looks up the path in the workspaces table to find the corresponding ID.
/// If the filter doesn't match a path, it's assumed to be a workspace ID and returned as-is.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `filter` - Workspace ID or path to resolve
///
/// # Returns
///
/// The workspace ID, or None if the path doesn't exist and wasn't a valid ID.
fn resolve_workspace_filter(conn: &Connection, filter: &str) -> Result<Option<String>, QueryError> {
    // First, try to look up by path
    let result: Result<String, _> = conn.query_row(
        "SELECT id FROM workspaces WHERE path = ?",
        [filter],
        |row| row.get(0),
    );

    match result {
        Ok(id) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            // Not a path - check if it's a valid workspace ID
            let exists: Result<i64, _> =
                conn.query_row("SELECT 1 FROM workspaces WHERE id = ?", [filter], |row| {
                    row.get(0)
                });
            match exists {
                Ok(_) => Ok(Some(filter.to_string())),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(QueryError::Sqlite(e)),
            }
        }
        Err(e) => Err(QueryError::Sqlite(e)),
    }
}

/// Query the timeline view for recent activity
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `limit` - Maximum number of events to return
/// * `workspace_filter` - Optional workspace path or ID to filter by
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn get_timeline(
    conn: &Connection,
    limit: usize,
    workspace_filter: Option<&str>,
) -> Result<Vec<TimelineEvent>, QueryError> {
    let mut events = Vec::new();

    // Resolve workspace filter (path or ID) to workspace ID
    let resolved_workspace_id = match workspace_filter {
        Some(filter) => resolve_workspace_filter(conn, filter)?,
        None => None,
    };

    if let Some(workspace_id) = resolved_workspace_id {
        let mut stmt = conn.prepare(
            r#"
            SELECT event_type, event_id, workspace_id, event_timestamp, summary, details_json
            FROM timeline
            WHERE workspace_id = ?
            ORDER BY event_timestamp DESC
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map(params![workspace_id, limit as i64], |row| {
            Ok(TimelineEvent {
                event_type: row.get(0)?,
                event_id: row.get(1)?,
                workspace_id: row.get(2)?,
                event_timestamp: row.get(3)?,
                summary: row.get(4)?,
                details_json: row.get(5)?,
            })
        })?;

        for row in rows {
            events.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            r#"
            SELECT event_type, event_id, workspace_id, event_timestamp, summary, details_json
            FROM timeline
            ORDER BY event_timestamp DESC
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map([limit as i64], |row| {
            Ok(TimelineEvent {
                event_type: row.get(0)?,
                event_id: row.get(1)?,
                workspace_id: row.get(2)?,
                event_timestamp: row.get(3)?,
                summary: row.get(4)?,
                details_json: row.get(5)?,
            })
        })?;

        for row in rows {
            events.push(row?);
        }
    }

    Ok(events)
}

/// Search commits using FTS5 full-text search
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `query` - Search query (FTS5 syntax)
/// * `limit` - Maximum number of results
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn search_commits(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, QueryError> {
    if query.is_empty() {
        return Err(QueryError::InvalidParameter("Query cannot be empty".into()));
    }

    let mut results = Vec::new();

    // FTS5 uses rowid which matches the internal SQLite rowid of commits table
    let mut stmt = conn.prepare(
        r#"
        SELECT
            c.id,
            snippet(commits_fts, 0, '<mark>', '</mark>', '...', 32) AS snippet,
            commits_fts.rank,
            c.timestamp
        FROM commits_fts
        JOIN commits c ON c.rowid = commits_fts.rowid
        WHERE commits_fts MATCH ?
        ORDER BY commits_fts.rank
        LIMIT ?
        "#,
    )?;

    let rows = stmt.query_map(params![query, limit as i64], |row| {
        Ok(SearchResult {
            result_type: "commit".to_string(),
            id: row.get(0)?,
            snippet: row.get(1)?,
            rank: row.get(2)?,
            timestamp: row.get(3)?,
        })
    })?;

    for row in rows {
        results.push(row?);
    }

    Ok(results)
}

/// Search Copilot messages using FTS5 full-text search
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `query` - Search query (FTS5 syntax)
/// * `limit` - Maximum number of results
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn search_messages(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, QueryError> {
    if query.is_empty() {
        return Err(QueryError::InvalidParameter("Query cannot be empty".into()));
    }

    let mut results = Vec::new();

    // FTS5 uses rowid which matches the internal SQLite rowid of copilot_messages table
    let mut stmt = conn.prepare(
        r#"
        SELECT
            m.id,
            snippet(copilot_messages_fts, 0, '<mark>', '</mark>', '...', 32) AS snippet,
            copilot_messages_fts.rank,
            m.timestamp
        FROM copilot_messages_fts
        JOIN copilot_messages m ON m.rowid = copilot_messages_fts.rowid
        WHERE copilot_messages_fts MATCH ?
        ORDER BY copilot_messages_fts.rank
        LIMIT ?
        "#,
    )?;

    let rows = stmt.query_map(params![query, limit as i64], |row| {
        Ok(SearchResult {
            result_type: "copilot_message".to_string(),
            id: row.get(0)?,
            snippet: row.get(1)?,
            rank: row.get(2)?,
            timestamp: row.get(3)?,
        })
    })?;

    for row in rows {
        results.push(row?);
    }

    Ok(results)
}

/// Combined search across commits and messages
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `query` - Search query (FTS5 syntax)
/// * `limit` - Maximum number of results per type
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn search_all(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, QueryError> {
    let mut results = Vec::new();

    // Search commits
    results.extend(search_commits(conn, query, limit)?);

    // Search messages
    results.extend(search_messages(conn, query, limit)?);

    // Sort by rank (lower is better)
    results.sort_by(|a, b| {
        a.rank
            .partial_cmp(&b.rank)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Limit total results
    results.truncate(limit);

    Ok(results)
}

/// Get failing tests from the failing_tests view
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `limit` - Maximum number of results
/// * `workspace_filter` - Optional workspace path or ID to filter by (via test_runs)
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn get_failing_tests(
    conn: &Connection,
    limit: usize,
    workspace_filter: Option<&str>,
) -> Result<Vec<FailingTest>, QueryError> {
    let mut tests = Vec::new();

    // Resolve workspace filter (path or ID) to workspace ID
    let resolved_workspace_id = match workspace_filter {
        Some(filter) => resolve_workspace_filter(conn, filter)?,
        None => None,
    };

    // The failing_tests view columns are:
    // test_name (from tr.id), suite_name, full_name (from tr.test_name),
    // duration_ms, output_json, run_id, commit_sha, started_at
    if let Some(workspace_id) = resolved_workspace_id {
        let mut stmt = conn.prepare(
            r#"
            SELECT ft.test_name, ft.suite_name, ft.full_name, ft.duration_ms,
                   ft.output_json, ft.run_id, ft.commit_sha, ft.started_at
            FROM failing_tests ft
            JOIN test_runs tr ON tr.id = ft.run_id
            WHERE tr.workspace_id = ?
            ORDER BY ft.started_at DESC
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map(params![workspace_id, limit as i64], |row| {
            Ok(FailingTest {
                test_name: row.get(0)?,
                suite_name: row.get(1)?,
                full_name: row.get(2)?,
                duration_ms: row.get(3)?,
                output_json: row.get(4)?,
                run_id: row.get(5)?,
                commit_sha: row.get(6)?,
                started_at: row.get(7)?,
            })
        })?;

        for row in rows {
            tests.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            r#"
            SELECT test_name, suite_name, full_name, duration_ms,
                   output_json, run_id, commit_sha, started_at
            FROM failing_tests
            ORDER BY started_at DESC
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map([limit as i64], |row| {
            Ok(FailingTest {
                test_name: row.get(0)?,
                suite_name: row.get(1)?,
                full_name: row.get(2)?,
                duration_ms: row.get(3)?,
                output_json: row.get(4)?,
                run_id: row.get(5)?,
                commit_sha: row.get(6)?,
                started_at: row.get(7)?,
            })
        })?;

        for row in rows {
            tests.push(row?);
        }
    }

    Ok(tests)
}

/// Get recent activity summary
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `days` - Number of days to look back
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn get_activity_summary(conn: &Connection, days: u32) -> Result<ActivitySummary, QueryError> {
    let since = format!("-{} days", days);

    let commit_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM commits WHERE timestamp >= datetime('now', ?)",
        [&since],
        |row| row.get(0),
    )?;

    // test_runs uses started_at, not timestamp
    let test_run_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM test_runs WHERE started_at >= datetime('now', ?)",
        [&since],
        |row| row.get(0),
    )?;

    // copilot_sessions uses created_at, not start_time
    let session_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM copilot_sessions WHERE created_at >= datetime('now', ?)",
        [&since],
        |row| row.get(0),
    )?;

    // test_results uses outcome, not status
    let failing_test_count: i64 = conn.query_row(
        r#"
        SELECT COUNT(*)
        FROM test_results tr
        JOIN test_runs r ON r.id = tr.run_id
        WHERE r.started_at >= datetime('now', ?)
        AND tr.outcome IN ('failed', 'timedout')
        "#,
        [&since],
        |row| row.get(0),
    )?;

    Ok(ActivitySummary {
        days,
        commits: commit_count as u64,
        test_runs: test_run_count as u64,
        copilot_sessions: session_count as u64,
        failing_tests: failing_test_count as u64,
    })
}

/// Summary of recent activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummary {
    /// Number of days covered
    pub days: u32,
    /// Number of commits
    pub commits: u64,
    /// Number of test runs
    pub test_runs: u64,
    /// Number of Copilot sessions
    pub copilot_sessions: u64,
    /// Number of failing tests
    pub failing_tests: u64,
}

/// Get commits with their associated test results
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `commit_sha` - Git commit SHA (or prefix)
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn get_commit_with_tests(
    conn: &Connection,
    commit_sha: &str,
) -> Result<Option<CommitWithTests>, QueryError> {
    // Find the commit - schema uses diff_json for file changes
    let commit: Option<(String, String, String, String, String, Option<String>)> = conn
        .query_row(
            r#"
            SELECT id, sha, message, author, timestamp, diff_json
            FROM commits
            WHERE sha LIKE ? || '%'
            LIMIT 1
            "#,
            [commit_sha],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .optional()?;

    let Some((id, sha, message, author, timestamp, diff_json)) = commit else {
        return Ok(None);
    };

    // Parse diff JSON to extract file paths
    let files: Vec<String> = match diff_json {
        Some(ref json) => {
            // Try to parse as object with "files" key, or as array of file paths
            serde_json::from_str::<serde_json::Value>(json)
                .ok()
                .and_then(|v| {
                    // Try {"files": [...]} format
                    if let Some(files) = v.get("files") {
                        files.as_array().map(|arr| {
                            arr.iter()
                                .filter_map(|f| f.get("path").and_then(|p| p.as_str()))
                                .map(String::from)
                                .collect()
                        })
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        }
        None => Vec::new(),
    };

    // Get associated test runs - schema uses started_at, passed_count, failed_count, ignored_count
    let mut stmt = conn.prepare(
        r#"
        SELECT r.id, r.started_at, r.passed_count, r.failed_count, r.ignored_count
        FROM test_runs r
        WHERE r.commit_sha = ?
        ORDER BY r.started_at DESC
        "#,
    )?;

    let test_runs: Vec<TestRunSummary> = stmt
        .query_map([&sha], |row| {
            Ok(TestRunSummary {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                passed: row.get(2)?,
                failed: row.get(3)?,
                skipped: row.get(4)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();

    Ok(Some(CommitWithTests {
        id,
        sha,
        message,
        author,
        timestamp,
        files,
        test_runs,
    }))
}

/// A commit with associated test information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitWithTests {
    /// Database ID (UUID)
    pub id: String,
    /// Git commit SHA
    pub sha: String,
    /// Commit message
    pub message: String,
    /// Author name
    pub author: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Changed files
    pub files: Vec<String>,
    /// Associated test runs
    pub test_runs: Vec<TestRunSummary>,
}

/// Summary of a test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunSummary {
    /// Database ID (UUID)
    pub id: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Number of passed tests
    pub passed: i32,
    /// Number of failed tests
    pub failed: i32,
    /// Number of skipped tests
    pub skipped: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("create db");
        migrations::migrate(&conn).expect("migrate");
        conn
    }

    #[test]
    fn test_get_timeline_empty() {
        let conn = setup_db();
        let events = get_timeline(&conn, 10, None).expect("timeline");
        assert!(events.is_empty());
    }

    #[test]
    fn test_search_commits_empty_query() {
        let conn = setup_db();
        let result = search_commits(&conn, "", 10);
        assert!(matches!(result, Err(QueryError::InvalidParameter(_))));
    }

    #[test]
    fn test_search_messages_empty_query() {
        let conn = setup_db();
        let result = search_messages(&conn, "", 10);
        assert!(matches!(result, Err(QueryError::InvalidParameter(_))));
    }

    #[test]
    fn test_get_failing_tests_empty() {
        let conn = setup_db();
        let tests = get_failing_tests(&conn, 10, None).expect("failing tests");
        assert!(tests.is_empty());
    }

    #[test]
    fn test_get_activity_summary() {
        let conn = setup_db();
        let summary = get_activity_summary(&conn, 7).expect("activity summary");
        assert_eq!(summary.days, 7);
        assert_eq!(summary.commits, 0);
        assert_eq!(summary.test_runs, 0);
        assert_eq!(summary.copilot_sessions, 0);
        assert_eq!(summary.failing_tests, 0);
    }

    #[test]
    fn test_get_commit_with_tests_not_found() {
        let conn = setup_db();
        let result = get_commit_with_tests(&conn, "nonexistent").expect("query");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_with_data() {
        let conn = setup_db();

        // Insert a workspace with all required columns
        conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES ('ws-1', 'test', '/test', datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert workspace");

        // Insert a commit with all required columns
        conn.execute(
            r#"
            INSERT INTO commits (id, workspace_id, sha, message, author, timestamp, created_at)
            VALUES ('c-1', 'ws-1', 'abc123def456789012345678901234567890abcd', 'Fix important bug in parser', 'Test Author', datetime('now'), datetime('now'))
            "#,
            [],
        )
        .expect("insert commit");

        // Search for the commit
        let results = search_commits(&conn, "parser", 10).expect("search");
        assert_eq!(results.len(), 1);
        assert!(results[0].snippet.contains("parser"));
    }

    #[test]
    fn test_timeline_with_data() {
        let conn = setup_db();

        // Insert a workspace with all required columns
        conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES ('ws-1', 'test', '/test', datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert workspace");

        // Insert a commit with all required columns
        conn.execute(
            r#"
            INSERT INTO commits (id, workspace_id, sha, message, author, timestamp, created_at)
            VALUES ('c-1', 'ws-1', 'abc123def456789012345678901234567890abcd', 'Test commit', 'Author', datetime('now'), datetime('now'))
            "#,
            [],
        )
        .expect("insert commit");

        // Get timeline
        let events = get_timeline(&conn, 10, None).expect("timeline");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "commit");
    }

    #[test]
    fn test_failing_tests_with_data() {
        let conn = setup_db();

        // Insert a workspace with all required columns
        conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES ('ws-1', 'test', '/test', datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert workspace");

        // Insert a test run with correct column names
        conn.execute(
            r#"
            INSERT INTO test_runs (id, workspace_id, started_at, passed_count, failed_count, ignored_count)
            VALUES ('tr-1', 'ws-1', datetime('now'), 5, 2, 0)
            "#,
            [],
        )
        .expect("insert test run");

        // Insert a failing test with correct column names
        conn.execute(
            r#"
            INSERT INTO test_results (id, run_id, suite_name, test_name, outcome, duration_ms, created_at)
            VALUES ('r-1', 'tr-1', 'hindsight-mcp', 'test_something', 'failed', 1500, datetime('now'))
            "#,
            [],
        )
        .expect("insert test result");

        // Get failing tests
        let tests = get_failing_tests(&conn, 10, None).expect("failing tests");
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].suite_name, "hindsight-mcp");
        assert_eq!(tests[0].full_name, "test_something");
    }

    #[test]
    fn test_resolve_workspace_filter_by_path() {
        let conn = setup_db();

        // Insert a workspace
        conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES ('ws-123', 'test', '/test/workspace', datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert workspace");

        // Resolve by path should return the workspace ID
        let result = resolve_workspace_filter(&conn, "/test/workspace").expect("resolve");
        assert_eq!(result, Some("ws-123".to_string()));
    }

    #[test]
    fn test_resolve_workspace_filter_by_id() {
        let conn = setup_db();

        // Insert a workspace
        conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES ('ws-456', 'test', '/another/path', datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert workspace");

        // Resolve by ID should return the same ID
        let result = resolve_workspace_filter(&conn, "ws-456").expect("resolve");
        assert_eq!(result, Some("ws-456".to_string()));
    }

    #[test]
    fn test_resolve_workspace_filter_not_found() {
        let conn = setup_db();

        // Resolve non-existent path or ID should return None
        let result = resolve_workspace_filter(&conn, "/nonexistent/path").expect("resolve");
        assert_eq!(result, None);
    }

    #[test]
    fn test_timeline_with_workspace_path_filter() {
        let conn = setup_db();

        // Insert a workspace with a specific path
        conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES ('ws-1', 'test', '/my/workspace', datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert workspace");

        // Insert a commit for this workspace
        conn.execute(
            r#"
            INSERT INTO commits (id, workspace_id, sha, message, author, timestamp, created_at)
            VALUES ('c-1', 'ws-1', 'abc123def456789012345678901234567890abcd', 'Test commit', 'Author', datetime('now'), datetime('now'))
            "#,
            [],
        )
        .expect("insert commit");

        // Get timeline filtered by path (not ID) - this tests the bug fix
        let events = get_timeline(&conn, 10, Some("/my/workspace")).expect("timeline");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "commit");
        assert_eq!(events[0].workspace_id, "ws-1");
    }

    #[test]
    fn test_failing_tests_with_workspace_path_filter() {
        let conn = setup_db();

        // Insert a workspace with a specific path
        conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES ('ws-1', 'test', '/my/workspace', datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert workspace");

        // Insert a test run
        conn.execute(
            r#"
            INSERT INTO test_runs (id, workspace_id, started_at, passed_count, failed_count, ignored_count)
            VALUES ('tr-1', 'ws-1', datetime('now'), 5, 1, 0)
            "#,
            [],
        )
        .expect("insert test run");

        // Insert a failing test
        conn.execute(
            r#"
            INSERT INTO test_results (id, run_id, suite_name, test_name, outcome, duration_ms, created_at)
            VALUES ('r-1', 'tr-1', 'my-crate', 'test_fails', 'failed', 100, datetime('now'))
            "#,
            [],
        )
        .expect("insert test result");

        // Get failing tests filtered by path (not ID) - this tests the bug fix
        let tests = get_failing_tests(&conn, 10, Some("/my/workspace")).expect("failing tests");
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].suite_name, "my-crate");
    }
}
