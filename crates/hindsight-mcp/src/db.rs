// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Database module for hindsight-mcp
//!
//! This module provides SQLite database operations for storing and querying
//! development history data including git commits, test results, and Copilot sessions.
//!
//! # Insertion Functions
//!
//! - [`Database::insert_workspace`] / [`Database::get_or_create_workspace`] - Workspace management
//! - [`Database::insert_commit`] / [`Database::insert_commits_batch`] - Git commit insertion
//! - [`Database::insert_test_run`] / [`Database::insert_test_results_batch`] - Test result insertion
//! - [`Database::insert_copilot_session`] / [`Database::insert_copilot_messages_batch`] - Copilot data

use crate::migrations;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Transaction, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

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

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ============================================================================
// Workspace Types
// ============================================================================

/// A workspace record for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    /// Workspace ID (UUID)
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Absolute filesystem path
    pub path: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl WorkspaceRecord {
    /// Create a new workspace record with auto-generated ID
    #[must_use]
    pub fn new(name: String, path: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            path,
            created_at: now,
            updated_at: now,
        }
    }
}

// ============================================================================
// Commit Types
// ============================================================================

/// A commit record for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRecord {
    /// Commit ID (UUID)
    pub id: String,
    /// Workspace ID (FK)
    pub workspace_id: String,
    /// Git SHA (40 hex chars)
    pub sha: String,
    /// Author name
    pub author: String,
    /// Author email
    pub author_email: Option<String>,
    /// Full commit message
    pub message: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Parent SHAs as JSON array
    pub parents_json: Option<String>,
    /// Diff summary as JSON
    pub diff_json: Option<String>,
    /// Record creation time
    pub created_at: DateTime<Utc>,
}

impl CommitRecord {
    /// Create a commit record with auto-generated ID
    #[must_use]
    pub fn new(
        workspace_id: String,
        sha: String,
        author: String,
        author_email: Option<String>,
        message: String,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workspace_id,
            sha,
            author,
            author_email,
            message,
            timestamp,
            parents_json: None,
            diff_json: None,
            created_at: Utc::now(),
        }
    }

    /// Set parent SHAs
    #[must_use]
    pub fn with_parents(mut self, parents: Vec<String>) -> Self {
        self.parents_json = Some(serde_json::to_string(&parents).unwrap_or_default());
        self
    }

    /// Set diff JSON
    #[must_use]
    pub fn with_diff_json(mut self, diff_json: String) -> Self {
        self.diff_json = Some(diff_json);
        self
    }
}

// ============================================================================
// Test Types
// ============================================================================

/// A test run record for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunRecord {
    /// Test run ID (UUID)
    pub id: String,
    /// Workspace ID (FK)
    pub workspace_id: String,
    /// Git SHA at time of run
    pub commit_sha: Option<String>,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// Finish timestamp
    pub finished_at: Option<DateTime<Utc>>,
    /// Number of passed tests
    pub passed_count: i32,
    /// Number of failed tests
    pub failed_count: i32,
    /// Number of ignored tests
    pub ignored_count: i32,
    /// Build metadata as JSON
    pub metadata_json: Option<String>,
}

impl TestRunRecord {
    /// Create a test run record with auto-generated ID and started_at
    #[must_use]
    pub fn new(workspace_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workspace_id,
            commit_sha: None,
            started_at: Utc::now(),
            finished_at: None,
            passed_count: 0,
            failed_count: 0,
            ignored_count: 0,
            metadata_json: None,
        }
    }

    /// Set commit SHA
    #[must_use]
    pub fn with_commit(mut self, sha: &str) -> Self {
        self.commit_sha = Some(sha.to_string());
        self
    }

    /// Set finished timestamp and counts
    #[must_use]
    pub fn finished(mut self, passed: i32, failed: i32, ignored: i32) -> Self {
        self.finished_at = Some(Utc::now());
        self.passed_count = passed;
        self.failed_count = failed;
        self.ignored_count = ignored;
        self
    }
}

/// A test result record for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResultRecord {
    /// Test result ID (UUID)
    pub id: String,
    /// Test run ID (FK)
    pub run_id: String,
    /// Suite/binary name
    pub suite_name: String,
    /// Full test name
    pub test_name: String,
    /// Outcome: passed/failed/ignored/timedout
    pub outcome: String,
    /// Duration in milliseconds
    pub duration_ms: Option<i64>,
    /// Output as JSON
    pub output_json: Option<String>,
    /// Record creation time
    pub created_at: DateTime<Utc>,
}

