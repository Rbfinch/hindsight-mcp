# hindsight-mcp Architecture

An MCP server for AI-assisted coding that leverages development history.

## Overview

hindsight-mcp consolidates various "development data" stored locally (git logs, test results, and GitHub Copilot logs) into a well-structured, searchable SQLite database, making it accessible to an LLM through MCP tool calls within VS Code.

## Workspace Structure

```
hindsight/
├── Cargo.toml                          # Workspace manifest with shared dependencies
├── .gitignore                          # Rust + macOS gitignore
├── README.md                           # Project documentation
└── crates/
    ├── hindsight-mcp/                  # Binary crate - MCP server
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── main.rs
    │   │   ├── db.rs
    │   │   └── server.rs
    │   ├── benches/mcp_bench.rs
    │   ├── tests/integration_tests.rs
    │   └── fuzz/                       # Fuzz testing setup
    │
    ├── hindsight-git/                  # Library - Git log processing
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── commit.rs
    │   │   ├── error.rs
    │   │   └── parser.rs
    │   ├── benches/git_bench.rs
    │   ├── tests/integration_tests.rs
    │   └── fuzz/
    │
    ├── hindsight-tests/                # Library - Test result processing
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── error.rs
    │   │   ├── nextest.rs
    │   │   └── result.rs
    │   ├── benches/tests_bench.rs
    │   ├── tests/integration_tests.rs
    │   └── fuzz/
    │
    └── hindsight-copilot/              # Library - Copilot log processing
        ├── Cargo.toml
        ├── src/
        │   ├── lib.rs
        │   ├── error.rs
        │   ├── lsp.rs
        │   ├── parser.rs
        │   └── session.rs
        ├── benches/copilot_bench.rs
        ├── tests/integration_tests.rs
        └── fuzz/
```

## Crates

### hindsight-mcp (binary)

The main MCP server that bridges AI and development history.

- **Purpose**: Exposes development history data via MCP tool calls
- **Key Dependencies**: `rust-mcp-sdk`, `rusqlite`
- **Responsibilities**:
  - Initialize and manage SQLite database
  - Start MCP server and register tools
  - Orchestrate data from library crates

### hindsight-git (library)

Processes git logs for consumption by hindsight-mcp.

- **Purpose**: Parse and extract meaningful data from git history
- **Key Dependencies**: `git2`
- **Responsibilities**:
  - Parse git commits, diffs, and history
  - Extract author, timestamp, and message information
  - Provide structured commit data

### hindsight-tests (library)

Processes test logs, particularly from cargo-nextest.

- **Purpose**: Parse and structure test execution results
- **Key Dependencies**: `nextest-metadata`
- **Responsibilities**:
  - Parse nextest JSON output
  - Extract test names, outcomes, and durations
  - Track test history over time

### hindsight-copilot (library)

Processes GitHub Copilot logs and chat sessions.

- **Purpose**: Extract and structure Copilot interaction history
- **Key Dependencies**: `serde_json`, `lsp-types`, `tracing-subscriber`
- **Responsibilities**:
  - Parse JSON Stream / LSP Trace formatted logs
  - Extract chat sessions and messages
  - Handle workspace-specific session storage

#### Copilot Log Locations

VS Code stores Copilot chat history in local SQLite databases and JSON files:

| Platform | Path |
|----------|------|
| macOS    | `~/Library/Application Support/Code/User/workspaceStorage/<workspace-id>/chatSessions/` |
| Windows  | `%APPDATA%\Code\User\workspaceStorage\<workspace-id>\chatSessions\` |
| Linux    | `~/.config/Code/User/workspaceStorage/<workspace-id>/chatSessions/` |

> **Note**: Sessions are tied to specific workspaces. Moving a project to a new directory changes the workspace ID, which may make history appear "lost".

## Data Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  hindsight-git  │     │ hindsight-tests │     │hindsight-copilot│
│                 │     │                 │     │                 │
│   Git History   │     │  Test Results   │     │ Copilot Sessions│
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │     hindsight-mcp      │
                    │                        │
                    │  ┌──────────────────┐  │
                    │  │  SQLite Database │  │
                    │  └──────────────────┘  │
                    │                        │
                    │  ┌──────────────────┐  │
                    │  │    MCP Server    │  │
                    │  └──────────────────┘  │
                    └────────────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │    VS Code / LLM       │
                    │   (MCP Tool Calls)     │
                    └────────────────────────┘
```

