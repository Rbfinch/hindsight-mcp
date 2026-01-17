# MILESTONE: Data Exploration and SQLite Schema Design

**Status**: 🔄 IN PROGRESS
**Priority**: 🟡 HIGH
**Created**: 2026-01-17T02:33:06Z
**Estimated Duration**: 4-5 sessions

---

## Executive Summary

**Objective**: Explore the content of test logs, git logs, and GitHub Copilot logs for the hindsight workspace, then propose a well-structured SQLite schema to store this data with full SQL join capabilities.

**Current State**: The hindsight-mcp workspace has scaffold crates with basic type definitions but minimal implementation. No tests exist, no database schema is defined.

**The Problem**: We need to understand the structure and content of development history data sources before designing a database schema that can effectively store and query this information.

**The Solution**: 
1. Write comprehensive tests (unit, property, integration) to generate real test logs
2. Analyze git log structure from the current repository
3. Examine Copilot chat session JSON format
4. Design a normalized SQLite schema with UUIDs and ISO 8601 timestamps
5. Prefer JSON storage for complex nested data

---

## Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| Unit tests written | ≥15 across all crates | ⏳ Pending |
| Property tests written | ≥5 using proptest | ⏳ Pending |
| Integration tests written | ≥4 (one per crate) | ⏳ Pending |
| Test logs generated | JSON format from nextest | ⏳ Pending |
| Git log format documented | Complete schema mapping | ⏳ Pending |
| Copilot log format documented | Complete schema mapping | ⏳ Pending |
| SQLite schema designed | All tables, indexes, FKs | ⏳ Pending |
| Schema supports SQL joins | Cross-table queries work | ⏳ Pending |

---

## Data Sources Analysis

### 1. Git Logs (hindsight-git)

**Source**: Git repository via `git2` crate

**Format**: Structured data from git objects

```json
{
  "sha": "1945ab9c752534e733c38ba0109dc3b741f0a6eb",
  "author": "Nicholas Crosbie",
  "author_email": "nicholascrosbie@Mac.lan",
  "timestamp": "2026-01-17T13:14:23+11:00",
  "parents": ["c460aeb7fb2d109c17e43de0ce681faec0b7374d"],
  "message": "feat(skills): add milestone-creator and rust-patterns skills"
}
```

**Additional data to capture**:
- File diffs (additions, deletions, file paths)
- Branch information
- Tags
- Merge commit detection

### 2. Test Logs (hindsight-tests)

**Source**: cargo-nextest JSON output via `nextest-metadata` crate

**Format**: Structured JSON from `cargo nextest run --message-format json`

```json
{
  "test_count": 25,
  "rust_suites": {
    "hindsight-git": {
      "test_cases": {
        "tests::test_commit_parsing": {
          "outcome": "pass",
          "duration_ms": 12
        }
      }
    }
  }
}
```

**Additional data to capture**:
- Test suite (crate) name
- Test case name (full path)
- Outcome (pass/fail/ignored/timeout)
- Duration
- stdout/stderr output
- Run timestamp
- Build metadata

### 3. Copilot Logs (hindsight-copilot)

**Source**: VS Code workspace storage (JSON files and SQLite DBs)

**Location**: `~/Library/Application Support/Code/User/workspaceStorage/<workspace-id>/chatSessions/`

**Format**: JSON chat sessions

```json
{
  "version": 3,
  "responderUsername": "GitHub Copilot",
  "initialLocation": "panel",
  "requests": [
    {
      "requestId": "request_2eef2349-1624-4da0-9078-ab340674c658",
      "message": {
        "parts": [{ "text": "User prompt...", "kind": "text" }],
        "text": "Full message text"
      },
      "variableData": {
        "variables": [
          { "kind": "file", "name": "filename.rs", "id": "file://..." }
        ]
      }
    }
  ]
}
```

**Additional data to capture**:
- Session ID (from filename)
- Workspace ID
- Message roles (user/assistant)
- Message timestamps
- Referenced files
- Code suggestions
- Agent used (@workspace, etc.)

---

## Phase Breakdown

