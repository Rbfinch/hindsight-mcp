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

## Development

### Building

```bash
cargo build --workspace
```

### Testing

```bash
cargo test --workspace
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
