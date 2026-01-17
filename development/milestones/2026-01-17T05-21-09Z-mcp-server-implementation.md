# MILESTONE: MCP Server Implementation

**Status**: 🔄 IN PROGRESS
**Priority**: 🔴 CRITICAL
**Created**: 2026-01-17T05:21:09Z
**Estimated Duration**: 4-5 sessions

---

## Executive Summary

**Objective**: Implement the MCP server that exposes development history data (git commits, test results, Copilot sessions) to LLMs via MCP tool calls, making hindsight-mcp a fully functional MCP server.

**Current State**: 
- SQLite schema fully implemented with 7 tables, FTS5 search, 3 views (302 lines)
- Query helper functions in `queries.rs` (728 lines): `get_timeline`, `search_commits`, `search_messages`, `search_all`, `get_failing_tests`, `get_activity_summary`, `get_commit_with_tests`
- Data ingestion pipeline complete via `Ingestor` in `ingest.rs` (823 lines)
- Database operations in `db.rs` with record types and batch insertion
- 255 tests passing across all crates
- MCP server skeleton complete: CLI parsing, tool schemas, server initialization
- Tool handlers implemented in `handlers.rs` (520 lines): all 6 tools wired to database queries
- **Next**: Server configuration and transport (Phase 3)

**The Problem**: All the data infrastructure exists but there is no way for an LLM to access it. The MCP server binary does nothing—it cannot register tools, handle requests, or expose queries to AI clients.

**The Solution**: 
1. Implement MCP server using `rust-mcp-sdk` with stdio transport
2. Define tool schemas for each query function
3. Implement tool handlers that bridge MCP requests to database queries
4. Add server configuration (database path, workspace selection)
5. Create command-line interface with proper argument handling
6. Test with actual MCP clients (Claude Desktop, VS Code Copilot)

---

## Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| MCP server starts | Binary runs and accepts connections | ✅ Done |
| Tools registered | ≥6 tools exposed via MCP | ✅ Done (6 tools) |
| Tool execution | All tools return valid JSON | ✅ Done |
| Database integration | Queries run against SQLite | ✅ Done |
| Configuration | CLI args for db path, workspace | ✅ Done |
| VS Code integration | Works with Copilot MCP | ⏳ Pending |
| Integration tests | ≥8 new tests | ✅ Done (255 total) |
| Documentation | README updated with usage | ⏳ Pending |

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `rust-mcp-sdk` | 0.2 | MCP server implementation |
| `tokio` | 1.x | Async runtime |
| `clap` | 4.x | CLI argument parsing |
| `rusqlite` | 0.38 | SQLite database (bundled) |
| `serde_json` | 1.0 | JSON serialization |

---

## MCP Tools Specification

Based on the existing query functions, these tools will be exposed:

### 1. `hindsight_timeline`

Get a chronological view of development activity.

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "limit": { "type": "integer", "default": 50, "description": "Maximum events to return" },
    "workspace": { "type": "string", "description": "Filter by workspace path (optional)" }
  }
}
```

**Output**: Array of `TimelineEvent` objects with type, timestamp, content, reference.

### 2. `hindsight_search`

Full-text search across all sources (commits, messages, tests).

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "query": { "type": "string", "description": "Search query (FTS5 syntax supported)" },
    "source": { "type": "string", "enum": ["all", "commits", "messages"], "default": "all" },
    "limit": { "type": "integer", "default": 20 }
  },
  "required": ["query"]
}
```

**Output**: Array of `SearchResult` objects with source, content, timestamp, relevance.

### 3. `hindsight_failing_tests`

Get currently failing tests.

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "limit": { "type": "integer", "default": 50 },
    "workspace": { "type": "string", "description": "Filter by workspace (optional)" }
  }
}
```

**Output**: Array of failing test objects with suite, name, output, last failure time.

### 4. `hindsight_activity_summary`

Get aggregate activity statistics.

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "days": { "type": "integer", "default": 7, "description": "Number of days to summarize" }
  }
}
```