impl TestResultRecord {
    /// Create a test result record with auto-generated ID
    #[must_use]
    pub fn new(
        run_id: String,
        suite_name: String,
        test_name: String,
        outcome: String,
        duration_ms: Option<i64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            run_id,
            suite_name,
            test_name,
            outcome,
            duration_ms,
            output_json: None,
            created_at: Utc::now(),
        }
    }

    /// Set output JSON
    #[must_use]
    pub fn with_output(mut self, stdout: Option<&str>, stderr: Option<&str>) -> Self {
        if stdout.is_some() || stderr.is_some() {
            let output = serde_json::json!({
                "stdout": stdout,
                "stderr": stderr
            });
            self.output_json = Some(output.to_string());
        }
        self
    }
}

// ============================================================================
// Copilot Types
// ============================================================================

/// A Copilot session record for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotSessionRecord {
    /// Session ID (UUID)
    pub id: String,
    /// Workspace ID (FK)
    pub workspace_id: String,
    /// Original VS Code session ID
    pub vscode_session_id: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Metadata as JSON
    pub metadata_json: Option<String>,
}

impl CopilotSessionRecord {
    /// Create a session record with auto-generated ID and timestamps
    #[must_use]
    pub fn new(workspace_id: String, vscode_session_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            workspace_id,
            vscode_session_id,
            created_at: now,
            updated_at: now,
            metadata_json: None,
        }
    }

    /// Set metadata JSON
    #[must_use]
    pub fn with_metadata(mut self, model: Option<&str>, mode: Option<&str>) -> Self {
        if model.is_some() || mode.is_some() {
            let metadata = serde_json::json!({
                "model": model,
                "mode": mode
            });
            self.metadata_json = Some(metadata.to_string());
        }
        self
    }
}

/// A Copilot message record for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotMessageRecord {
    /// Message ID (UUID)
    pub id: String,
    /// Session ID (FK)
    pub session_id: String,
    /// Original request ID
    pub request_id: Option<String>,
    /// Role: user/assistant/system
    pub role: String,
    /// Message content
    pub content: String,
    /// Variables as JSON
    pub variables_json: Option<String>,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Record creation time
    pub created_at: DateTime<Utc>,
}