### Phase 0: Write Unit Tests (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Create comprehensive unit tests for existing types and functions

#### Tasks

1. **hindsight-git unit tests** (~80 lines) ✅
   - Test `Commit` struct serialization/deserialization
   - Test commit SHA validation
   - Test timestamp parsing
   - Test parent SHA handling

2. **hindsight-tests unit tests** (~80 lines) ✅
   - Test `TestResult` struct serialization
   - Test `TestOutcome` enum variants
   - Test duration handling
   - Test output capture

3. **hindsight-copilot unit tests** (~100 lines) ✅
   - Test `ChatSession` serialization
   - Test `ChatMessage` struct
   - Test `MessageRole` enum
   - Test `LspMessage` parsing
   - Test `default_chat_sessions_dir()` platform logic

4. **hindsight-mcp unit tests** (~40 lines) ✅
   - Test database connection setup
   - Test basic SQLite operations

#### Deliverables

- `crates/hindsight-git/src/commit.rs` - Unit tests added ✅
- `crates/hindsight-tests/src/result.rs` - Unit tests added ✅
- `crates/hindsight-copilot/src/session.rs` - Unit tests added ✅
- `crates/hindsight-copilot/src/lsp.rs` - Unit tests added ✅
- `crates/hindsight-copilot/src/parser.rs` - Unit tests added ✅
- `crates/hindsight-mcp/src/db.rs` - Unit tests added + SQLite schema ✅

#### Validation Gate

```bash
cargo nextest run --workspace
cargo fmt --all --check
cargo clippy --workspace
```

#### Success Criteria

- [x] All unit tests pass (71 tests)
- [x] ≥15 unit tests across crates (67 unit tests + 4 integration placeholders)
- [x] Tests cover serialization/deserialization
- [x] Tests validate edge cases

**Commit**: `test(all): add unit tests for core types`

---

### Phase 1: Write Property Tests (1 session)

**Status**: ⏳ not-started
**Goal**: Add property-based tests using proptest for robustness
**Dependencies**: Phase 0

#### Tasks

1. **Add proptest to crate dependencies** (~10 lines)
   - Update `Cargo.toml` for each crate
   - Add proptest strategies for custom types

2. **hindsight-git property tests** (~60 lines)
   - Arbitrary `Commit` generation
   - Round-trip serialization property
   - SHA format property (40 hex chars)

3. **hindsight-tests property tests** (~50 lines)
   - Arbitrary `TestResult` generation
   - Round-trip serialization property
   - Duration always positive property

4. **hindsight-copilot property tests** (~50 lines)
   - Arbitrary `ChatMessage` generation
   - Round-trip serialization property
   - Message role exhaustiveness

#### Deliverables

- `crates/hindsight-git/Cargo.toml` - proptest dependency
- `crates/hindsight-git/src/commit.rs` - Property tests module
- `crates/hindsight-tests/Cargo.toml` - proptest dependency
- `crates/hindsight-tests/src/result.rs` - Property tests module
- `crates/hindsight-copilot/Cargo.toml` - proptest dependency
- `crates/hindsight-copilot/src/session.rs` - Property tests module

#### Validation Gate

```bash
cargo nextest run --workspace
# Property tests run 256 cases by default
```

#### Success Criteria

- [ ] All property tests pass
- [ ] ≥5 property tests across crates
- [ ] Round-trip properties verified
- [ ] Edge cases discovered and handled

**Commit**: `test(all): add property-based tests with proptest`

---

### Phase 2: Write Integration Tests (1 session)

**Status**: ⏳ not-started
**Goal**: Create integration tests that exercise real data parsing
**Dependencies**: Phase 1

#### Tasks

1. **hindsight-git integration tests** (~60 lines)
   - Parse commits from actual git repository
   - Test with current workspace's git history
   - Verify commit chain traversal

2. **hindsight-tests integration tests** (~60 lines)
   - Parse actual nextest JSON output
   - Test with sample nextest output files
   - Verify test suite discovery

3. **hindsight-copilot integration tests** (~80 lines)
   - Parse actual Copilot chat session JSON
   - Test session discovery from workspace storage
   - Verify message extraction

