<img src="https://raw.githubusercontent.com/Rbfinch/hindsight-mcp/main/assets/hindsight-logo.png" alt="Hindsight Logo" width="120">

<!-- mcp-name: io.github.rbfinch/hindsight-mcp -->

# hindsight-mcp

An MCP server for AI-assisted coding that leverages development history.

## Overview

**hindsight-mcp** consolidates development data (git logs, test results, and GitHub Copilot sessions) into a searchable SQLite database, making it accessible to AI assistants through MCP tool calls in VS Code.

**Key Features:**
- Full-text search across commits and Copilot conversations
- Track test results linked to specific commits
- Activity summaries and timeline views
- Automatic git and Copilot session ingestion

## Quick Start

### Prerequisites

- **VS Code v1.99+** with GitHub Copilot
- **cargo-nextest** (for test ingestion): `cargo install cargo-nextest`

### Installation

```bash
cargo install hindsight-mcp
```

### Configure VS Code

Create `.vscode/mcp.json` in your project:

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

### Verify Setup

1. Open VS Code Command Palette (`Cmd+Shift+P`)
2. Run **"MCP: List Servers"**
3. Confirm `hindsight` is listed
4. In Copilot Chat, switch to **Agent** mode
5. Ask: *"What have I been working on recently?"*

That's it! Copilot will use hindsight tools to answer questions about your development history.

## MCP Tools

| Tool | Purpose | Example Prompt |
|------|---------|----------------|
| `hindsight_timeline` | Chronological activity view | "Show recent commits and test runs" |
| `hindsight_search` | Full-text search | "Find commits about authentication" |
| `hindsight_failing_tests` | Query test failures | "What tests are failing?" |
| `hindsight_activity_summary` | Aggregate stats | "Summarise my week" |
| `hindsight_commit_details` | Commit info with tests | "Details for commit abc123" |
| `hindsight_ingest` | Trigger data refresh | "Refresh development history" |

<details>
<summary><strong>Tool Arguments Reference</strong></summary>

### hindsight_timeline
- `limit` (int): Max events, default 50
- `workspace` (string): Filter by path

### hindsight_search
- `query` (string): Search query (required)
- `source` (string): "all", "commits", or "messages"
- `limit` (int): Max results, default 20

### hindsight_failing_tests
- `limit` (int): Max tests, default 50
- `workspace` (string): Filter by path
- `commit` (string): Filter by SHA

### hindsight_activity_summary
- `days` (int): Days to summarise, default 7

### hindsight_commit_details
- `sha` (string): Commit SHA (required)

### hindsight_ingest
- `workspace` (string): Path to ingest (required)
- `source` (string): "git", "copilot", or "all"
- `incremental` (bool): Only new data, default true
- `limit` (int): Max items

</details>

## Test Ingestion

Run tests and automatically ingest results:

```bash
# Run all tests and ingest
hindsight-mcp test

# Test specific package
hindsight-mcp test -p my-crate

# Preview without writing to database
hindsight-mcp test --dry-run
```

The `test` command automatically:
- Spawns `cargo nextest` with correct flags
- Auto-detects the current git commit
- Ingests results to the database

<details>
<summary><strong>CI / Advanced Usage</strong></summary>

For CI pipelines or custom nextest invocations:

```bash
# Using stdin mode
NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run \
  --message-format libtest-json 2>/dev/null | \
  hindsight-mcp test --stdin

# Using ingest command with explicit commit
NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run \
  --message-format libtest-json 2>/dev/null | \
  hindsight-mcp ingest --tests --commit $(git rev-parse HEAD)
```

</details>

## Data Sources

| Source | Data Collected |
|--------|----------------|
| **Git** | Commits (SHA, author, message, timestamp, parents) |
| **Tests** | Run metadata, outcomes, durations, failure output |
| **Copilot** | Chat sessions, prompts, responses, attached files |

Git and Copilot data are ingested automatically. Test results require running `hindsight-mcp test`.

## CLI Reference

```
hindsight-mcp [OPTIONS] [COMMAND]

Commands:
  ingest    Ingest data from various sources
  test      Run tests and ingest results

Options:
  -d, --database <PATH>   Database path [default: ~/.hindsight/hindsight.db]
  -w, --workspace <PATH>  Workspace path [default: current directory]
  -v, --verbose           Debug logging
  -q, --quiet             Errors only
      --skip-init         Skip database init
  -h, --help              Print help
  -V, --version           Print version
```

<details>
<summary><strong>Test Subcommand Options</strong></summary>

```
hindsight-mcp test [OPTIONS] [-- <NEXTEST_ARGS>...]

Options:
  -p, --package <PKG>     Package(s) to test
      --bin <BIN>         Binary(ies) to run
  -E, --filter <EXPR>     Filter expression
      --stdin             Read from stdin
      --dry-run           Preview only
      --no-commit         Do not link to commit
      --commit <SHA>      Explicit commit SHA
      --show-output       Show test output
```

</details>

### Environment Variables

| Variable | Description |
|----------|-------------|
| `HINDSIGHT_DATABASE` | Database path |
| `HINDSIGHT_WORKSPACE` | Default workspace |

### Database Location

| Platform | Default Path |
|----------|--------------|
| macOS | `~/Library/Application Support/hindsight/hindsight.db` |
| Linux | `~/.local/share/hindsight/hindsight.db` |
| Windows | `%LOCALAPPDATA%\hindsight\hindsight.db` |

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Server does not start | Check binary path; run with `--verbose` |
| No data showing | Run `hindsight_ingest` tool via Copilot |
| Log interference | Use `--quiet` in production |

## Development

<details>
<summary><strong>Building from Source</strong></summary>

```bash
git clone https://github.com/Rbfinch/hindsight-mcp.git
cd hindsight-mcp
cargo build --release
```

</details>

<details>
<summary><strong>Running Tests</strong></summary>

```bash
cargo install cargo-nextest
cargo nextest run --workspace
```

</details>

<details>
<summary><strong>Workspace Structure</strong></summary>

```
hindsight/
├── crates/
│   ├── hindsight-mcp/      # MCP server binary
│   ├── hindsight-git/      # Git log processing
│   ├── hindsight-tests/    # Test result processing
│   └── hindsight-copilot/  # Copilot session parsing
```

</details>

<details>
<summary><strong>Fuzzing</strong></summary>

```bash
cd crates/hindsight-tests
cargo +nightly fuzz run fuzz_nextest_run

cd crates/hindsight-copilot
cargo +nightly fuzz run fuzz_session_json
```

</details>

## License

MIT