**Output**: `ActivitySummary` with counts of commits, test runs, copilot sessions, failing tests.

### 5. `hindsight_commit_details`

Get detailed information about a specific commit including linked test runs.

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sha": { "type": "string", "description": "Full or partial commit SHA" }
  },
  "required": ["sha"]
}
```

**Output**: `CommitWithTests` object with commit details and associated test runs.

### 6. `hindsight_ingest`

Trigger data ingestion from sources.

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "source": { "type": "string", "enum": ["git", "tests", "copilot", "all"], "default": "all" },
    "workspace": { "type": "string", "description": "Workspace path to ingest" },
    "incremental": { "type": "boolean", "default": true },
    "limit": { "type": "integer", "description": "Max items to ingest (optional)" }
  },
  "required": ["workspace"]
}
```

**Output**: `IngestStats` with counts of items processed.

---

## Phase Breakdown

### Phase 0: MCP Server Skeleton (1 session)

**Status**: ✅ completed
**Goal**: Implement basic MCP server that starts and responds to `initialize` and `tools/list`
**Dependencies**: None

#### Tasks

1. **Add tokio and clap dependencies** (~10 lines)
   - Add `tokio` with full features to workspace
   - Add `clap` with derive feature for CLI
   - Update `hindsight-mcp/Cargo.toml`

2. **Implement CLI argument parsing** (~60 lines)
   - `--database` / `-d` - Path to SQLite database
   - `--workspace` / `-w` - Default workspace path
   - `--verbose` / `-v` - Enable debug logging
   - `--version` - Show version info

3. **Create MCP server struct** (~100 lines)
   - `HindsightServer` struct with database handle
   - Implement `rust_mcp_sdk::server::Server` trait
   - Handle `initialize` request
   - Implement `tools/list` to return tool schemas

4. **Wire up main.rs** (~50 lines)
   - Parse CLI arguments
   - Initialize database
   - Create server instance
   - Start stdio transport

5. **Unit tests** (~80 lines)
   - Test CLI argument parsing
   - Test server initialization
   - Test tool list response

#### Deliverables

- `crates/hindsight-mcp/src/main.rs` - CLI and server startup (~120 lines)
- `crates/hindsight-mcp/src/server.rs` - MCP server implementation (~200 lines)
- Updated `Cargo.toml` with new dependencies

#### Validation Gate

```bash
cargo build -p hindsight-mcp
./target/debug/hindsight-mcp --help
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./target/debug/hindsight-mcp
cargo nextest run -p hindsight-mcp
```

#### Success Criteria

- [x] Binary builds and runs
- [x] `--help` shows usage information
- [x] Server responds to `initialize` with capabilities
- [x] `tools/list` returns tool schemas
- [x] ≥3 unit tests pass (8 new tests)

**Commit**: `feat(mcp): implement MCP server skeleton with tool listing`

---

### Phase 1: Tool Schemas and Input Validation (0.5 session)

**Status**: ✅ completed
**Goal**: Define JSON schemas for all tools with input validation
**Dependencies**: Phase 0

#### Tasks

1. **Define tool schemas** (~150 lines)
   - `hindsight_timeline` schema
   - `hindsight_search` schema
   - `hindsight_failing_tests` schema
   - `hindsight_activity_summary` schema
   - `hindsight_commit_details` schema
   - `hindsight_ingest` schema

2. **Implement input types** (~100 lines)
   - `TimelineInput`, `SearchInput`, `FailingTestsInput`, etc.
   - Derive `Deserialize` for JSON parsing
   - Add validation with `#[serde(default)]`

3. **Wire schemas to tool list** (~40 lines)
   - Return schemas in `tools/list` response
   - Include name, description, inputSchema

4. **Unit tests** (~60 lines)
   - Test schema serialization
   - Test input deserialization
   - Test validation edge cases