## Data Ingestion

The ingestion pipeline populates the database from all supported data sources using a unified API.

### Ingestion Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        DATA SOURCES                                  │
├─────────────────┬─────────────────────┬─────────────────────────────┤
│   Git Repo      │   Nextest Output    │   VS Code Storage           │
│   (via git2)    │   (JSON stream)     │   (chatSessions/)           │
└────────┬────────┴──────────┬──────────┴─────────────┬───────────────┘
         │                   │                        │
         ▼                   ▼                        ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────────┐
│ hindsight-git   │ │ hindsight-tests │ │     hindsight-copilot       │
│                 │ │                 │ │                             │
│ parser.rs       │ │ nextest.rs      │ │ session.rs                  │
│ - walk commits  │ │ - parse JSON    │ │ - discover sessions         │
│ - extract diffs │ │ - extract runs  │ │ - parse messages            │
│ -> Vec<Commit>  │ │ -> TestSummary  │ │ -> Vec<ChatSession>         │
└────────┬────────┘ └────────┬────────┘ └─────────────┬───────────────┘
         │                   │                        │
         └───────────────────┼────────────────────────┘
                             │
                             ▼
                ┌────────────────────────┐
                │     hindsight-mcp      │
                │                        │
                │ ┌────────────────────┐ │
                │ │   ingest.rs        │ │
                │ │   - Ingestor       │ │
                │ │   - ingest_git()   │ │
                │ │   - ingest_tests() │ │
                │ │   - ingest_copilot │ │
                │ └──────────┬─────────┘ │
                │            │           │
                │ ┌──────────▼─────────┐ │
                │ │      db.rs         │ │
                │ │   - insert_*()     │ │
                │ │   - batch ops      │ │
                │ └──────────┬─────────┘ │
                │            │           │
                │ ┌──────────▼─────────┐ │
                │ │  SQLite Database   │ │
                │ │  - 7 tables        │ │
                │ │  - FTS5 indexes    │ │
                │ │  - 3 views         │ │
                │ └────────────────────┘ │
                └────────────────────────┘
```

### Usage Example

```rust
use hindsight_mcp::db::Database;
use hindsight_mcp::ingest::{Ingestor, IngestOptions};

// Create and initialize database
let mut db = Database::open("/path/to/hindsight.db")?;
db.initialize()?;

// Create ingestor with optional progress callback
let mut ingestor = Ingestor::new(db)
    .with_progress(Box::new(|event| {
        println!("Progress: {:?}", event);
    }));

// Ingest git commits (full or incremental)
let git_options = IngestOptions::full().with_limit(100);
let git_stats = ingestor.ingest_git("/path/to/repo", &git_options)?;
println!("Ingested {} commits", git_stats.commits_inserted);

// Ingest test results from nextest output
let nextest_json = std::fs::read_to_string("nextest-output.json")?;
let test_stats = ingestor.ingest_tests("/path/to/repo", &nextest_json, None)?;
println!("Ingested {} test results", test_stats.test_results_inserted);

