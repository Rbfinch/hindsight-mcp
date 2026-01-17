-- Hindsight-MCP SQLite Schema
-- Version: 1
-- Description: Schema for storing development history data
-- 
-- This schema provides tables for:
--   - workspaces: Monitored development workspaces
--   - commits: Git commit history with JSON diff data
--   - test_runs: Test execution sessions (nextest)
--   - test_results: Individual test outcomes
--   - copilot_sessions: GitHub Copilot chat sessions
--   - copilot_messages: Individual chat messages
--
-- Design Principles:
--   - All primary keys are UUIDs stored as TEXT
--   - All timestamps are ISO 8601 format (e.g., 2026-01-17T02:33:06Z)
--   - Complex nested data stored as JSON TEXT columns
--   - Foreign key relationships for data integrity
--   - Indexes on commonly queried columns
--   - FTS5 for full-text search on messages and commits
-- Enable foreign key enforcement
PRAGMA foreign_keys = ON;
--------------------------------------------------------------------------------
-- SCHEMA VERSION TRACKING
--------------------------------------------------------------------------------
-- Track schema migrations
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL -- ISO 8601 timestamp
);
--------------------------------------------------------------------------------
-- CORE TABLES
--------------------------------------------------------------------------------
-- Workspaces table (root entity)
-- Represents a monitored development workspace/repository
CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    -- UUID (e.g., 550e8400-e29b-41d4-a716-446655440000)
    name TEXT NOT NULL,
    -- Human-readable workspace name
    path TEXT NOT NULL UNIQUE,
    -- Absolute filesystem path
    created_at TEXT NOT NULL,
    -- ISO 8601 timestamp
    updated_at TEXT NOT NULL -- ISO 8601 timestamp
);
-- Git commits
-- Stores parsed git commit data with optional diff information
CREATE TABLE IF NOT EXISTS commits (
    id TEXT PRIMARY KEY,
    -- UUID
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    -- FK to workspaces
    sha TEXT NOT NULL,
    -- 40-char hex SHA
    author TEXT NOT NULL,
    -- Author name
    author_email TEXT,
    -- Author email
    message TEXT NOT NULL,
    -- Full commit message
    timestamp TEXT NOT NULL,
    -- Commit timestamp (ISO 8601)
    parents_json TEXT,
    -- JSON array: ["sha1", "sha2"]
    diff_json TEXT,
    -- JSON object with file changes
    created_at TEXT NOT NULL,
    -- Record creation (ISO 8601)
    UNIQUE(workspace_id, sha)
);
-- Test runs (single nextest execution)
-- Represents one execution of `cargo nextest run`
CREATE TABLE IF NOT EXISTS test_runs (
    id TEXT PRIMARY KEY,
    -- UUID
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    -- FK to workspaces
    commit_sha TEXT,
    -- Git SHA at time of run
    started_at TEXT NOT NULL,
    -- ISO 8601 timestamp
    finished_at TEXT,
    -- ISO 8601 timestamp (null if incomplete)
    passed_count INTEGER NOT NULL DEFAULT 0,
    -- Number of passed tests
    failed_count INTEGER NOT NULL DEFAULT 0,
    -- Number of failed tests
    ignored_count INTEGER NOT NULL DEFAULT 0,
    -- Number of ignored tests
    metadata_json TEXT -- Build metadata from nextest
);
-- Individual test results
-- Each test case outcome from a test run
CREATE TABLE IF NOT EXISTS test_results (
    id TEXT PRIMARY KEY,
    -- UUID
    run_id TEXT NOT NULL REFERENCES test_runs(id),
    -- FK to test_runs
    suite_name TEXT NOT NULL,
    -- Crate/binary name
    test_name TEXT NOT NULL,
    -- Full test path (e.g., module::test_fn)
    outcome TEXT NOT NULL,
    -- passed/failed/ignored/timedout
    duration_ms INTEGER,
    -- Duration in milliseconds
    output_json TEXT,
    -- JSON: {"stdout": "...", "stderr": "..."}
    created_at TEXT NOT NULL -- Record creation (ISO 8601)
);
-- Copilot chat sessions
-- Represents a GitHub Copilot chat conversation
CREATE TABLE IF NOT EXISTS copilot_sessions (
    id TEXT PRIMARY KEY,
    -- UUID
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    -- FK to workspaces
    vscode_session_id TEXT NOT NULL,
    -- Original VS Code session ID
    created_at TEXT NOT NULL,
    -- ISO 8601 timestamp
    updated_at TEXT NOT NULL,
    -- ISO 8601 timestamp
    metadata_json TEXT,
    -- JSON: version, responder, etc.
    UNIQUE(workspace_id, vscode_session_id)
);
-- Copilot messages
-- Individual messages within a chat session
CREATE TABLE IF NOT EXISTS copilot_messages (
    id TEXT PRIMARY KEY,
    -- UUID
    session_id TEXT NOT NULL REFERENCES copilot_sessions(id),
    -- FK to copilot_sessions
    request_id TEXT,
    -- Original request ID from VS Code
    role TEXT NOT NULL,
    -- user/assistant/system
    content TEXT NOT NULL,
    -- Message content
    variables_json TEXT,
    -- JSON: attached files, selections
    timestamp TEXT NOT NULL,
    -- Message timestamp (ISO 8601)
    created_at TEXT NOT NULL -- Record creation (ISO 8601)
);
--------------------------------------------------------------------------------
-- INDEXES
--------------------------------------------------------------------------------
-- Workspace queries
CREATE INDEX IF NOT EXISTS idx_workspaces_path ON workspaces(path);
-- Commit queries
CREATE INDEX IF NOT EXISTS idx_commits_workspace ON commits(workspace_id);
CREATE INDEX IF NOT EXISTS idx_commits_timestamp ON commits(timestamp);
CREATE INDEX IF NOT EXISTS idx_commits_sha ON commits(sha);
CREATE INDEX IF NOT EXISTS idx_commits_author ON commits(author);
-- Test run queries
CREATE INDEX IF NOT EXISTS idx_test_runs_workspace ON test_runs(workspace_id);
CREATE INDEX IF NOT EXISTS idx_test_runs_started ON test_runs(started_at);
CREATE INDEX IF NOT EXISTS idx_test_runs_commit ON test_runs(commit_sha);
-- Test result queries
CREATE INDEX IF NOT EXISTS idx_test_results_run ON test_results(run_id);
CREATE INDEX IF NOT EXISTS idx_test_results_outcome ON test_results(outcome);
CREATE INDEX IF NOT EXISTS idx_test_results_suite ON test_results(suite_name);
CREATE INDEX IF NOT EXISTS idx_test_results_name ON test_results(test_name);
-- Copilot session queries
CREATE INDEX IF NOT EXISTS idx_copilot_sessions_workspace ON copilot_sessions(workspace_id);
CREATE INDEX IF NOT EXISTS idx_copilot_sessions_created ON copilot_sessions(created_at);
-- Copilot message queries
CREATE INDEX IF NOT EXISTS idx_copilot_messages_session ON copilot_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_copilot_messages_timestamp ON copilot_messages(timestamp);
CREATE INDEX IF NOT EXISTS idx_copilot_messages_role ON copilot_messages(role);
--------------------------------------------------------------------------------
-- FULL-TEXT SEARCH (FTS5)
--------------------------------------------------------------------------------
-- FTS5 table for commit message search
-- Uses external content to avoid data duplication
CREATE VIRTUAL TABLE IF NOT EXISTS commits_fts USING fts5(
    message,
    content = 'commits',
    content_rowid = 'rowid'
);
-- FTS5 table for Copilot message content search
CREATE VIRTUAL TABLE IF NOT EXISTS copilot_messages_fts USING fts5(
    content,
    content = 'copilot_messages',
    content_rowid = 'rowid'
);
--------------------------------------------------------------------------------
-- FTS5 TRIGGERS
-- Keep FTS indexes synchronized with main tables
--------------------------------------------------------------------------------
-- Commits FTS triggers
CREATE TRIGGER IF NOT EXISTS commits_ai
AFTER
INSERT ON commits BEGIN
INSERT INTO commits_fts(rowid, message)
VALUES (new.rowid, new.message);
END;
CREATE TRIGGER IF NOT EXISTS commits_ad
AFTER DELETE ON commits BEGIN
INSERT INTO commits_fts(commits_fts, rowid, message)
VALUES('delete', old.rowid, old.message);
END;
CREATE TRIGGER IF NOT EXISTS commits_au
AFTER
UPDATE ON commits BEGIN
INSERT INTO commits_fts(commits_fts, rowid, message)
VALUES('delete', old.rowid, old.message);
INSERT INTO commits_fts(rowid, message)
VALUES (new.rowid, new.message);
END;
-- Copilot messages FTS triggers
CREATE TRIGGER IF NOT EXISTS copilot_messages_ai
AFTER
INSERT ON copilot_messages BEGIN
INSERT INTO copilot_messages_fts(rowid, content)
VALUES (new.rowid, new.content);
END;
CREATE TRIGGER IF NOT EXISTS copilot_messages_ad
AFTER DELETE ON copilot_messages BEGIN
INSERT INTO copilot_messages_fts(copilot_messages_fts, rowid, content)
VALUES('delete', old.rowid, old.content);
END;
CREATE TRIGGER IF NOT EXISTS copilot_messages_au
AFTER
UPDATE ON copilot_messages BEGIN
INSERT INTO copilot_messages_fts(copilot_messages_fts, rowid, content)
VALUES('delete', old.rowid, old.content);
INSERT INTO copilot_messages_fts(rowid, content)
VALUES (new.rowid, new.content);
END;
--------------------------------------------------------------------------------
-- VIEWS
--------------------------------------------------------------------------------
-- Unified timeline view across all data sources
CREATE VIEW IF NOT EXISTS timeline AS
SELECT 'commit' AS event_type,
    c.id AS event_id,
    c.workspace_id,
    c.timestamp AS event_timestamp,
    c.message AS summary,
    json_object('sha', c.sha, 'author', c.author) AS details_json
