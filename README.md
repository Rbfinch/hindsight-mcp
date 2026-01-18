# hindsight-mcp

[![CI](https://github.com/Rbfinch/hindsight-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/Rbfinch/hindsight-mcp/actions/workflows/ci.yml)

An MCP server for AI-assisted coding that leverages development history.

## Overview

**hindsight-mcp** consolidates various "development data" stored locally (git logs, nextest test results, and GitHub Copilot logs) into a well-structured, searchable SQLite database, making it accessible to an AI assistant through MCP tool calls within VS Code. It is designed to help Rust developers and an AI assistant gain insights into their coding history, find relevant information quickly, and improve productivity by providing context-aware assistance.

## Quick Start

### Installation

**Option A: Install from crates.io** (recommended)
```bash
cargo install hindsight-mcp
```

**Option B: Build from source**
```bash
git clone https://github.com/Rbfinch/hindsight-mcp.git
cd hindsight-mcp
cargo build --release

# The binary is at ./target/release/hindsight-mcp
```

### Configure VS Code

Add to `.vscode/mcp.json` in your project:

```json
{
  "servers": {
    "hindsight": {
      "type": "stdio",
      "command": "hindsight-mcp",
      "args": ["--workspace", "${workspaceFolder}"]
    }
  }
}
```

> **Note:** If installed via `cargo install`, `hindsight-mcp` will be in your PATH. Otherwise, use the full path to the binary.

### First Run

On first use, ingest your development history:

```bash
# Option A: Run the server directly (it auto-creates the database)
./target/release/hindsight-mcp -w /path/to/your/project
```

Then ask Copilot to use the `hindsight_ingest` tool, or ingest happens automatically on first query.

**That's it!** You can now ask Copilot questions like:
- "What have I been working on recently?" → uses `hindsight_timeline`
- "Find commits about authentication" → uses `hindsight_search`
- "What tests are failing?" → uses `hindsight_failing_tests`
- "Summarise my activity this week" → uses `hindsight_activity_summary`

### Available Tools

| Tool | Purpose |
|------|---------|
| `hindsight_timeline` | Chronological view of commits, tests, Copilot sessions |
| `hindsight_search` | Full-text search across commits and messages |
| `hindsight_failing_tests` | Currently failing tests from recent runs |
| `hindsight_activity_summary` | Aggregate stats for a time period |
| `hindsight_commit_details` | Detailed commit info with linked test runs |
| `hindsight_ingest` | Trigger data ingestion from git/copilot |

### Claude Desktop Configuration

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "hindsight": {
      "command": "/path/to/hindsight-mcp",
      "args": ["--workspace", "/path/to/your/project"]
    }
  }
}
```

## Usage

### Command Line Options

```
hindsight-mcp [OPTIONS] [COMMAND]

Commands:
  ingest    Ingest data from various sources (e.g., test results)
  help      Print help for commands

Options:
  -d, --database <PATH>   Path to SQLite database file
                          [env: HINDSIGHT_DATABASE]
                          [default: ~/.hindsight/hindsight.db]

  -w, --workspace <PATH>  Default workspace path for queries
                          [env: HINDSIGHT_WORKSPACE]
                          [default: current directory]

  -v, --verbose           Enable verbose logging (debug level)

  -q, --quiet             Suppress info-level logs (errors/warnings only)

      --skip-init         Skip database initialization/migration check

  -h, --help              Print help

  -V, --version           Print version
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `HINDSIGHT_DATABASE` | Path to SQLite database (alternative to `--database`) |
| `HINDSIGHT_WORKSPACE` | Default workspace path (alternative to `--workspace`) |

### Ingest Subcommand

The `ingest` command imports test results from stdin:

```
hindsight-mcp ingest [OPTIONS]

Options:
      --tests             Ingest test results from stdin (nextest JSON format)
      --commit <SHA>      Git commit SHA to associate with test results
  -h, --help              Print help
```

**Example:**
```bash
NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --message-format libtest-json | \
  hindsight-mcp ingest --tests --commit $(git rev-parse HEAD)
```

## MCP Tools

