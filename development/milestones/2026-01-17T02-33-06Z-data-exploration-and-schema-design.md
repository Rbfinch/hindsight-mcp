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
| Unit tests written | ≥15 across all crates | ✅ 71 tests |
| Property tests written | ≥5 using proptest | ✅ 31 tests |
| Integration tests written | ≥4 (one per crate) | ✅ 24 tests |
| Test logs generated | JSON format from nextest | ✅ Fixtures created |
| Git log format documented | Complete schema mapping | ✅ Explored via git2 |
| Copilot log format documented | Complete schema mapping | ✅ Fixtures created |
| SQLite schema designed | All tables, indexes, FKs | ✅ 302 lines + FTS5 |
| Schema supports SQL joins | Cross-table queries work | ✅ Views + query helpers |
| Migration system | Version tracking | ✅ migrations.rs |

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

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Add property-based tests using proptest for robustness
**Dependencies**: Phase 0

#### Tasks

1. **Add proptest to crate dependencies** ✅
   - proptest already configured as workspace dependency
   - All crates inherit via `proptest.workspace = true`

2. **hindsight-git property tests** (~95 lines) ✅
   - Arbitrary `Commit` generation via `commit_strategy()`
   - SHA format strategy generating valid 40-char hex strings
   - 9 property tests:
     - `prop_commit_sha_is_valid`
     - `prop_commit_roundtrip_serialization`
     - `prop_short_sha_length`
     - `prop_is_merge_iff_multiple_parents`
     - `prop_is_root_iff_no_parents`
     - `prop_subject_is_prefix_of_message`
     - `prop_all_parent_shas_valid`
     - `prop_valid_sha_format`
     - `prop_invalid_sha_wrong_length`

3. **hindsight-tests property tests** (~75 lines) ✅
   - Arbitrary `TestResult` generation via `test_result_strategy()`
   - `TestOutcome` strategy for all variants
   - 8 property tests:
     - `prop_test_result_roundtrip_serialization`
     - `prop_passed_failed_exclusive`
     - `prop_duration_display_format`
     - `prop_test_fn_name_is_suffix`
     - `prop_module_path_plus_fn_equals_name`
     - `prop_outcome_is_success_consistency`
     - `prop_outcome_has_symbol`
     - `prop_outcome_serialization_lowercase`

4. **hindsight-copilot property tests** (~110 lines) ✅
   - Arbitrary `ChatMessage` generation via `message_strategy()`
   - Arbitrary `ChatSession` generation via `session_strategy()`
   - `MessageRole` strategy for all variants
   - 14 property tests:
     - `prop_message_roundtrip_serialization`
     - `prop_session_roundtrip_serialization`
     - `prop_content_len_matches`
     - `prop_has_agent_consistency`
     - `prop_message_count_matches`
     - `prop_is_empty_consistency`
     - `prop_user_messages_role`
     - `prop_assistant_messages_role`
     - `prop_message_filter_counts`
     - `prop_role_serialization_lowercase`
     - `prop_display_name_non_empty`
     - `prop_with_agent_sets_agent`
     - `prop_user_message_has_user_role`
     - `prop_assistant_message_has_assistant_role`

#### Deliverables

- `crates/hindsight-git/src/commit.rs` - Property tests module ✅
- `crates/hindsight-tests/src/result.rs` - Property tests module ✅
- `crates/hindsight-copilot/src/session.rs` - Property tests module ✅

#### Validation Gate

```bash
cargo nextest run --workspace
# Result: 102 tests run: 102 passed, 0 skipped
```

#### Success Criteria

- [x] All property tests pass (31 property tests)
- [x] ≥5 property tests across crates (31 property tests)
- [x] Round-trip properties verified for all serializable types
- [x] Edge cases discovered and handled

**Commit**: `test(all): add property-based tests with proptest`

---

### Phase 2: Write Integration Tests (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Create integration tests that exercise real data parsing
**Dependencies**: Phase 1

#### Tasks

1. **hindsight-git integration tests** (~165 lines) ✅
   - Parse commits from actual git repository via git2
   - Test with current workspace's git history
   - Verify commit chain traversal (5 tests):
     - `test_parse_commits_from_real_repository`
     - `test_commit_chain_traversal`
     - `test_commit_serialization_from_real_data`
     - `test_find_merge_commits`
     - `test_commit_timestamps_are_iso8601`

2. **hindsight-tests integration tests** (~180 lines) ✅
   - Parse actual nextest JSON output
   - Test with sample nextest output files (fixture created)
   - Verify test suite discovery (6 tests):
     - `test_parse_sample_nextest_list_output`
     - `test_create_test_results_from_parsed_data`
     - `test_result_json_serialization`
     - `test_run_actual_nextest_and_parse_output`
     - `test_result_module_path_extraction`
     - `test_duration_display_formatting`