FROM commits c
UNION ALL
SELECT 'test_run' AS event_type,
    tr.id AS event_id,
    tr.workspace_id,
    tr.started_at AS event_timestamp,
    printf(
        'Tests: %d passed, %d failed, %d ignored',
        tr.passed_count,
        tr.failed_count,
        tr.ignored_count
    ) AS summary,
    json_object(
        'commit_sha',
        tr.commit_sha,
        'passed',
        tr.passed_count,
        'failed',
        tr.failed_count
    ) AS details_json
FROM test_runs tr
UNION ALL
SELECT 'copilot_message' AS event_type,
    cm.id AS event_id,
    cs.workspace_id,
    cm.timestamp AS event_timestamp,
    substr(cm.content, 1, 100) AS summary,
    json_object('role', cm.role, 'session_id', cm.session_id) AS details_json
FROM copilot_messages cm
    JOIN copilot_sessions cs ON cm.session_id = cs.id;
-- Failing tests view
CREATE VIEW IF NOT EXISTS failing_tests AS
SELECT tr.id AS test_name,
    tr.suite_name,
    tr.test_name AS full_name,
    tr.duration_ms,
    tr.output_json,
    r.id AS run_id,
    r.commit_sha,
    r.started_at
FROM test_results tr
    JOIN test_runs r ON tr.run_id = r.id
WHERE tr.outcome = 'failed';
-- Recent activity summary
CREATE VIEW IF NOT EXISTS recent_activity AS
SELECT workspace_id,
    event_type,
    COUNT(*) AS event_count,
    MAX(event_timestamp) AS latest_timestamp
FROM timeline
GROUP BY workspace_id,
    event_type;
--------------------------------------------------------------------------------
-- INITIAL MIGRATION RECORD
--------------------------------------------------------------------------------
INSERT
    OR IGNORE INTO schema_migrations (version, name, applied_at)
VALUES (1, 'initial_schema', datetime('now'));