4. **hindsight-mcp integration tests** (~60 lines)
   - Test full database round-trip
   - Test cross-crate data flow
   - Test MCP tool registration

#### Deliverables

- `crates/hindsight-git/tests/integration_tests.rs` - Real git parsing
- `crates/hindsight-tests/tests/integration_tests.rs` - Nextest parsing
- `crates/hindsight-copilot/tests/integration_tests.rs` - Session parsing
- `crates/hindsight-mcp/tests/integration_tests.rs` - Database tests
- `crates/hindsight-tests/tests/fixtures/` - Sample nextest output

#### Validation Gate

```bash
cargo nextest run --workspace
cargo nextest run --workspace --message-format json > target/nextest-output.json
```

#### Success Criteria

- [ ] All integration tests pass
- [ ] Tests use real data sources where possible
- [ ] Fixtures created for reproducible tests
- [ ] Cross-crate dependencies verified

**Commit**: `test(all): add integration tests with real data`

---

### Phase 3: Design SQLite Schema (1 session)

**Status**: ⏳ not-started
**Goal**: Create a normalized SQLite schema for all data sources
**Dependencies**: Phase 2

#### Tasks

1. **Core schema tables** (~100 lines)
   - `workspaces` - Track monitored workspaces
   - `commits` - Git commits with JSON diff data
   - `test_runs` - Test execution sessions
   - `test_results` - Individual test outcomes
   - `copilot_sessions` - Chat sessions
   - `copilot_messages` - Individual messages

2. **Indexing strategy** (~30 lines)
   - Primary keys using UUIDs
   - Foreign key relationships
   - Timestamp indexes for range queries
   - Full-text search indexes for content

3. **JSON columns design** (~20 lines)
   - `commit.diff_json` - File changes as JSON
   - `test_result.output_json` - stdout/stderr
   - `copilot_message.variables_json` - Attached context

4. **Schema migration system** (~50 lines)
   - Version tracking table
   - Up/down migration scripts
   - Schema validation

#### Deliverables

- `crates/hindsight-mcp/src/db.rs` - Full schema implementation
- `crates/hindsight-mcp/src/schema.sql` - Raw SQL schema file
- `crates/hindsight-mcp/src/migrations/` - Migration scripts

#### Validation Gate

```bash
cargo nextest run -p hindsight-mcp -- db
sqlite3 :memory: < crates/hindsight-mcp/src/schema.sql
```

#### Success Criteria

- [ ] All tables have UUID primary keys
- [ ] All timestamps in ISO 8601 format
- [ ] Foreign key relationships defined
- [ ] JSON columns for complex data
- [ ] Indexes on frequently queried columns
- [ ] Example JOIN queries documented

**Commit**: `feat(db): implement SQLite schema for development history`

---

### Phase 4: Document and Validate Schema (0.5 session)

**Status**: ⏳ not-started
**Goal**: Document the schema and validate with sample queries
**Dependencies**: Phase 3

#### Tasks

1. **Schema documentation** (~80 lines)
   - Table descriptions
   - Column definitions with types
   - Relationship diagrams
   - JSON column structures

2. **Sample queries** (~60 lines)
   - Query commits by date range
   - Join commits with test results
   - Find failing tests for a commit
   - Search Copilot sessions by content
   - Timeline view across all sources

3. **Performance validation** (~20 lines)
   - Index usage verification
   - Query plan analysis
   - Benchmark critical queries

#### Deliverables

- `ARCHITECTURE.md` - Updated with schema documentation
- `crates/hindsight-mcp/src/queries.rs` - Common query implementations
- `development/milestones/` - Phase completion updates

#### Validation Gate

```bash
cargo nextest run --workspace
cargo doc --workspace --no-deps
```

#### Success Criteria

- [ ] Schema fully documented
- [ ] ≥5 example JOIN queries
- [ ] Query performance acceptable
- [ ] ARCHITECTURE.md updated

**Commit**: `docs(schema): add SQLite schema documentation and examples`

---

## Proposed SQLite Schema

### Entity Relationship Diagram

