//! Database migrations for hindsight-mcp
//!
//! This module provides schema migration functionality, allowing the database
//! schema to evolve over time while maintaining backward compatibility.

use rusqlite::Connection;
use thiserror::Error;

/// Migration errors
#[derive(Debug, Error)]
pub enum MigrationError {
    /// SQLite error during migration
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Migration version mismatch
    #[error("Migration version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: i32, found: i32 },

    /// Migration already applied
    #[error("Migration {version} already applied")]
    AlreadyApplied { version: i32 },
}

/// Current schema version
pub const CURRENT_VERSION: i32 = 1;

/// A database migration
#[allow(dead_code)]
pub struct Migration {
    /// Migration version number
    pub version: i32,
    /// Migration name/description
    pub name: &'static str,
    /// SQL to apply the migration
    pub up: &'static str,
    /// SQL to revert the migration (optional)
    pub down: Option<&'static str>,
}

/// All available migrations in order
pub static MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial_schema",
    up: include_str!("schema.sql"),
    down: Some(
        r#"
        DROP VIEW IF EXISTS recent_activity;
        DROP VIEW IF EXISTS failing_tests;
        DROP VIEW IF EXISTS timeline;
        DROP TRIGGER IF EXISTS copilot_messages_au;
        DROP TRIGGER IF EXISTS copilot_messages_ad;
        DROP TRIGGER IF EXISTS copilot_messages_ai;
        DROP TRIGGER IF EXISTS commits_au;
        DROP TRIGGER IF EXISTS commits_ad;
        DROP TRIGGER IF EXISTS commits_ai;
        DROP TABLE IF EXISTS copilot_messages_fts;
        DROP TABLE IF EXISTS commits_fts;
        DROP TABLE IF EXISTS copilot_messages;
        DROP TABLE IF EXISTS copilot_sessions;
        DROP TABLE IF EXISTS test_results;
        DROP TABLE IF EXISTS test_runs;
        DROP TABLE IF EXISTS commits;
        DROP TABLE IF EXISTS workspaces;
        DROP TABLE IF EXISTS schema_migrations;
    "#,
    ),
}];

/// Get the current schema version from the database
///
/// Returns 0 if no migrations have been applied.
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn get_version(conn: &Connection) -> Result<i32, MigrationError> {
    // Check if schema_migrations table exists
    let table_exists: i32 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
        [],
        |row| row.get(0),
    )?;

    if table_exists == 0 {
        return Ok(0);
    }

    // Get the latest version
    let version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(version)
}

/// Apply all pending migrations
///
/// # Errors
///
/// Returns an error if any migration fails.
pub fn migrate(conn: &Connection) -> Result<Vec<i32>, MigrationError> {
    let current_version = get_version(conn)?;
    let mut applied = Vec::new();

    for migration in MIGRATIONS {
        if migration.version > current_version {
            apply_migration(conn, migration)?;
            applied.push(migration.version);
        }
    }

    Ok(applied)
}

/// Apply a single migration
///
/// # Errors
///
/// Returns an error if the migration fails.
pub fn apply_migration(conn: &Connection, migration: &Migration) -> Result<(), MigrationError> {
    // Execute the migration SQL
    conn.execute_batch(migration.up)?;

    Ok(())
}

/// Rollback to a specific version
///
/// # Errors
///
/// Returns an error if the rollback fails or if down migrations are not available.
#[allow(dead_code)]
pub fn rollback_to(conn: &Connection, target_version: i32) -> Result<Vec<i32>, MigrationError> {
    let current_version = get_version(conn)?;
    let mut rolled_back = Vec::new();

    // Find migrations to rollback (in reverse order)
    for migration in MIGRATIONS.iter().rev() {
        if migration.version > target_version && migration.version <= current_version {
            if let Some(down) = migration.down {
                conn.execute_batch(down)?;
                rolled_back.push(migration.version);
            }
        }
    }

    Ok(rolled_back)
}

/// Check if the database is up to date
#[must_use]
pub fn is_up_to_date(conn: &Connection) -> bool {
    get_version(conn)
        .map(|v| v >= CURRENT_VERSION)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_get_version_empty_db() {
        let conn = Connection::open_in_memory().expect("create db");
        let version = get_version(&conn).expect("get version");
        assert_eq!(version, 0);
    }

    #[test]
    fn test_migrate_applies_all() {
        let conn = Connection::open_in_memory().expect("create db");
        let applied = migrate(&conn).expect("migrate");

        assert!(!applied.is_empty());
        assert_eq!(applied[0], 1);

        let version = get_version(&conn).expect("get version");
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn test_migrate_idempotent() {
        let conn = Connection::open_in_memory().expect("create db");

        let first = migrate(&conn).expect("first migrate");
        assert!(!first.is_empty());

        let second = migrate(&conn).expect("second migrate");
        assert!(second.is_empty(), "Second migrate should apply nothing");
    }

    #[test]
    fn test_is_up_to_date() {
        let conn = Connection::open_in_memory().expect("create db");

        assert!(!is_up_to_date(&conn));

        migrate(&conn).expect("migrate");

        assert!(is_up_to_date(&conn));
    }

    #[test]
    fn test_migration_creates_tables() {
        let conn = Connection::open_in_memory().expect("create db");
        migrate(&conn).expect("migrate");

        let tables = [
            "workspaces",
            "commits",
            "test_runs",
            "test_results",
            "copilot_sessions",
            "copilot_messages",
            "schema_migrations",
        ];

        for table in tables {
            let exists: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    [table],
                    |row| row.get(0),
                )
                .expect("query");
            assert_eq!(exists, 1, "Table {} should exist", table);
        }
    }

    #[test]
    fn test_migration_creates_fts_tables() {
        let conn = Connection::open_in_memory().expect("create db");
        migrate(&conn).expect("migrate");

        let fts_tables = ["commits_fts", "copilot_messages_fts"];

        for table in fts_tables {
            let exists: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    [table],
                    |row| row.get(0),
                )
                .expect("query");
            assert_eq!(exists, 1, "FTS table {} should exist", table);
        }
    }

    #[test]
    fn test_migration_creates_views() {
        let conn = Connection::open_in_memory().expect("create db");
        migrate(&conn).expect("migrate");

        let views = ["timeline", "failing_tests", "recent_activity"];

        for view in views {
            let exists: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='view' AND name=?",
                    [view],
                    |row| row.get(0),
                )
                .expect("query");
            assert_eq!(exists, 1, "View {} should exist", view);
        }
    }

    #[test]
    fn test_rollback() {
        let conn = Connection::open_in_memory().expect("create db");
        migrate(&conn).expect("migrate");

        assert!(is_up_to_date(&conn));

        let rolled_back = rollback_to(&conn, 0).expect("rollback");
        assert!(!rolled_back.is_empty());

        // Tables should be gone
        let exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='workspaces'",
                [],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(exists, 0, "workspaces table should be dropped");
    }
}
