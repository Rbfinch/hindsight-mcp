//! Database module for hindsight-mcp
//!
//! This module provides SQLite database operations for storing and querying
//! development history data including git commits, test results, and Copilot sessions.

use rusqlite::{Connection, Result as SqliteResult};
use thiserror::Error;

/// Database errors
#[derive(Debug, Error)]
pub enum DbError {
    /// SQLite error
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Database not initialized
    #[error("Database not initialized")]
    NotInitialized,

    /// Record not found
    #[error("Record not found: {table}/{id}")]
    NotFound { table: String, id: String },
}

/// Database connection wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create a new in-memory database
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be created.
    pub fn in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        Ok(Self { conn })
    }

    /// Open a database file
    ///
    /// # Errors
    ///
    /// Returns an error if the database file cannot be opened.
    pub fn open(path: &std::path::Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    /// Initialize the database schema
    ///
    /// # Errors
    ///
    /// Returns an error if the schema cannot be created.
    pub fn initialize(&self) -> Result<(), DbError> {
        self.conn.execute_batch(SCHEMA)?;
        Ok(())
    }

    /// Check if the database is initialized
    pub fn is_initialized(&self) -> bool {
        self.conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='workspaces'",
                [],
                |_| Ok(()),
            )
            .is_ok()
    }

    /// Get the underlying connection (for advanced queries)
    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Execute a simple query and return the count
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn count(&self, table: &str) -> Result<i64, DbError> {
        let query = format!("SELECT COUNT(*) FROM {table}");
        let count: i64 = self.conn.query_row(&query, [], |row| row.get(0))?;
        Ok(count)
    }
}

/// Core database schema
const SCHEMA: &str = r#"
-- Workspaces table (root entity)
CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Git commits
CREATE TABLE IF NOT EXISTS commits (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    sha TEXT NOT NULL,
    author TEXT NOT NULL,
    author_email TEXT,
    message TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    parents_json TEXT,
    diff_json TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(workspace_id, sha)
);

-- Test runs (single nextest execution)
CREATE TABLE IF NOT EXISTS test_runs (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    commit_sha TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    passed_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    ignored_count INTEGER NOT NULL DEFAULT 0,
    metadata_json TEXT
);

-- Individual test results
CREATE TABLE IF NOT EXISTS test_results (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES test_runs(id),
    suite_name TEXT NOT NULL,
    test_name TEXT NOT NULL,
    outcome TEXT NOT NULL,
    duration_ms INTEGER,
    output_json TEXT,
    created_at TEXT NOT NULL
);

-- Copilot chat sessions
CREATE TABLE IF NOT EXISTS copilot_sessions (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    vscode_session_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    metadata_json TEXT,
    UNIQUE(workspace_id, vscode_session_id)
);

-- Copilot messages
CREATE TABLE IF NOT EXISTS copilot_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES copilot_sessions(id),
    request_id TEXT,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    variables_json TEXT,
    timestamp TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_commits_workspace ON commits(workspace_id);