```
┌─────────────────┐       ┌─────────────────┐
│   workspaces    │       │     commits     │
├─────────────────┤       ├─────────────────┤
│ id (UUID) PK    │──┐    │ id (UUID) PK    │
│ name            │  │    │ workspace_id FK │──┐
│ path            │  │    │ sha             │  │
│ created_at      │  │    │ author          │  │
│ updated_at      │  │    │ author_email    │  │
└─────────────────┘  │    │ message         │  │
                     │    │ timestamp       │  │
                     │    │ parents_json    │  │
                     │    │ diff_json       │  │
                     │    │ created_at      │  │
                     │    └─────────────────┘  │
                     │                         │
                     │    ┌─────────────────┐  │
                     │    │   test_runs     │  │
                     │    ├─────────────────┤  │
                     └────│ id (UUID) PK    │  │
                          │ workspace_id FK │──┘
                          │ commit_sha      │
                          │ started_at      │
                          │ finished_at     │
                          │ passed_count    │
                          │ failed_count    │
                          │ ignored_count   │
                          │ metadata_json   │
                          └────────┬────────┘
                                   │
                          ┌────────▼────────┐
                          │  test_results   │
                          ├─────────────────┤
                          │ id (UUID) PK    │
                          │ run_id FK       │
                          │ suite_name      │
                          │ test_name       │
                          │ outcome         │
                          │ duration_ms     │
                          │ output_json     │
                          │ created_at      │
                          └─────────────────┘

┌─────────────────┐       ┌─────────────────┐
│copilot_sessions │       │copilot_messages │
├─────────────────┤       ├─────────────────┤
│ id (UUID) PK    │──┐    │ id (UUID) PK    │
│ workspace_id FK │  │    │ session_id FK   │──┘
│ vs_code_id      │  │    │ request_id      │
│ created_at      │  │    │ role            │
│ updated_at      │  │    │ content         │
│ metadata_json   │  │    │ variables_json  │
└─────────────────┘  │    │ timestamp       │
                     │    │ created_at      │
                     │    └─────────────────┘
                     │
                     └──── (joins to workspaces)
```

### SQL Schema Definition

```sql
-- Workspaces table (root entity)
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,  -- UUID
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,  -- ISO 8601
    updated_at TEXT NOT NULL   -- ISO 8601
);

-- Git commits
CREATE TABLE commits (
    id TEXT PRIMARY KEY,  -- UUID
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    sha TEXT NOT NULL,
    author TEXT NOT NULL,
    author_email TEXT,
    message TEXT NOT NULL,
    timestamp TEXT NOT NULL,  -- ISO 8601
    parents_json TEXT,  -- JSON array of parent SHAs
    diff_json TEXT,     -- JSON object with file changes
    created_at TEXT NOT NULL,
    UNIQUE(workspace_id, sha)
);

-- Test runs (single nextest execution)
CREATE TABLE test_runs (
    id TEXT PRIMARY KEY,  -- UUID
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    commit_sha TEXT,
    started_at TEXT NOT NULL,   -- ISO 8601
    finished_at TEXT,           -- ISO 8601
    passed_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    ignored_count INTEGER NOT NULL DEFAULT 0,
    metadata_json TEXT  -- Build metadata from nextest
);

-- Individual test results
CREATE TABLE test_results (
    id TEXT PRIMARY KEY,  -- UUID
    run_id TEXT NOT NULL REFERENCES test_runs(id),
    suite_name TEXT NOT NULL,  -- Crate name
    test_name TEXT NOT NULL,   -- Full test path
    outcome TEXT NOT NULL,     -- pass/fail/ignored/timeout
    duration_ms INTEGER,
    output_json TEXT,  -- stdout/stderr as JSON
    created_at TEXT NOT NULL
);

-- Copilot chat sessions
CREATE TABLE copilot_sessions (
    id TEXT PRIMARY KEY,  -- UUID
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    vscode_session_id TEXT NOT NULL,  -- Original VS Code session ID
    created_at TEXT NOT NULL,  -- ISO 8601
    updated_at TEXT NOT NULL,
    metadata_json TEXT,  -- Version, responder info, etc.
    UNIQUE(workspace_id, vscode_session_id)
);

-- Copilot messages
CREATE TABLE copilot_messages (
    id TEXT PRIMARY KEY,  -- UUID
    session_id TEXT NOT NULL REFERENCES copilot_sessions(id),
    request_id TEXT,  -- Original request ID
    role TEXT NOT NULL,  -- user/assistant/system
    content TEXT NOT NULL,
    variables_json TEXT,  -- Attached files, selections, etc.
    timestamp TEXT NOT NULL,  -- ISO 8601
    created_at TEXT NOT NULL
);

-- Indexes for common queries
CREATE INDEX idx_commits_workspace ON commits(workspace_id);
CREATE INDEX idx_commits_timestamp ON commits(timestamp);
CREATE INDEX idx_commits_sha ON commits(sha);
CREATE INDEX idx_test_runs_workspace ON test_runs(workspace_id);
CREATE INDEX idx_test_runs_started ON test_runs(started_at);
CREATE INDEX idx_test_results_run ON test_results(run_id);
CREATE INDEX idx_test_results_outcome ON test_results(outcome);
CREATE INDEX idx_copilot_sessions_workspace ON copilot_sessions(workspace_id);
CREATE INDEX idx_copilot_messages_session ON copilot_messages(session_id);
CREATE INDEX idx_copilot_messages_timestamp ON copilot_messages(timestamp);

-- Full-text search for content
CREATE VIRTUAL TABLE commits_fts USING fts5(message, content='commits', content_rowid='rowid');
CREATE VIRTUAL TABLE copilot_messages_fts USING fts5(content, content='copilot_messages', content_rowid='rowid');
```