#### Deliverables

- `crates/hindsight-mcp/src/tools.rs` - Tool definitions and schemas (~300 lines)
- Updated `server.rs` with tool registration

#### Validation Gate

```bash
cargo nextest run -p hindsight-mcp
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | ./target/debug/hindsight-mcp
```

#### Success Criteria

- [x] All 6 tool schemas defined
- [x] Input types deserialize correctly
- [x] `tools/list` returns complete schemas
- [x] ≥4 unit tests pass

**Commit**: `feat(mcp): define tool schemas with input validation`

---

### Phase 2: Tool Handler Implementation (1.5 sessions)

**Status**: ✅ completed
**Goal**: Implement handlers for all tools that execute database queries
**Dependencies**: Phase 1

#### Tasks

1. **Implement `hindsight_timeline` handler** (~40 lines)
   - Parse `TimelineInput`
   - Call `queries::get_timeline()`
   - Serialize response as JSON content

2. **Implement `hindsight_search` handler** (~50 lines)
   - Parse `SearchInput`
   - Route to `search_all`, `search_commits`, or `search_messages`
   - Return results array

3. **Implement `hindsight_failing_tests` handler** (~40 lines)
   - Parse `FailingTestsInput`
   - Call `queries::get_failing_tests()`
   - Format test failures

4. **Implement `hindsight_activity_summary` handler** (~30 lines)
   - Parse `ActivitySummaryInput`
   - Call `queries::get_activity_summary()`
   - Return summary object

5. **Implement `hindsight_commit_details` handler** (~40 lines)
   - Parse `CommitDetailsInput`
   - Call `queries::get_commit_with_tests()`
   - Handle not-found case

6. **Implement `hindsight_ingest` handler** (~80 lines)
   - Parse `IngestInput`
   - Create `Ingestor` with options
   - Execute appropriate ingest method
   - Return stats

7. **Error handling** (~60 lines)
   - Map database errors to MCP errors
   - Return meaningful error messages
   - Handle invalid inputs gracefully

8. **Integration tests** (~120 lines)
   - Test each tool with valid inputs
   - Test error cases
   - Test with populated database

#### Deliverables

- `crates/hindsight-mcp/src/handlers.rs` - Tool handlers (~520 lines)
- Updated `server.rs` with handler routing and `db_path` field
- Updated `lib.rs` to export handlers module
- Extended integration tests (6 new tests)

#### Validation Gate

```bash
cargo nextest run -p hindsight-mcp
# Test with real MCP request
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"hindsight_activity_summary","arguments":{"days":7}}}' | ./target/debug/hindsight-mcp -d test.db
```

#### Success Criteria

- [x] All 6 tool handlers implemented
- [x] Handlers return valid JSON responses
- [x] Error cases handled gracefully
- [x] ≥6 integration tests pass (6 handler tests + existing)

**Commit**: `feat(mcp): implement tool handlers with database queries`

---

### Phase 3: Server Configuration and Transport (1 session)

**Status**: ⏳ not-started
**Goal**: Complete server configuration, transport setup, and graceful shutdown
**Dependencies**: Phase 2

#### Tasks

1. **Database lifecycle management** (~60 lines)
   - Auto-create database if not exists
   - Run migrations on startup
   - Handle connection pooling (single connection for SQLite)
   - Graceful shutdown with cleanup

2. **Workspace management** (~50 lines)
   - Auto-detect workspace from cwd
   - Store default workspace in config
   - Allow per-request workspace override

3. **Logging configuration** (~40 lines)
   - Configure tracing with levels
   - Log MCP requests/responses at debug
   - Log errors at error level
   - Support `--verbose` flag

4. **Stdio transport** (~40 lines)
   - Set up stdin/stdout handlers
   - Handle EOF gracefully
   - Buffer responses appropriately

5. **Server capabilities** (~30 lines)
   - Declare tool capabilities
   - Handle capability negotiation
   - Return proper server info

