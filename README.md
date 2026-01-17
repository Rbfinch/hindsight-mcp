# hindsight-mcp

An MCP server for AI-assisted coding that leverages development history.

## Overview

hindsight-mcp consolidates various "development data" stored locally (git logs, test results, and GitHub Copilot logs) into a well-structured, searchable SQLite database, making it accessible to an LLM through MCP tool calls within VS Code.

## Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/Rbfinch/hindsight-mcp.git
cd hindsight-mcp
cargo build --release

# The binary is at ./target/release/hindsight-mcp
```

### VS Code Configuration

Add to your VS Code settings or `.vscode/mcp.json`:

```json
{
  "servers": {
    "hindsight": {
      "type": "stdio",
      "command": "/path/to/hindsight-mcp",
      "args": ["--workspace", "${workspaceFolder}"]
    }
  }
}
```

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
hindsight-mcp [OPTIONS]

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

Get currently failing tests from recent test runs.

```
Arguments:
  limit (integer): Maximum tests to return (default: 50)
  workspace (string): Filter by workspace (optional)
```

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

## License

MIT