3. **hindsight-copilot integration tests** (~200 lines) ✅
   - Parse actual Copilot chat session JSON (fixture created)
   - Test session discovery from workspace storage
   - Verify message extraction (8 tests):
     - `test_parse_sample_chat_session`
     - `test_extract_messages_from_chat_session`
     - `test_extract_variables_from_requests`
     - `test_chat_session_serialization_roundtrip`
     - `test_message_role_serialization`
     - `test_default_chat_sessions_dir_exists_on_supported_platforms`
     - `test_discover_real_chat_sessions`
     - `test_message_with_agent_extraction`

4. **hindsight-mcp integration tests** (~200 lines) ✅
   - Test full database round-trip (JSON serialization)
   - Test cross-crate data flow
   - Test JSON column structures (10 tests):
     - `test_commit_to_json_for_database`
     - `test_test_result_to_json_for_database`
     - `test_chat_session_to_json_for_database`
     - `test_cross_crate_type_compatibility`
     - `test_timeline_data_structure`
     - `test_uuid_and_timestamp_format`
     - `test_json_column_structures`
     - `test_message_role_as_database_enum`
     - `test_test_outcome_as_database_enum`

#### Deliverables

- `crates/hindsight-git/tests/integration_tests.rs` - Real git parsing ✅
- `crates/hindsight-tests/tests/integration_tests.rs` - Nextest parsing ✅
- `crates/hindsight-copilot/tests/integration_tests.rs` - Session parsing ✅
- `crates/hindsight-mcp/tests/integration_tests.rs` - Database tests ✅
- `crates/hindsight-tests/tests/fixtures/nextest-sample.json` - Sample nextest output ✅
- `crates/hindsight-copilot/tests/fixtures/chat-session-sample.json` - Sample chat session ✅

#### Validation Gate

```bash
cargo nextest run --workspace
# Result: 126 tests run: 126 passed, 0 skipped
```

#### Success Criteria

- [x] All integration tests pass (24 integration tests)
- [x] Tests use real data sources where possible (git repo, nextest)
- [x] Fixtures created for reproducible tests (2 JSON fixtures)
- [x] Cross-crate dependencies verified

**Commit**: `test(all): add integration tests with real data`
- [ ] Fixtures created for reproducible tests
- [ ] Cross-crate dependencies verified

**Commit**: `test(all): add integration tests with real data`

---

### Phase 3: Design SQLite Schema (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Create a normalized SQLite schema for all data sources
**Dependencies**: Phase 2

#### Tasks

1. **Core schema tables** (~302 lines) ✅
   - `workspaces` - Track monitored workspaces
   - `commits` - Git commits with JSON diff data
   - `test_runs` - Test execution sessions
   - `test_results` - Individual test outcomes
   - `copilot_sessions` - Chat sessions
   - `copilot_messages` - Individual messages
   - `schema_migrations` - Version tracking

2. **Indexing strategy** (~20 indexes) ✅
   - Primary keys using TEXT UUIDs
   - Foreign key relationships with ON DELETE CASCADE
   - Timestamp indexes for range queries
   - FTS5 full-text search for commits and messages

3. **JSON columns design** ✅
   - `commit.diff_json` - File changes as JSON
   - `commit.parents_json` - Parent SHA array
   - `test_run.metadata_json` - Build metadata
   - `test_result.output_json` - stdout/stderr
   - `copilot_session.metadata_json` - Session info
   - `copilot_message.variables_json` - Attached context

4. **Schema migration system** (~200 lines) ✅
   - `migrations.rs` - Version tracking and migration functions
   - `get_version()`, `migrate()`, `rollback_to()`, `is_up_to_date()`
   - Migration struct with version, name, up/down SQL
   - MIGRATIONS static array with versioned migrations

5. **Query helper module** (~728 lines) ✅
   - `queries.rs` - High-level query functions
   - `get_timeline()` - Unified activity timeline
   - `search_commits()` / `search_messages()` - FTS5 search
   - `get_failing_tests()` - View-based query
   - `get_activity_summary()` - Aggregate statistics
   - `get_commit_with_tests()` - Join query

6. **Views for common queries** ✅
   - `timeline` - Unified view across commits, tests, copilot
   - `failing_tests` - Failed test results with run info
   - `recent_activity` - Summary by workspace and type

#### Deliverables

- `crates/hindsight-mcp/src/schema.sql` - Comprehensive SQL schema (302 lines) ✅
- `crates/hindsight-mcp/src/migrations.rs` - Migration system (200 lines) ✅
- `crates/hindsight-mcp/src/queries.rs` - Query helpers (728 lines) ✅
- `crates/hindsight-mcp/src/db.rs` - Updated to use migrations ✅

#### Validation Gate

```bash
cargo nextest run -p hindsight-mcp
# Result: 46 tests run: 46 passed, 0 skipped
```

#### Success Criteria

- [x] All tables have TEXT UUID primary keys
- [x] All timestamps in ISO 8601 format
- [x] Foreign key relationships defined
- [x] JSON columns for complex data
- [x] 20+ indexes on frequently queried columns
- [x] FTS5 full-text search with triggers
- [x] Views for common queries
- [x] Migration system with version tracking
- [x] Query helper functions with tests

**Commit**: `feat(db): implement SQLite schema with migrations and FTS5`

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