// Ingest Copilot sessions
let copilot_stats = ingestor.ingest_copilot("/path/to/repo")?;
println!("Ingested {} sessions", copilot_stats.sessions_inserted);
```

### Ingestion Options

| Option | Description | Default |
|--------|-------------|---------|
| `commit_limit` | Maximum commits to ingest | None (all) |
| `include_diffs` | Include diff information | true |
| `incremental` | Skip already-ingested commits | false |

**Constructors**:
- `IngestOptions::default()` - All defaults (no limit, no diffs, not incremental)
- `IngestOptions::full()` - Full ingestion with diffs
- `IngestOptions::incremental()` - Incremental sync with diffs

### Progress Reporting

The `Ingestor` supports optional progress callbacks:

```rust
pub enum ProgressEvent {
    Started { source: String, total_items: Option<usize> },
    Progress { source: String, processed: usize, total: Option<usize> },
    Warning { source: String, message: String },
    Completed { source: String, stats: IngestStats },
}
```

### Performance Characteristics

Based on testing with real repository data:

| Operation | Typical Performance |
|-----------|---------------------|
| Git ingestion (100 commits with diffs) | < 5 seconds |
| Timeline query (50 events) | < 10ms |
| FTS5 search | < 10ms |
| Activity summary | < 10ms |

## SQLite Schema

The database uses SQLite with FTS5 full-text search, JSON1 functions, and foreign key constraints.

### Entity Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              WORKSPACES                                     │
│  (Central entity - all other tables reference this)                         │
└─────────────────────────────────────────────────────────────────────────────┘
        │                           │                           │
        │ 1:N                       │ 1:N                       │ 1:N
        ▼                           ▼                           ▼
┌───────────────────┐     ┌───────────────────┐     ┌───────────────────────┐
│     COMMITS       │     │    TEST_RUNS      │     │   COPILOT_SESSIONS    │
│ (Git history)     │     │ (Nextest runs)    │     │ (Chat conversations)  │
└───────────────────┘     └───────────────────┘     └───────────────────────┘
        │                           │                           │
        │                           │ 1:N                       │ 1:N
        │                           ▼                           ▼
        │                 ┌───────────────────┐     ┌───────────────────────┐
        │                 │   TEST_RESULTS    │     │   COPILOT_MESSAGES    │
        │                 │ (Individual tests)│     │ (User/assistant msgs) │
        │                 └───────────────────┘     └───────────────────────┘
        │                                                       │
        ▼                                                       ▼
┌───────────────────┐                               ┌───────────────────────┐
│   COMMITS_FTS     │                               │ COPILOT_MESSAGES_FTS  │
│ (Full-text index) │                               │ (Full-text index)     │
└───────────────────┘                               └───────────────────────┘
```

### Tables

#### workspaces

The root entity representing a monitored development workspace/repository.

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT (UUID) | Primary key |
| `name` | TEXT | Human-readable workspace name |
| `path` | TEXT | Absolute filesystem path (unique) |
| `created_at` | TEXT | ISO 8601 timestamp |
| `updated_at` | TEXT | ISO 8601 timestamp |

#### commits

Stores parsed git commit data with optional diff information.

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT (UUID) | Primary key |
| `workspace_id` | TEXT | FK → workspaces(id) |
| `sha` | TEXT | 40-character hex SHA |
| `author` | TEXT | Commit author name |
| `author_email` | TEXT | Author email (optional) |
| `message` | TEXT | Full commit message |
| `timestamp` | TEXT | Commit timestamp (ISO 8601) |
| `parents_json` | TEXT | JSON array of parent SHAs |
| `diff_json` | TEXT | JSON object with file changes |
| `created_at` | TEXT | Record creation (ISO 8601) |

**Unique Constraint**: `(workspace_id, sha)`

#### test_runs

Represents one execution of `cargo nextest run`.

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT (UUID) | Primary key |
| `workspace_id` | TEXT | FK → workspaces(id) |
| `commit_sha` | TEXT | Git SHA at time of run |
| `started_at` | TEXT | ISO 8601 timestamp |
| `finished_at` | TEXT | ISO 8601 (null if incomplete) |
| `passed_count` | INTEGER | Number of passed tests |
| `failed_count` | INTEGER | Number of failed tests |
| `ignored_count` | INTEGER | Number of ignored tests |
| `metadata_json` | TEXT | Build metadata from nextest |

#### test_results

Individual test case outcomes from a test run.

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT (UUID) | Primary key |
| `run_id` | TEXT | FK → test_runs(id) |
| `suite_name` | TEXT | Crate/binary name |
| `test_name` | TEXT | Full test path (e.g., `module::test_fn`) |
| `outcome` | TEXT | `passed`, `failed`, `ignored`, `timedout` |
| `duration_ms` | INTEGER | Duration in milliseconds |
| `output_json` | TEXT | JSON: `{"stdout": "...", "stderr": "..."}` |
| `created_at` | TEXT | Record creation (ISO 8601) |

#### copilot_sessions

