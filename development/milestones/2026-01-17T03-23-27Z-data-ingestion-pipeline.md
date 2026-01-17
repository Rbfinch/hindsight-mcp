# MILESTONE: Data Ingestion Pipeline

**Status**: 🔄 IN PROGRESS
**Priority**: 🔴 CRITICAL
**Created**: 2026-01-17T03:23:27Z
**Estimated Duration**: 5-6 sessions

---

## Executive Summary

**Objective**: Implement the data ingestion layer that parses git commits, test results, and Copilot chat sessions, storing them in the SQLite database designed in the previous milestone.

**Current State**: 
- SQLite schema fully designed with 7 tables, 20+ indexes, FTS5, views, and migrations (302 lines)
- Query helper functions implemented in `queries.rs` (728 lines)
- Type definitions exist for `Commit`, `TestResult`, `ChatSession`, `ChatMessage`
- Parser stubs exist but contain no implementation

**The Problem**: The database schema exists but there is no way to populate it. The parser modules (`parser.rs`, `nextest.rs`) are empty stubs. The library crates cannot yet extract data from their respective sources.

**The Solution**: 
1. Implement git log parsing using `git2` crate
2. Implement nextest output parsing using `nextest-metadata` crate
3. Implement Copilot session discovery and database insertion
4. Create a unified ingestion API in `hindsight-mcp` that orchestrates all sources
5. Add comprehensive tests for real-world data scenarios

---

## Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| Git commits parseable | Last 100 commits from repo | ⏳ Pending |
| Test results parseable | From nextest JSON output | ⏳ Pending |
| Copilot sessions parseable | From VS Code storage | ⏳ Pending |
| Database population | All sources insertable | ⏳ Pending |
| Integration tests | ≥10 new tests | ⏳ Pending |
| Unit tests | ≥20 new tests | ⏳ Pending |
| End-to-end test | Full pipeline verified | ⏳ Pending |

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `git2` | 0.20 | Git repository access |
| `nextest-metadata` | 0.13 | Parse nextest JSON output |
| `rusqlite` | 0.38 | SQLite database (bundled) |
| `uuid` | 1.0 | Generate UUIDs for records |
| `chrono` | 0.4 | Timestamp handling |

---

## Phase Breakdown

### Phase 0: Git Log Parsing (1.5 sessions)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Implement `hindsight-git::parser` to extract commits from a git repository

#### Tasks

1. **Repository access** (~60 lines) ✅
   - Open repository from path
   - Handle bare vs worktree repos
   - Error handling for invalid paths

2. **Commit walker** (~80 lines) ✅
   - Walk commit history from HEAD
   - Support limiting by count or date range
   - Handle merge commits correctly

3. **Commit extraction** (~100 lines) ✅
   - Extract SHA, author, email, message, timestamp
   - Parse parent SHAs
   - Convert to `Commit` struct

4. **Diff parsing** (~120 lines) ✅
   - Compute diff between commit and parent
   - Extract file paths, insertions, deletions
   - Generate `diff_json` structure

5. **Unit tests** (~80 lines) ✅
   - Test repository opening
   - Test commit walking limits
   - Test diff extraction
   - Test error handling

#### Deliverables

- `crates/hindsight-git/src/parser.rs` - Full implementation (~340 lines) ✅
- `crates/hindsight-git/src/lib.rs` - Extended with exports ✅

#### Validation Gate

```bash
cargo nextest run -p hindsight-git  # ✅ 42 tests pass
cargo clippy -p hindsight-git       # ✅ No warnings
```

#### Success Criteria

- [x] Can open git repository from path
- [x] Can walk N most recent commits
- [x] Can extract all commit metadata
- [x] Can parse diffs to JSON structure
- [x] ≥8 unit tests pass (14 new tests)

**Commit**: `feat(git): implement git log parsing with git2`

---

### Phase 1: Nextest Output Parsing (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Implement `hindsight-tests::nextest` to parse cargo-nextest JSON output
**Dependencies**: None (can run in parallel with Phase 0)

#### Tasks

1. **JSON stream parsing** (~80 lines) ✅
   - Parse nextest `--message-format json` output
   - Handle test-start, test-finish events
   - Extract test list from `nextest list --message-format json`

2. **Test result extraction** (~100 lines) ✅
   - Map nextest events to `TestResult` struct
   - Extract suite name, test name, outcome
   - Capture duration, stdout, stderr

3. **Test run aggregation** (~60 lines) ✅
   - Group results by test run
   - Calculate pass/fail/ignored counts
   - Extract build metadata

