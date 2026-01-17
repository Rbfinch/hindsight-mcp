//! Database module for hindsight-mcp
//!
//! This module provides SQLite database operations for storing and querying
//! development history data including git commits, test results, and Copilot sessions.

use crate::migrations;
use rusqlite::Connection;
use thiserror::Error;

/// Database errors
#[derive(Debug, Error)]
pub enum DbError {
    /// SQLite error
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(#[from] migrations::MigrationError),

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

    /// Initialize the database schema using migrations
    ///
    /// # Errors
    ///
    /// Returns an error if the schema cannot be created.
    pub fn initialize(&self) -> Result<(), DbError> {
        migrations::migrate(&self.conn)?;
        Ok(())
    }

    /// Check if the database is initialized and up to date
    pub fn is_initialized(&self) -> bool {
        migrations::is_up_to_date(&self.conn)
    }

    /// Get the current schema version
    ///
    /// # Errors
    ///
    /// Returns an error if the version cannot be read.
    pub fn schema_version(&self) -> Result<i32, DbError> {
        Ok(migrations::get_version(&self.conn)?)
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

    #[test]
    fn test_database_schema_version() {
        let db = Database::in_memory().expect("should create db");

        // Before initialization, version should be 0
        assert_eq!(db.schema_version().expect("version"), 0);

        db.initialize().expect("should initialize");

        // After initialization, version should match CURRENT_VERSION
        assert_eq!(
            db.schema_version().expect("version"),
            crate::migrations::CURRENT_VERSION
        );
    }

    #[test]
    fn test_database_fts_tables_created() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let fts_tables = vec!["commits_fts", "copilot_messages_fts"];

        for table in fts_tables {
            let exists: i32 = db
                .connection()
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    [table],
                    |row| row.get(0),
                )
                .expect("query should succeed");
            assert_eq!(exists, 1, "FTS table {table} should exist");
        }
    }

    #[test]
    fn test_database_views_created() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let views = vec!["timeline", "failing_tests", "recent_activity"];

        for view in views {
            let exists: i32 = db
                .connection()
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='view' AND name=?",
                    [view],
                    |row| row.get(0),
                )
                .expect("query should succeed");
            assert_eq!(exists, 1, "View {view} should exist");
        }
    }
}