Represents a GitHub Copilot chat conversation.

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT (UUID) | Primary key |
| `workspace_id` | TEXT | FK → workspaces(id) |
| `vscode_session_id` | TEXT | Original VS Code session ID |
| `created_at` | TEXT | ISO 8601 timestamp |
| `updated_at` | TEXT | ISO 8601 timestamp |
| `metadata_json` | TEXT | JSON: version, responder, etc. |

**Unique Constraint**: `(workspace_id, vscode_session_id)`

#### copilot_messages

Individual messages within a chat session.

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT (UUID) | Primary key |
| `session_id` | TEXT | FK → copilot_sessions(id) |
| `request_id` | TEXT | Original request ID from VS Code |
| `role` | TEXT | `user`, `assistant`, `system` |
| `content` | TEXT | Message content |
| `variables_json` | TEXT | JSON: attached files, selections |
| `timestamp` | TEXT | Message timestamp (ISO 8601) |
| `created_at` | TEXT | Record creation (ISO 8601) |

### JSON Column Structures

#### commits.parents_json

```json
["abc123def456...", "789012abc345..."]
```

#### commits.diff_json

```json
{
  "files_changed": 3,
  "insertions": 42,
  "deletions": 7,
  "files": [
    {"path": "src/lib.rs", "status": "modified", "insertions": 10, "deletions": 2}
  ]
}
```

#### test_results.output_json

```json
{
  "stdout": "test output...",
  "stderr": "error messages..."
}
```

#### copilot_messages.variables_json

```json
{
  "files": [{"uri": "file:///path/to/file.rs", "lines": "10-20"}],
  "selections": [{"text": "selected code", "uri": "file:///path/to/file.rs"}]
}
```

### Indexes

| Table | Index | Columns | Purpose |
|-------|-------|---------|---------|
| workspaces | `idx_workspaces_path` | `path` | Fast workspace lookup by path |
| commits | `idx_commits_workspace` | `workspace_id` | Filter commits by workspace |
| commits | `idx_commits_timestamp` | `timestamp` | Date range queries |
| commits | `idx_commits_sha` | `sha` | SHA lookup |
| commits | `idx_commits_author` | `author` | Author filtering |
| test_runs | `idx_test_runs_workspace` | `workspace_id` | Filter runs by workspace |
| test_runs | `idx_test_runs_started` | `started_at` | Date range queries |
| test_runs | `idx_test_runs_commit` | `commit_sha` | Link runs to commits |
| test_results | `idx_test_results_run` | `run_id` | Get results for a run |
| test_results | `idx_test_results_outcome` | `outcome` | Filter by pass/fail |
| test_results | `idx_test_results_suite` | `suite_name` | Filter by crate |
| test_results | `idx_test_results_name` | `test_name` | Find specific tests |
| copilot_sessions | `idx_copilot_sessions_workspace` | `workspace_id` | Filter sessions |
| copilot_sessions | `idx_copilot_sessions_created` | `created_at` | Date range queries |
| copilot_messages | `idx_copilot_messages_session` | `session_id` | Get messages for session |
| copilot_messages | `idx_copilot_messages_timestamp` | `timestamp` | Date range queries |
| copilot_messages | `idx_copilot_messages_role` | `role` | Filter by role |

### Full-Text Search (FTS5)

Two FTS5 virtual tables provide fast text search:

| Table | Source | Indexed Column | Purpose |
|-------|--------|----------------|---------|
| `commits_fts` | `commits` | `message` | Search commit messages |
| `copilot_messages_fts` | `copilot_messages` | `content` | Search Copilot chat |

FTS tables use external content (no data duplication) and are kept synchronized via triggers.

**Search Examples**:

```sql
-- Search commit messages
SELECT c.* FROM commits c
JOIN commits_fts ON c.rowid = commits_fts.rowid
WHERE commits_fts MATCH 'refactor error handling';

-- Search Copilot conversations
SELECT cm.* FROM copilot_messages cm
JOIN copilot_messages_fts ON cm.rowid = copilot_messages_fts.rowid
WHERE copilot_messages_fts MATCH 'async await';
```

### Views

#### timeline

Unified view across all data sources for chronological analysis.