4. **Unit tests** (~80 lines) ✅
   - Test JSON parsing
   - Test result extraction
   - Test aggregation logic
   - Test with sample fixtures

#### Deliverables

- `crates/hindsight-tests/src/nextest.rs` - Full implementation (~350 lines) ✅
- `crates/hindsight-tests/src/lib.rs` - Extended with exports ✅

#### Validation Gate

```bash
cargo nextest run -p hindsight-tests  # ✅ 37 tests pass
cargo clippy -p hindsight-tests       # ✅ No warnings
```

#### Success Criteria

- [x] Can parse nextest JSON stream
- [x] Can extract all test result fields
- [x] Can aggregate into test runs
- [x] ≥6 unit tests pass (10 new tests)

**Commit**: `feat(tests): implement nextest output parsing`

---

### Phase 2: Copilot Session Discovery (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Implement discovery of Copilot chat sessions from VS Code storage
**Dependencies**: None (can run in parallel with Phases 0-1)

#### Tasks

1. **Session file discovery** (~80 lines) ✅
   - Locate VS Code storage directory per platform
   - Enumerate workspace directories
   - Find chat session JSON files

2. **Session parsing** (~60 lines) ✅
   - Parse session JSON structure
   - Extract session metadata
   - Convert to `ChatSession` struct

3. **Message extraction** (~80 lines) ✅
   - Extract user/assistant messages from requests
   - Parse variables (file references, selections)
   - Handle nested message structures

4. **Workspace correlation** (~50 lines) ✅
   - Map VS Code workspace ID to file path
   - Read `workspace.json` for workspace info
   - Handle missing or corrupted files

5. **Unit tests** (~70 lines) ✅
   - Test session discovery on supported platforms
   - Test JSON parsing
   - Test message extraction
   - Test error handling

#### Deliverables

- `crates/hindsight-copilot/src/session.rs` - Extended with discovery functions (~550 new lines) ✅
- `crates/hindsight-copilot/src/lib.rs` - Extended with exports ✅

#### Validation Gate

```bash
cargo nextest run -p hindsight-copilot  # ✅ 62 tests pass
cargo clippy -p hindsight-copilot       # ✅ No warnings
```

#### Success Criteria

- [x] Can discover VS Code storage directory
- [x] Can enumerate chat session files
- [x] Can parse session JSON to structs
- [x] Can extract messages with variables
- [x] ≥6 unit tests pass (12 new tests)

**Commit**: `feat(copilot): implement session discovery and parsing`

---

### Phase 3: Database Insertion Layer (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Create insertion functions that populate the database from parsed data
**Dependencies**: Phases 0, 1, 2

#### Tasks

1. **Workspace management** (~60 lines) ✅
   - `insert_workspace()` - Create or update workspace
   - `get_or_create_workspace()` - Idempotent upsert
   - `list_workspaces()` - Enumerate all workspaces

2. **Commit insertion** (~80 lines) ✅
   - `insert_commit()` - Single commit with JSON columns
   - `insert_commits_batch()` - Bulk insert with transaction
   - `get_commit_by_sha()` - Retrieve by SHA
   - Update FTS5 index (via triggers)

3. **Test result insertion** (~100 lines) ✅
   - `insert_test_run()` - Create test run record
   - `insert_test_results_batch()` - Bulk insert results
   - `link_test_run_to_commit()` - Associate with SHA

4. **Copilot insertion** (~100 lines) ✅
   - `insert_copilot_session()` - Create session record (idempotent)
   - `insert_copilot_messages_batch()` - Bulk insert messages
   - `get_session_message_count()` - Query message count
   - Update FTS5 index (via triggers)

5. **Unit tests** (~100 lines) ✅
   - Test workspace CRUD (4 tests)
   - Test commit insertion with JSON (4 tests)
   - Test test result insertion (3 tests)
   - Test Copilot insertion (4 tests)
   - Test record type builders (7 tests)

#### Deliverables

- `crates/hindsight-mcp/src/db.rs` - Extended with insertion functions (~800 lines total) ✅
- Record types: `WorkspaceRecord`, `CommitRecord`, `TestRunRecord`, `TestResultRecord`, `CopilotSessionRecord`, `CopilotMessageRecord` ✅
- Builder pattern with `with_*()` methods for optional fields ✅

#### Validation Gate

```bash
cargo nextest run -p hindsight-mcp  # ✅ 61 tests pass
cargo clippy -p hindsight-mcp       # ✅ No warnings
```

#### Success Criteria

