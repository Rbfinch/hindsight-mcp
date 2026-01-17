# hindsight-mcp

An MCP server for AI-assisted coding that leverages development history.

## Overview

hindsight-mcp consolidates various "development data" stored locally (git logs, test results, and GitHub Copilot logs) into a well-structured, searchable SQLite database, making it accessible to an LLM through MCP tool calls within VS Code.

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
- Dependencies: `rust-mcp-sdk`, `rusqlite`

### hindsight-git (library)
Processes git logs for consumption by hindsight-mcp.
- Dependencies: `git2`

### hindsight-tests (library)
Processes test logs (particularly from cargo-nextest).
- Dependencies: `nextest-metadata`

### hindsight-copilot (library)
Processes GitHub Copilot logs and chat sessions.
- Dependencies: `serde_json`, `lsp-types`, `tracing-subscriber`

## Building

```bash
cargo build --workspace
```

## Testing

```bash
cargo test --workspace
```

## Benchmarks

```bash
cargo bench --workspace
```

## Fuzzing

Each crate has a `fuzz/` directory with fuzz targets. To run:

```bash
cd crates/<crate-name>
cargo +nightly fuzz run <target-name>
```

## License

MIT