impl CopilotMessageRecord {
    /// Create a message record with auto-generated ID
    #[must_use]
    pub fn new(
        session_id: String,
        role: String,
        content: String,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            request_id: None,
            role,
            content,
            variables_json: None,
            timestamp,
            created_at: Utc::now(),
        }
    }

    /// Set request ID
    #[must_use]
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = Some(request_id.to_string());
        self
    }

    /// Set variables JSON
    #[must_use]
    pub fn with_variables_json(mut self, json: String) -> Self {
        self.variables_json = Some(json);
        self
    }
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

    // ========================================================================
    // Workspace Management
    // ========================================================================

    /// Insert a new workspace
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails (e.g., duplicate path).
    pub fn insert_workspace(&self, record: &WorkspaceRecord) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                record.id,
                record.name,
                record.path,
                record.created_at.to_rfc3339(),
                record.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get or create a workspace by path
    ///
    /// Returns the workspace ID. If the workspace already exists, returns its ID.
    /// Otherwise, creates a new workspace and returns the new ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn get_or_create_workspace(&self, name: &str, path: &str) -> Result<String, DbError> {
        // Try to find existing workspace
        let existing: Result<String, _> =
            self.conn
                .query_row("SELECT id FROM workspaces WHERE path = ?1", [path], |row| {
                    row.get(0)
                });

        match existing {
            Ok(id) => Ok(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Create new workspace
                let record = WorkspaceRecord::new(name.to_string(), path.to_string());
                self.insert_workspace(&record)?;
                Ok(record.id)
            }
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }

    /// List all workspaces
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceRecord>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, created_at, updated_at FROM workspaces ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(WorkspaceRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                created_at: parse_timestamp(row.get::<_, String>(3)?),
                updated_at: parse_timestamp(row.get::<_, String>(4)?),
            })
        })?;

        let mut workspaces = Vec::new();
        for row in rows {
            workspaces.push(row?);
        }
        Ok(workspaces)
    }

    // ========================================================================
    // Commit Insertion
    // ========================================================================

    /// Insert a single commit
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails.
    pub fn insert_commit(&self, record: &CommitRecord) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO commits (id, workspace_id, sha, author, author_email, message, timestamp, parents_json, diff_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                record.id,
                record.workspace_id,
                record.sha,
                record.author,
                record.author_email,
                record.message,
                record.timestamp.to_rfc3339(),
                record.parents_json,
                record.diff_json,
                record.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Insert multiple commits in a transaction
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails. All inserts are rolled back on error.
    pub fn insert_commits_batch(&mut self, records: &[CommitRecord]) -> Result<usize, DbError> {
        let tx = self.conn.transaction()?;
        let count = Self::insert_commits_in_tx(&tx, records)?;
        tx.commit()?;
        Ok(count)
    }

    fn insert_commits_in_tx(
        tx: &Transaction<'_>,
        records: &[CommitRecord],
    ) -> Result<usize, DbError> {
        let mut count = 0;
        for record in records {
            tx.execute(
                "INSERT OR IGNORE INTO commits (id, workspace_id, sha, author, author_email, message, timestamp, parents_json, diff_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    record.id,
                    record.workspace_id,
                    record.sha,
                    record.author,
                    record.author_email,
                    record.message,
                    record.timestamp.to_rfc3339(),
                    record.parents_json,
                    record.diff_json,
                    record.created_at.to_rfc3339(),
                ],
            )?;
            count += 1;
        }
        Ok(count)
    }

    /// Get a commit by SHA within a workspace
    ///
    /// # Errors
    ///
    /// Returns `DbError::NotFound` if the commit doesn't exist.
    pub fn get_commit_by_sha(
        &self,
        workspace_id: &str,
        sha: &str,
    ) -> Result<CommitRecord, DbError> {
        self.conn
            .query_row(
                "SELECT id, workspace_id, sha, author, author_email, message, timestamp, parents_json, diff_json, created_at
                 FROM commits WHERE workspace_id = ?1 AND sha = ?2",
                [workspace_id, sha],
                |row| {
                    Ok(CommitRecord {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        sha: row.get(2)?,
                        author: row.get(3)?,
                        author_email: row.get(4)?,
                        message: row.get(5)?,
                        timestamp: parse_timestamp(row.get::<_, String>(6)?),
                        parents_json: row.get(7)?,
                        diff_json: row.get(8)?,
                        created_at: parse_timestamp(row.get::<_, String>(9)?),
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    table: "commits".to_string(),
                    id: sha.to_string(),
                },
                _ => DbError::Sqlite(e),
            })
    }

    // ========================================================================
    // Test Result Insertion
    // ========================================================================

    /// Insert a test run
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails.
    pub fn insert_test_run(&self, record: &TestRunRecord) -> Result<String, DbError> {
        self.conn.execute(
            "INSERT INTO test_runs (id, workspace_id, commit_sha, started_at, finished_at, passed_count, failed_count, ignored_count, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.workspace_id,
                record.commit_sha,
                record.started_at.to_rfc3339(),
                record.finished_at.map(|t| t.to_rfc3339()),
                record.passed_count,
                record.failed_count,
                record.ignored_count,
                record.metadata_json,
            ],
        )?;
        Ok(record.id.clone())
    }

    /// Insert multiple test results in a transaction
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails.
    pub fn insert_test_results_batch(
        &mut self,
        records: &[TestResultRecord],
    ) -> Result<usize, DbError> {
        let tx = self.conn.transaction()?;
        let count = Self::insert_test_results_in_tx(&tx, records)?;
        tx.commit()?;
        Ok(count)
    }

    fn insert_test_results_in_tx(
        tx: &Transaction<'_>,
        records: &[TestResultRecord],
    ) -> Result<usize, DbError> {
        let mut count = 0;
        for record in records {
            tx.execute(
                "INSERT INTO test_results (id, run_id, suite_name, test_name, outcome, duration_ms, output_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.id,
                    record.run_id,
                    record.suite_name,
                    record.test_name,
                    record.outcome,
                    record.duration_ms,
                    record.output_json,
                    record.created_at.to_rfc3339(),
                ],
            )?;
            count += 1;
        }
        Ok(count)
    }

    /// Link a test run to a commit SHA
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    pub fn link_test_run_to_commit(&self, run_id: &str, commit_sha: &str) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE test_runs SET commit_sha = ?1 WHERE id = ?2",
            [commit_sha, run_id],
        )?;
        Ok(())
    }

    // ========================================================================
    // Copilot Insertion
    // ========================================================================

    /// Insert a Copilot session
    ///
    /// Returns the session ID. Uses INSERT OR IGNORE to handle duplicates.
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails.
    pub fn insert_copilot_session(&self, record: &CopilotSessionRecord) -> Result<String, DbError> {
        // Check if session already exists
        let existing: Result<String, _> = self.conn.query_row(
            "SELECT id FROM copilot_sessions WHERE workspace_id = ?1 AND vscode_session_id = ?2",
            [&record.workspace_id, &record.vscode_session_id],
            |row| row.get(0),
        );

        match existing {
            Ok(id) => Ok(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                self.conn.execute(
                    "INSERT INTO copilot_sessions (id, workspace_id, vscode_session_id, created_at, updated_at, metadata_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.vscode_session_id,
                        record.created_at.to_rfc3339(),
                        record.updated_at.to_rfc3339(),
                        record.metadata_json,
                    ],
                )?;
                Ok(record.id.clone())
            }
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }

    /// Insert multiple Copilot messages in a transaction
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails.
    pub fn insert_copilot_messages_batch(
        &mut self,
        records: &[CopilotMessageRecord],
    ) -> Result<usize, DbError> {
        let tx = self.conn.transaction()?;
        let count = Self::insert_copilot_messages_in_tx(&tx, records)?;
        tx.commit()?;
        Ok(count)
    }

    fn insert_copilot_messages_in_tx(
        tx: &Transaction<'_>,
        records: &[CopilotMessageRecord],
    ) -> Result<usize, DbError> {
        let mut count = 0;
        for record in records {
            tx.execute(
                "INSERT INTO copilot_messages (id, session_id, request_id, role, content, variables_json, timestamp, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.id,
                    record.session_id,
                    record.request_id,
                    record.role,
                    record.content,
                    record.variables_json,
                    record.timestamp.to_rfc3339(),
                    record.created_at.to_rfc3339(),
                ],
            )?;
            count += 1;
        }
        Ok(count)
    }

    /// Get message count for a session
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn get_session_message_count(&self, session_id: &str) -> Result<i64, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM copilot_messages WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