6. **Integration tests** (~80 lines)
   - Test server startup/shutdown
   - Test database creation
   - Test workspace detection
   - Test logging output

#### Deliverables

- Updated `main.rs` with complete lifecycle
- Updated `server.rs` with configuration
- `crates/hindsight-mcp/src/config.rs` - Configuration types (~100 lines)

#### Validation Gate

```bash
cargo nextest run -p hindsight-mcp
cargo run -p hindsight-mcp -- --help
cargo run -p hindsight-mcp -- -d /tmp/test.db -w . -v &
# Send test requests
```

#### Success Criteria

- [ ] Server creates database on startup
- [ ] Migrations run automatically
- [ ] Workspace detected from cwd
- [ ] Logging respects verbosity flag
- [ ] ≥4 integration tests pass

**Commit**: `feat(mcp): add server configuration and transport`

---

### Phase 4: VS Code Integration and Testing (0.5 session)

**Status**: ⏳ not-started
**Goal**: Test with real MCP clients and document usage
**Dependencies**: Phase 3

#### Tasks

1. **Create VS Code MCP configuration** (~30 lines)
   - `.vscode/mcp.json` example
   - Document server path configuration
   - Document environment variables

2. **Test with Claude Desktop** (~testing)
   - Install in Claude Desktop config
   - Verify tool discovery
   - Test each tool manually

3. **Test with VS Code Copilot** (~testing)
   - Configure as MCP server
   - Verify tool integration
   - Document any limitations

4. **Update README** (~80 lines)
   - Installation instructions
   - Configuration examples
   - Tool documentation
   - Troubleshooting guide

5. **Update ARCHITECTURE.md** (~60 lines)
   - MCP server architecture section
   - Tool reference
   - Transport details

#### Deliverables

- `.vscode/mcp.json` - Example configuration
- Updated `README.md` with usage documentation
- Updated `ARCHITECTURE.md` with MCP details

#### Validation Gate

```bash
cargo nextest run --workspace
cargo build --release -p hindsight-mcp
./target/release/hindsight-mcp --version
```

#### Success Criteria

- [ ] Works with Claude Desktop
- [ ] Works with VS Code Copilot MCP
- [ ] README has complete usage docs
- [ ] ARCHITECTURE.md updated
- [ ] All 235+ tests pass

**Commit**: `docs(mcp): add usage documentation and VS Code integration`

---

### Phase 5: Performance and Polish (0.5 session)

**Status**: ⏳ not-started
**Goal**: Optimize performance and add finishing touches
**Dependencies**: Phase 4

#### Tasks

1. **Performance optimization** (~40 lines)
   - Benchmark tool response times
   - Optimize hot paths if needed
   - Add query caching if beneficial

2. **Error message improvements** (~30 lines)
   - User-friendly error messages
   - Actionable suggestions
   - Context in errors

3. **Release preparation** (~20 lines)
   - Update version in Cargo.toml
   - Create release profile optimizations
   - Document build process

4. **Final testing** (~60 lines)
   - End-to-end test with real data
   - Performance baseline documentation
   - Edge case verification

#### Deliverables

- Performance baseline documentation
- Updated Cargo.toml with release settings
- Final test results

#### Validation Gate

```bash
cargo nextest run --workspace
cargo build --release -p hindsight-mcp
cargo bench -p hindsight-mcp
```

#### Success Criteria

- [ ] Tool responses < 100ms average
- [ ] No memory leaks in extended use
- [ ] Release build optimized
- [ ] All tests pass