### Example JOIN Queries

```sql
-- Find all test results for a specific commit
SELECT c.sha, c.message, tr.test_name, tr.outcome, tr.duration_ms
FROM commits c
JOIN test_runs r ON r.commit_sha = c.sha
JOIN test_results tr ON tr.run_id = r.id
WHERE c.sha LIKE 'abc123%';

-- Timeline of all activity for a workspace
SELECT 'commit' as type, timestamp, message as content, sha as ref
FROM commits WHERE workspace_id = ?
UNION ALL
SELECT 'test_run' as type, started_at, 
       printf('Tests: %d passed, %d failed', passed_count, failed_count),
       id
FROM test_runs WHERE workspace_id = ?
UNION ALL
SELECT 'copilot' as type, timestamp, content, session_id
FROM copilot_messages m
JOIN copilot_sessions s ON m.session_id = s.id
WHERE s.workspace_id = ?
ORDER BY timestamp DESC;

-- Find commits with failing tests
SELECT DISTINCT c.sha, c.message, c.timestamp
FROM commits c
JOIN test_runs r ON r.commit_sha = c.sha
WHERE r.failed_count > 0
ORDER BY c.timestamp DESC;

-- Search Copilot conversations
SELECT s.id as session_id, m.content, m.timestamp
FROM copilot_messages m
JOIN copilot_sessions s ON m.session_id = s.id
WHERE copilot_messages_fts MATCH 'error handling';
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Copilot log format changes | Medium | Medium | Abstract parser behind trait |
| Nextest output format changes | Low | Low | Use nextest-metadata crate |
| Large log files cause memory issues | Medium | High | Stream parsing, pagination |
| SQLite FTS5 not available | Low | Medium | Graceful fallback to LIKE |
| Cross-platform path handling | Medium | Medium | Use `std::path::Path` throughout |

---

## Notes

- **JSON preference**: Store complex nested data as JSON TEXT columns for flexibility and searchability
- **UUID format**: Use UUIDv7 (time-ordered) for better index locality
- **Timestamp format**: ISO 8601 with timezone (e.g., `2026-01-17T02:33:06Z`)
- **Indexing strategy**: Index columns used in WHERE, JOIN, and ORDER BY clauses
- **FTS5**: Use SQLite's full-text search for content search across messages and commits
- **proptest configuration**: Use `PROPTEST_CASES=1000` for CI, default 256 for local development