/// Parse an ISO 8601 timestamp string
fn parse_timestamp(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
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

    // ========================================================================
    // Record Type Tests
    // ========================================================================

    #[test]
    fn test_workspace_record_new() {
        let record = WorkspaceRecord::new("test".to_string(), "/path".to_string());
        assert!(!record.id.is_empty());
        assert_eq!(record.name, "test");
        assert_eq!(record.path, "/path");
    }

    #[test]
    fn test_commit_record_new() {
        let record = CommitRecord::new(
            "ws-1".to_string(),
            "abc123".to_string(),
            "Author".to_string(),
            Some("author@example.com".to_string()),
            "Test message".to_string(),
            Utc::now(),
        );
        assert!(!record.id.is_empty());
        assert_eq!(record.workspace_id, "ws-1");
        assert_eq!(record.sha, "abc123");
        assert!(record.parents_json.is_none());
        assert!(record.diff_json.is_none());
    }

    #[test]
    fn test_commit_record_with_parents() {
        let record = CommitRecord::new(
            "ws-1".to_string(),
            "abc123".to_string(),
            "Author".to_string(),
            Some("author@example.com".to_string()),
            "Test".to_string(),
            Utc::now(),
        )
        .with_parents(vec!["parent1".to_string(), "parent2".to_string()]);

        assert!(record.parents_json.is_some());
        let json = record.parents_json.unwrap();
        assert!(json.contains("parent1"));
        assert!(json.contains("parent2"));
    }

    #[test]
    fn test_test_run_record_new() {
        let record = TestRunRecord::new("ws-1".to_string());
        assert!(!record.id.is_empty());
        assert_eq!(record.workspace_id, "ws-1");
        assert!(record.commit_sha.is_none());
        assert!(record.finished_at.is_none());
        assert_eq!(record.passed_count, 0);
    }

    #[test]
    fn test_test_run_record_finished() {
        let record = TestRunRecord::new("ws-1".to_string()).finished(10, 2, 1);
        assert!(record.finished_at.is_some());
        assert_eq!(record.passed_count, 10);
        assert_eq!(record.failed_count, 2);
        assert_eq!(record.ignored_count, 1);
    }

    #[test]
    fn test_copilot_session_record_new() {
        let record = CopilotSessionRecord::new("ws-1".to_string(), "vscode-123".to_string());
        assert!(!record.id.is_empty());
        assert_eq!(record.workspace_id, "ws-1");
        assert_eq!(record.vscode_session_id, "vscode-123");
    }

    #[test]
    fn test_copilot_message_record_new() {
        let record = CopilotMessageRecord::new(
            "session-1".to_string(),
            "user".to_string(),
            "Hello".to_string(),
            Utc::now(),
        );
        assert!(!record.id.is_empty());
        assert_eq!(record.session_id, "session-1");
        assert_eq!(record.role, "user");
        assert_eq!(record.content, "Hello");
        assert!(record.request_id.is_none());
    }

    // ========================================================================
    // Workspace Insertion Tests
    // ========================================================================

    #[test]
    fn test_insert_workspace_via_method() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let record =
            WorkspaceRecord::new("my-project".to_string(), "/home/user/project".to_string());
        db.insert_workspace(&record).expect("insert should succeed");

        assert_eq!(db.count("workspaces").expect("count"), 1);
    }

    #[test]
    fn test_get_or_create_workspace_creates() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let id = db
            .get_or_create_workspace("new-project", "/new/path")
            .expect("should create");
        assert!(!id.is_empty());
        assert_eq!(db.count("workspaces").expect("count"), 1);
    }

    #[test]
    fn test_get_or_create_workspace_returns_existing() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let id1 = db
            .get_or_create_workspace("project", "/path")
            .expect("first call");
        let id2 = db
            .get_or_create_workspace("project", "/path")
            .expect("second call");

        assert_eq!(id1, id2);
        assert_eq!(db.count("workspaces").expect("count"), 1);
    }

    #[test]
    fn test_list_workspaces() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        db.get_or_create_workspace("alpha", "/alpha")
            .expect("create");
        db.get_or_create_workspace("beta", "/beta").expect("create");

        let workspaces = db.list_workspaces().expect("list");
        assert_eq!(workspaces.len(), 2);
        assert_eq!(workspaces[0].name, "alpha"); // Sorted by name
        assert_eq!(workspaces[1].name, "beta");
    }

    // ========================================================================
    // Commit Insertion Tests
    // ========================================================================

    #[test]
    fn test_insert_commit_via_method() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let record = CommitRecord::new(
            ws_id,
            "abc123def456".to_string(),
            "Test Author".to_string(),
            Some("test@example.com".to_string()),
            "Initial commit".to_string(),
            Utc::now(),
        );

        db.insert_commit(&record).expect("insert should succeed");
        assert_eq!(db.count("commits").expect("count"), 1);
    }

    #[test]
    fn test_insert_commits_batch() {
        let mut db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let records: Vec<CommitRecord> = (0..5)
            .map(|i| {
                CommitRecord::new(
                    ws_id.clone(),
                    format!("sha{i}"),
                    "Author".to_string(),
                    Some("a@b.com".to_string()),
                    format!("Commit {i}"),
                    Utc::now(),
                )
            })
            .collect();

        let count = db.insert_commits_batch(&records).expect("batch insert");
        assert_eq!(count, 5);
        assert_eq!(db.count("commits").expect("count"), 5);
    }

    #[test]
    fn test_get_commit_by_sha() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let record = CommitRecord::new(
            ws_id.clone(),
            "findme123".to_string(),
            "Author".to_string(),
            Some("a@b.com".to_string()),
            "Find this commit".to_string(),
            Utc::now(),
        );
        db.insert_commit(&record).expect("insert");

        let found = db
            .get_commit_by_sha(&ws_id, "findme123")
            .expect("should find");
        assert_eq!(found.message, "Find this commit");
    }

    #[test]
    fn test_get_commit_by_sha_not_found() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let result = db.get_commit_by_sha(&ws_id, "nonexistent");

        assert!(matches!(result, Err(DbError::NotFound { .. })));
    }

    // ========================================================================
    // Test Result Insertion Tests
    // ========================================================================

    #[test]
    fn test_insert_test_run() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let record = TestRunRecord::new(ws_id).finished(10, 2, 1);

        let run_id = db.insert_test_run(&record).expect("insert");
        assert!(!run_id.is_empty());
        assert_eq!(db.count("test_runs").expect("count"), 1);
    }

    #[test]
    fn test_insert_test_results_batch() {
        let mut db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let run_record = TestRunRecord::new(ws_id);
        let run_id = db.insert_test_run(&run_record).expect("insert run");

        let results: Vec<TestResultRecord> = (0..3)
            .map(|i| {
                TestResultRecord::new(
                    run_id.clone(),
                    "my_suite".to_string(),
                    format!("test_{i}"),
                    "passed".to_string(),
                    Some(100 + i as i64),
                )
            })
            .collect();

        let count = db
            .insert_test_results_batch(&results)
            .expect("batch insert");
        assert_eq!(count, 3);
        assert_eq!(db.count("test_results").expect("count"), 3);
    }

    #[test]
    fn test_link_test_run_to_commit() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");

        // Create commit
        let commit = CommitRecord::new(
            ws_id.clone(),
            "commitsha".to_string(),
            "Author".to_string(),
            Some("a@b.com".to_string()),
            "Test".to_string(),
            Utc::now(),
        );
        db.insert_commit(&commit).expect("insert commit");

        // Create test run without commit
        let run = TestRunRecord::new(ws_id);
        let run_id = db.insert_test_run(&run).expect("insert run");

        // Link them
        db.link_test_run_to_commit(&run_id, "commitsha")
            .expect("link");

        // Verify
        let linked_sha: String = db
            .connection()
            .query_row(
                "SELECT commit_sha FROM test_runs WHERE id = ?",
                [&run_id],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(linked_sha, "commitsha");
    }

    // ========================================================================
    // Copilot Insertion Tests
    // ========================================================================

    #[test]
    fn test_insert_copilot_session() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let record = CopilotSessionRecord::new(ws_id, "vscode-session-abc".to_string());

        let session_id = db.insert_copilot_session(&record).expect("insert");
        assert!(!session_id.is_empty());
        assert_eq!(db.count("copilot_sessions").expect("count"), 1);
    }

    #[test]
    fn test_insert_copilot_session_idempotent() {
        let db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let record = CopilotSessionRecord::new(ws_id.clone(), "same-session".to_string());

        let id1 = db.insert_copilot_session(&record).expect("first insert");

        // Try inserting again with same vscode_session_id
        let record2 = CopilotSessionRecord::new(ws_id, "same-session".to_string());
        let id2 = db.insert_copilot_session(&record2).expect("second insert");

        assert_eq!(id1, id2);
        assert_eq!(db.count("copilot_sessions").expect("count"), 1);
    }

    #[test]
    fn test_insert_copilot_messages_batch() {
        let mut db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let session_record = CopilotSessionRecord::new(ws_id, "vscode-123".to_string());
        let session_id = db
            .insert_copilot_session(&session_record)
            .expect("insert session");

        let messages: Vec<CopilotMessageRecord> = vec![
            CopilotMessageRecord::new(
                session_id.clone(),
                "user".to_string(),
                "Hello".to_string(),
                Utc::now(),
            ),
            CopilotMessageRecord::new(
                session_id.clone(),
                "assistant".to_string(),
                "Hi there!".to_string(),
                Utc::now(),
            ),
            CopilotMessageRecord::new(
                session_id.clone(),
                "user".to_string(),
                "Help me".to_string(),
                Utc::now(),
            ),
        ];

        let count = db
            .insert_copilot_messages_batch(&messages)
            .expect("batch insert");
        assert_eq!(count, 3);
        assert_eq!(db.count("copilot_messages").expect("count"), 3);
    }

    #[test]
    fn test_get_session_message_count() {
        let mut db = Database::in_memory().expect("should create db");
        db.initialize().expect("should initialize");

        let ws_id = db
            .get_or_create_workspace("test", "/test")
            .expect("workspace");
        let session_record = CopilotSessionRecord::new(ws_id, "vscode-456".to_string());
        let session_id = db
            .insert_copilot_session(&session_record)
            .expect("insert session");

        // Initially empty
        assert_eq!(db.get_session_message_count(&session_id).expect("count"), 0);

        // Add messages
        let messages: Vec<CopilotMessageRecord> = vec![
            CopilotMessageRecord::new(
                session_id.clone(),
                "user".to_string(),
                "Q1".to_string(),
                Utc::now(),
            ),
            CopilotMessageRecord::new(
                session_id.clone(),
                "assistant".to_string(),
                "A1".to_string(),
                Utc::now(),
            ),
        ];
        db.insert_copilot_messages_batch(&messages).expect("insert");

        assert_eq!(db.get_session_message_count(&session_id).expect("count"), 2);
    }
}