CREATE INDEX IF NOT EXISTS idx_commits_timestamp ON commits(timestamp);
CREATE INDEX IF NOT EXISTS idx_commits_sha ON commits(sha);
CREATE INDEX IF NOT EXISTS idx_test_runs_workspace ON test_runs(workspace_id);
CREATE INDEX IF NOT EXISTS idx_test_runs_started ON test_runs(started_at);
CREATE INDEX IF NOT EXISTS idx_test_results_run ON test_results(run_id);
CREATE INDEX IF NOT EXISTS idx_test_results_outcome ON test_results(outcome);
CREATE INDEX IF NOT EXISTS idx_copilot_sessions_workspace ON copilot_sessions(workspace_id);
CREATE INDEX IF NOT EXISTS idx_copilot_messages_session ON copilot_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_copilot_messages_timestamp ON copilot_messages(timestamp);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;

    #[test]
    fn test_database_in_memory() {
        let db = Database::in_memory().expect("should create in-memory db");
        assert!(!db.is_initialized());
    }

    #[test]
    fn test_database_initialize() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");
        assert!(db.is_initialized());
    }

    #[test]
    fn test_database_initialize_idempotent() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("first init");
        db.initialize().expect("second init should succeed");
        assert!(db.is_initialized());
    }

    #[test]
    fn test_database_tables_created() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let tables = vec![
            "workspaces",
            "commits",
            "test_runs",
            "test_results",
            "copilot_sessions",
            "copilot_messages",
        ];

        for table in tables {
            let exists: i32 = db
                .connection()
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    [table],
                    |row| row.get(0),
                )
                .expect("query should succeed");
            assert_eq!(exists, 1, "Table {table} should exist");
        }
    }

    #[test]
    fn test_database_count_empty() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        assert_eq!(db.count("workspaces").expect("count"), 0);
        assert_eq!(db.count("commits").expect("count"), 0);
    }

    #[test]
    fn test_database_insert_workspace() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        db.connection()
            .execute(
                "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                [
                    "550e8400-e29b-41d4-a716-446655440000",
                    "test-workspace",
                    "/path/to/workspace",
                    "2026-01-17T02:33:06Z",
                    "2026-01-17T02:33:06Z",
                ],
            )
            .expect("insert should succeed");

        assert_eq!(db.count("workspaces").expect("count"), 1);
    }

    #[test]
    fn test_database_foreign_key_references() {
        let db = Database::in_memory().expect("should create db");
        // Enable foreign key enforcement
        db.connection()
            .execute("PRAGMA foreign_keys = ON", [])
            .expect("pragma");
        db.initialize().expect("should initialize");

        // Insert a workspace first
        db.connection()
            .execute(
                "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                ["ws-1", "test", "/test", "2026-01-17T00:00:00Z", "2026-01-17T00:00:00Z"],
            )
            .expect("insert workspace");

        // Insert a commit referencing the workspace
        db.connection()
            .execute(
                "INSERT INTO commits (id, workspace_id, sha, author, message, timestamp, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                ["c-1", "ws-1", "abc123", "Test Author", "Test commit", "2026-01-17T00:00:00Z", "2026-01-17T00:00:00Z"],
            )
            .expect("insert commit");

        assert_eq!(db.count("commits").expect("count"), 1);
    }

    #[test]
    fn test_database_json_column() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        // Insert workspace
        db.connection()
            .execute(
                "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                ["ws-1", "test", "/test", "2026-01-17T00:00:00Z", "2026-01-17T00:00:00Z"],
            )
            .expect("insert workspace");

        // Insert commit with JSON data
        let parents_json = r#"["parent1", "parent2"]"#;
        let diff_json = r#"{"files": [{"path": "test.rs", "added": 10, "deleted": 5}]}"#;

        db.connection()
            .execute(
                "INSERT INTO commits (id, workspace_id, sha, author, message, timestamp, parents_json, diff_json, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                ["c-1", "ws-1", "abc123", "Author", "Message", "2026-01-17T00:00:00Z", parents_json, diff_json, "2026-01-17T00:00:00Z"],
            )
            .expect("insert commit");

        // Query and verify JSON is stored correctly
        let stored_json: String = db
            .connection()
            .query_row(
                "SELECT parents_json FROM commits WHERE id = ?",
                ["c-1"],
                |row| row.get(0),
            )
            .expect("query");

        assert_eq!(stored_json, parents_json);
    }

    #[test]
    fn test_database_unique_constraint() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        // Insert workspace
        db.connection()
            .execute(
                "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                ["ws-1", "test", "/test", "2026-01-17T00:00:00Z", "2026-01-17T00:00:00Z"],
            )
            .expect("insert workspace");

        // Try to insert duplicate path
        let result = db.connection().execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
            ["ws-2", "test2", "/test", "2026-01-17T00:00:00Z", "2026-01-17T00:00:00Z"],
        );

        assert!(result.is_err(), "Duplicate path should fail");
    }

    #[test]
    fn test_database_indexes_created() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let indexes = vec![
            "idx_commits_workspace",
            "idx_commits_timestamp",
            "idx_commits_sha",
            "idx_test_runs_workspace",
            "idx_test_results_run",
            "idx_copilot_sessions_workspace",
            "idx_copilot_messages_session",
        ];

        for index in indexes {
            let exists: i32 = db
                .connection()
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?",
                    [index],
                    |row| row.get(0),
                )
                .expect("query should succeed");
            assert_eq!(exists, 1, "Index {index} should exist");
        }
    }
}