hindsight-mcp exposes 6 tools for AI-assisted development:

### `hindsight_timeline`

Get a chronological view of development activity (commits, test runs, Copilot sessions).

```
Arguments:
  limit (integer): Maximum events to return (default: 50)
  workspace (string): Filter by workspace path (optional)
```

### `hindsight_search`

Full-text search across commits and Copilot messages using FTS5 syntax.

```
Arguments:
  query (string): Search query (required)
  source (string): "all", "commits", or "messages" (default: "all")
  limit (integer): Maximum results (default: 20)
```

### `hindsight_failing_tests`

Get currently failing tests from recent test runs, optionally filtered by commit.

```
Arguments:
  limit (integer): Maximum tests to return (default: 50)
  workspace (string): Filter by workspace (optional)
  commit (string): Filter by commit SHA - full or partial (optional)
```

**Example queries:**
- All failing tests: `hindsight_failing_tests()`
- For a specific commit: `hindsight_failing_tests(commit: "a566594")`
- Combined filters: `hindsight_failing_tests(workspace: "/path/to/project", commit: "abc123")`

### `hindsight_activity_summary`

Get aggregate activity statistics for a time period.

```
Arguments:
  days (integer): Number of days to summarize (default: 7)
```

### `hindsight_commit_details`

Get detailed information about a specific commit including linked test runs.

```
Arguments:
  sha (string): Full or partial commit SHA (required)
```

### `hindsight_ingest`

Trigger data ingestion from sources (git, Copilot).

```
Arguments:
  workspace (string): Workspace path to ingest (required)
  source (string): "git", "copilot", or "all" (default: "all")
  incremental (boolean): Only ingest new data (default: true)
  limit (integer): Max items to ingest (optional)
```

## Data Sources

### Git Commits

Automatically ingests commit history including:
- SHA, author, message, timestamp
- Parent commit references
- Optional diff statistics

### Test Results

Ingests cargo-nextest output:
- Test run metadata (pass/fail counts)
- Individual test outcomes
- Duration and output capture

#### Ingesting Test Results

Test results require piping nextest JSON output to the CLI:

```bash
# Run tests and ingest results
NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --message-format libtest-json 2>/dev/null | \
  hindsight-mcp --database ~/.hindsight/hindsight.db --workspace /path/to/project ingest --tests

# Ingest specific test targets
NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --package my-crate --message-format libtest-json 2>/dev/null | \
  hindsight-mcp --database ~/.hindsight/hindsight.db --workspace /path/to/project ingest --tests

# Associate test run with a specific commit
NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --message-format libtest-json 2>/dev/null | \
  hindsight-mcp --workspace /path/to/project ingest --tests --commit abc123def
```

**Note:** The `2>/dev/null` redirects compiler warnings/errors to avoid mixing them with the JSON output.

#### Querying Failing Tests

After ingesting test results, query for failures using the MCP tool or Copilot:

```
# Ask Copilot:
"What tests are failing?"
"Show me the failing test output"
"Which tests failed in the last run?"
"What tests failed for commit abc123?"
```

The `hindsight_failing_tests` tool returns:
- Test name and suite
- Duration and timestamp
- Failure output (panic messages, assertion errors)
- Associated commit SHA (if linked)

#### Complete Workflow: Linking Tests to Commits

This workflow demonstrates ingesting test results linked to a specific commit, then querying those failures:

**Step 1: Get the current commit SHA**
```bash
git rev-parse HEAD
# Output: a5665945a0efb9f59fea1392dbdbdcc7e5ce48c6
```

**Step 2: Run tests and ingest with commit linkage**
```bash
NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --message-format libtest-json 2>/dev/null | \
  hindsight-mcp --workspace /path/to/project ingest --tests --commit a5665945a0efb9f59fea1392dbdbdcc7e5ce48c6
```

Output:
```
Ingested 6 test results in 1 test run(s)
```

**Step 3: Query failing tests for that commit**

Using the MCP tool (via Copilot or directly):
```
hindsight_failing_tests(commit: "a5665945")
```