**Commit**: `perf(mcp): optimize tool performance and release preparation`

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        LLM / AI Client                               │
│                 (Claude Desktop, VS Code Copilot)                    │
└────────────────────────────┬────────────────────────────────────────┘
                             │ MCP Protocol (JSON-RPC over stdio)
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       hindsight-mcp                                  │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │    main.rs      │  │   server.rs     │  │     tools.rs        │  │
│  │                 │  │                 │  │                     │  │
│  │ - CLI parsing   │  │ - MCP Server    │  │ - Tool schemas      │  │
│  │ - DB init       │──│ - Request route │──│ - Input types       │  │
│  │ - Server start  │  │ - Response fmt  │  │ - Validation        │  │
│  └─────────────────┘  └────────┬────────┘  └─────────────────────┘  │
│                                │                                     │
│  ┌─────────────────────────────▼─────────────────────────────────┐  │
│  │                      handlers.rs                               │  │
│  │                                                                │  │
│  │  hindsight_timeline    → get_timeline()                        │  │
│  │  hindsight_search      → search_all() / search_commits()       │  │
│  │  hindsight_failing_tests → get_failing_tests()                 │  │
│  │  hindsight_activity_summary → get_activity_summary()           │  │
│  │  hindsight_commit_details → get_commit_with_tests()            │  │
│  │  hindsight_ingest      → Ingestor.ingest_*()                   │  │
│  └─────────────────────────────┬─────────────────────────────────┘  │
│                                │                                     │
│  ┌─────────────────────────────▼─────────────────────────────────┐  │
│  │              queries.rs              │         db.rs           │  │
│  │                                      │                         │  │
│  │  get_timeline()                      │  Database struct        │  │
│  │  search_commits()                    │  insert_*()             │  │
│  │  search_messages()                   │  get_*()                │  │
│  │  get_failing_tests()                 │  batch operations       │  │
│  │  get_activity_summary()              │                         │  │
│  │  get_commit_with_tests()             │                         │  │
│  └──────────────────────────────────────┴─────────────────────────┘  │
│                                │                                     │
│  ┌─────────────────────────────▼─────────────────────────────────┐  │
│  │                    SQLite Database                             │  │
│  │                                                                │  │
│  │  Tables: workspaces, commits, test_runs, test_results,         │  │
│  │          copilot_sessions, copilot_messages                    │  │
│  │  FTS5: commits_fts, copilot_messages_fts                       │  │
│  │  Views: timeline, failing_tests, recent_activity               │  │
│  └────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### MCP Message Flow

```
Client                    Server
  │                         │
  │──── initialize ────────►│
  │◄─── capabilities ───────│
  │                         │
  │──── tools/list ────────►│
  │◄─── tool schemas ───────│
  │                         │
  │──── tools/call ────────►│
  │     (hindsight_search)  │
  │                         │
  │     ┌───────────────────┤
  │     │ Parse input       │
  │     │ Execute query     │
  │     │ Format response   │
  │     └───────────────────┤
  │                         │
  │◄─── content result ─────│
  │                         │
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| rust-mcp-sdk API instability | Medium | High | Pin version, abstract behind traits |
| Async complexity with SQLite | Medium | Medium | Use sync SQLite, spawn_blocking |
| Tool schema changes breaking clients | Low | Medium | Version tool schemas, deprecation |
| Large query results overwhelming LLM | Medium | Low | Enforce limits, pagination |
| Stdio transport issues on Windows | Low | Medium | Test on Windows, document workarounds |

---

## Notes

### Transport Choice

Using stdio transport (not HTTP) because:
- Simpler setup for VS Code integration
- No port management required
- Direct process communication
- Standard for MCP servers

### SQLite Thread Safety

SQLite connections are not thread-safe. Since MCP requests are sequential over stdio, a single connection is sufficient. If concurrent requests are needed in the future, consider connection pooling or `spawn_blocking`.

### Tool Naming Convention

All tools prefixed with `hindsight_` to avoid conflicts with other MCP servers that might be registered alongside.

### Future Enhancements

After this milestone, potential follow-ups include:
- **Resources**: Expose data as MCP resources (workspace list, etc.)
- **Prompts**: Pre-built prompts for common workflows
- **Streaming**: Stream large query results
- **Watch mode**: Real-time updates on data changes