```sql
SELECT event_type, event_id, workspace_id, event_timestamp, summary, details_json
FROM timeline
WHERE workspace_id = ?
ORDER BY event_timestamp DESC;
```

| Column | Description |
|--------|-------------|
| `event_type` | `commit`, `test_run`, or `copilot_message` |
| `event_id` | UUID of the source record |
| `workspace_id` | Workspace reference |
| `event_timestamp` | When the event occurred |
| `summary` | Human-readable summary (message, test counts, or truncated content) |
| `details_json` | Type-specific details |

#### failing_tests

Convenient view of failed tests with run context.

```sql
SELECT test_name, suite_name, full_name, duration_ms, run_id, commit_sha, started_at
FROM failing_tests
WHERE commit_sha = ?;
```

#### recent_activity

Aggregate counts by workspace and event type.

```sql
SELECT workspace_id, event_type, event_count, latest_timestamp
FROM recent_activity;
```

### Sample Queries

#### Query commits by date range

```sql
SELECT sha, author, message, timestamp
FROM commits
WHERE workspace_id = ?
  AND timestamp BETWEEN '2026-01-01T00:00:00Z' AND '2026-01-31T23:59:59Z'
ORDER BY timestamp DESC;
```

#### Join commits with test results

```sql
SELECT c.sha, c.message, tr.passed_count, tr.failed_count, tr.started_at
FROM commits c
JOIN test_runs tr ON tr.commit_sha = c.sha AND tr.workspace_id = c.workspace_id
WHERE c.workspace_id = ?
ORDER BY c.timestamp DESC;
```

#### Find failing tests for a commit

```sql
SELECT t.suite_name, t.test_name, t.duration_ms, t.output_json
FROM test_results t
JOIN test_runs r ON t.run_id = r.id
WHERE r.commit_sha = ?
  AND t.outcome = 'failed';
```

#### Search Copilot sessions by content

```sql
SELECT cs.id AS session_id, cm.role, cm.content, cm.timestamp
FROM copilot_sessions cs
JOIN copilot_messages cm ON cm.session_id = cs.id
JOIN copilot_messages_fts fts ON cm.rowid = fts.rowid
WHERE cs.workspace_id = ?
  AND fts MATCH 'async error handling'
ORDER BY cm.timestamp;
```

#### Timeline view with JSON extraction

```sql
SELECT 
    event_type,
    event_timestamp,
    summary,
    json_extract(details_json, '$.sha') AS commit_sha,
    json_extract(details_json, '$.author') AS author
FROM timeline
WHERE workspace_id = ?
  AND event_type = 'commit'
ORDER BY event_timestamp DESC
LIMIT 50;
```

### Performance Notes

1. **Index Usage**: All foreign keys and commonly filtered columns are indexed
2. **FTS5 External Content**: Uses triggers for synchronization, avoiding data duplication
3. **Query Planning**: Use `EXPLAIN QUERY PLAN` to verify index usage:

   ```sql
   EXPLAIN QUERY PLAN
   SELECT * FROM commits WHERE workspace_id = ? AND timestamp > ?;
   -- Expected: SEARCH commits USING INDEX idx_commits_workspace
   ```

4. **JSON Functions**: Use `json_extract()` for filtering on JSON columns:

   ```sql
   SELECT * FROM test_results
   WHERE json_extract(output_json, '$.stderr') LIKE '%error%';
   ```

## Development

### Building

```bash
cargo build --workspace
```

### Testing

```bash
cargo nextest run --workspace
```

### Benchmarks

```bash
cargo bench --workspace
```

### Fuzzing

Each crate has a `fuzz/` directory with fuzz targets. To run:

```bash
cd crates/<crate-name>
cargo +nightly fuzz run <target-name>
```

## Key Design Decisions

1. **Workspace-level dependencies**: All shared dependencies are defined in the root `Cargo.toml` for consistency
2. **Separate library crates**: Each data source (git, tests, copilot) is isolated for maintainability and testing
3. **SQLite storage**: Provides durable, queryable storage with good performance for local use
4. **MCP protocol**: Standard protocol for LLM tool integration, works seamlessly with VS Code