Returns failures linked to that specific commit:
```json
[
  {
    "commit_sha": "a5665945a0efb9f59fea1392dbdbdcc7e5ce48c6",
    "full_name": "test_assertion_failure",
    "duration_ms": 8,
    "output_json": "assertion `left == right` failed: left: 2, right: 3"
  },
  {
    "commit_sha": "a5665945a0efb9f59fea1392dbdbdcc7e5ce48c6",
    "full_name": "test_panic_failure",
    "duration_ms": 8,
    "output_json": "panicked at: This test panics on purpose"
  }
]
```

**Step 4: View commit details with linked test runs**
```
hindsight_commit_details(sha: "a5665945")
```

Returns commit info including all associated test runs:
```json
{
  "sha": "a5665945a0efb9f59fea1392dbdbdcc7e5ce48c6",
  "message": "feat: add commit filter to hindsight_failing_tests",
  "test_runs": [
    { "passed": 2, "failed": 4, "timestamp": "2026-01-17T23:38:43Z" }
  ]
}
```

This enables powerful queries like:
- "Which commit introduced these test failures?"
- "Did the tests pass after this fix?"
- "Show me all failures from yesterday's commits"

### GitHub Copilot Sessions

Parses VS Code Copilot chat history:
- User prompts and assistant responses
- Attached files and selections
- Session timestamps

## Database Location

Default database path by platform:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/hindsight/hindsight.db` |
| Linux | `~/.local/share/hindsight/hindsight.db` |
| Windows | `%LOCALAPPDATA%\hindsight\hindsight.db` |

Override with `--database` or `HINDSIGHT_DATABASE`.

## Troubleshooting

### Server doesn't start

1. Check the binary path is correct
2. Verify write permissions for database directory
3. Run with `--verbose` for debug logs:
   ```bash
   hindsight-mcp --verbose --database /tmp/test.db
   ```

### No data showing

1. Run ingestion manually via the `hindsight_ingest` tool
2. Ensure workspace path points to a git repository
3. Check database exists: `ls ~/.hindsight/`

### Logs interference

The server logs to stderr to avoid interfering with MCP stdio transport. Use `--quiet` in production.

## Dependencies

hindsight-mcp bundles its native dependencies for ease of installation:

| Dependency | Purpose | Notes |
|------------|---------|-------|
| `rusqlite` | SQLite database | Bundled; no system SQLite required |
| `git2` | Git repository access | Uses bundled libgit2 |
| `rust-mcp-sdk` | MCP protocol | Pure Rust |
| `tokio` | Async runtime | Pure Rust |

### System Requirements

- **Rust**: 1.89 or later (for building from source)
- **OS**: Linux, macOS, or Windows
- **No additional system libraries required** — all native dependencies are bundled

## Workspace Structure

```
hindsight/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── hindsight-mcp/      # Binary crate - MCP server
│   ├── hindsight-git/      # Library - Git log processing
│   ├── hindsight-tests/    # Library - Test result processing
│   └── hindsight-copilot/  # Library - Copilot log processing
```

## Crates

### hindsight-mcp (binary)
The main MCP server that bridges AI and development history.
- Dependencies: `rust-mcp-sdk`, `rusqlite`, `clap`, `tokio`

### hindsight-git (library)
Processes git logs for consumption by hindsight-mcp.
- Dependencies: `git2`

### hindsight-tests (library)
Processes test logs (particularly from cargo-nextest).
- Dependencies: `nextest-metadata`

### hindsight-copilot (library)
Processes GitHub Copilot logs and chat sessions.
- Dependencies: `serde_json`, `lsp-types`, `tracing-subscriber`

## Development

This project uses [cargo-nextest](https://nexte.st/) as its test runner for faster, more reliable test execution.

### Prerequisites

```bash
# Install cargo-nextest (required for running tests)
cargo install cargo-nextest
```

### Building

```bash
cargo build --workspace
```

### Testing

```bash
# Run tests with nextest (recommended)
cargo nextest run --workspace

# Or use standard cargo test for doc tests
cargo test --workspace --doc
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

## License

MIT