- [x] Can insert workspaces
- [x] Can insert commits with FTS5 update
- [x] Can insert test runs and results
- [x] Can insert Copilot sessions and messages
- [x] ≥10 unit tests pass (22 new tests)

**Commit**: `feat(db): implement database insertion layer`

---

### Phase 4: Unified Ingestion API (1 session)

**Status**: ✅ completed
**Completed**: 2026-01-17
**Goal**: Create a high-level API that orchestrates ingestion from all sources
**Dependencies**: Phase 3

#### Tasks

1. **Ingestion orchestrator** (~100 lines) ✅
   - `Ingestor` struct with database handle
   - `ingest_git()` - Ingest commits from repository
   - `ingest_tests()` - Ingest from nextest output
   - `ingest_copilot()` - Ingest from VS Code storage

2. **Incremental sync** (~80 lines) ✅
   - Track last ingested commit SHA
   - Only ingest new commits since last sync
   - Handle deleted/amended commits

3. **Progress reporting** (~40 lines) ✅
   - Callback for progress updates
   - Count processed items
   - Report errors without aborting

4. **Integration tests** (~120 lines) ✅
   - Test full git ingestion pipeline
   - Test nextest ingestion pipeline
   - Test Copilot ingestion pipeline
   - Test incremental sync

#### Deliverables

- `crates/hindsight-mcp/src/ingest.rs` - Full implementation (~823 lines) ✅
- `crates/hindsight-mcp/src/lib.rs` - New file exposing public API ✅
- `crates/hindsight-mcp/tests/integration_tests.rs` - Extended with 7 ingestor tests ✅

#### Validation Gate

```bash
cargo nextest run --workspace  # ✅ 220 tests pass
cargo clippy --workspace       # ✅ No warnings
```

#### Success Criteria

- [x] `Ingestor` can orchestrate all sources
- [x] Incremental sync works for git
- [x] Progress reporting implemented
- [x] ≥4 integration tests pass (7 new tests)

**Commit**: `feat(ingest): implement unified ingestion API`

---

### Phase 5: End-to-End Validation (0.5 session)

**Status**: ⏳ not-started
**Goal**: Validate the complete pipeline with real data from this workspace
**Dependencies**: Phase 4

#### Tasks

1. **Real-world test** (~50 lines)
   - Ingest commits from hindsight-mcp repo
   - Run nextest and ingest results
   - Ingest Copilot sessions (if available)

2. **Query validation** (~40 lines)
   - Verify timeline view works
   - Verify FTS5 search works
   - Verify cross-table joins

3. **Performance baseline** (~30 lines)
   - Measure ingestion time for 100 commits
   - Measure query times
   - Document baseline in milestone

4. **Documentation** (~40 lines)
   - Update ARCHITECTURE.md with ingestion flow
   - Add usage examples to README

#### Deliverables

- Updated `ARCHITECTURE.md` with ingestion documentation
- Performance baseline recorded

#### Validation Gate

```bash
cargo nextest run --workspace
cargo run -- --help  # Verify binary runs
```

#### Success Criteria

- [ ] Real data ingested successfully
- [ ] Queries return expected results
- [ ] Performance baseline established
- [ ] Documentation updated

**Commit**: `docs(ingest): document data ingestion pipeline`

---

## Architecture Overview

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
│ parser.rs       │ │ nextest.rs      │ │ session.rs + parser.rs      │
│ - walk commits  │ │ - parse JSON    │ │ - discover sessions         │
│ - extract diffs │ │ - extract runs  │ │ - parse messages            │
│ -> Vec<Commit>  │ │ -> Vec<Result>  │ │ -> Vec<ChatSession>         │
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

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| git2 API complexity | Medium | Medium | Consult git2 examples, start simple |
| nextest-metadata API changes | Low | Medium | Pin version, test with fixtures |
| VS Code storage format changes | Medium | Low | Graceful fallback, version detection |
| Large repo performance | Medium | Medium | Implement pagination, batch inserts |
| FTS5 trigger overhead | Low | Low | Benchmark, consider async rebuild |

---

## Notes

### Parallel Development

Phases 0, 1, and 2 can be developed in parallel as they have no interdependencies. This allows faster completion if multiple sessions are available.

### Future Considerations

This milestone focuses on ingestion only. The next milestone will implement:
- MCP server with tool handlers
- Query tools exposed via MCP protocol
- VS Code integration testing

### Testing Strategy

- Unit tests use in-memory SQLite
- Integration tests use temporary directories
- Real-world tests use the hindsight-mcp repository itself
