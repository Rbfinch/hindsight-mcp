# Copilot Instructions for hindsight-mcp

This document provides coding guidelines and context for GitHub Copilot when working in this workspace.

## Project Overview

hindsight-mcp is an MCP (Model Context Protocol) server for AI-assisted coding that leverages development history. It consolidates git logs, test results, and GitHub Copilot logs into a searchable SQLite database, exposed via MCP tool calls.

## Rust Edition and Toolchain

- **Edition**: Rust 2024
- **Minimum Rust Version**: 1.85
- **Resolver**: Workspace resolver version 2

## Workspace Structure

This is a Cargo workspace with multiple crates:

| Crate | Type | Purpose |
|-------|------|---------|
| `hindsight-mcp` | binary | Main MCP server application |
| `hindsight-git` | library | Git log parsing and processing |
| `hindsight-tests` | library | Test result processing (nextest) |
| `hindsight-copilot` | library | Copilot log and session parsing |

## Key Dependencies

| Dependency | Purpose |
|------------|---------|
| `rust-mcp-sdk` | MCP server implementation |
| `rusqlite` | SQLite database (bundled) |
| `git2` | Git repository access |
| `nextest-metadata` | Parse cargo-nextest output |
| `lsp-types` | LSP protocol types for Copilot logs |
| `thiserror` | Error type derivation |
| `tracing` | Structured logging |
| `serde` / `serde_json` | Serialization |
| `chrono` | Date/time handling |

## Common Commands

```bash
cargo build --workspace      # Build all crates
cargo nextest run --workspace  # Run all tests (preferred)
cargo fmt --all --check      # Check formatting
cargo clippy --workspace     # Run lints
```

## Platform Considerations

Supports macOS, Windows, and Linux. Use `std::path::Path` for cross-platform paths.

## Available Skills

Skills are loaded on-demand when relevant to the task. Located in `.github/skills/`:

| Skill | Use When |
|-------|----------|
| `rust-patterns` | Writing/reviewing Rust code, implementing features, error handling |
| `milestone-creator` | Planning multi-phase development work, creating milestone documents |
| `skill-creator` | Creating or updating skills for this workspace |

## Agent Workflow

This workspace includes a multi-agent system for complex development tasks.

| Agent | Role | Purpose |
|-------|------|---------|
| **Orchestrator** | The Conductor | Decomposes tasks, coordinates workflow |
| **Context** | The Retriever | Gathers codebase context |
| **Implementation** | The Coder | Writes code from specifications |
| **Verification** | The Tester | Reviews code, generates tests |
| **Runtime** | The Executor | Runs builds, tests, linters |
| **Commit** | The Archivist | Summarizes and commits to dev |

### Typical Workflow

1. **Orchestrator** decomposes the objective
2. **Context** gathers relevant codebase information
3. **Implementation** writes the code
4. **Verification** reviews and generates tests
5. **Runtime** executes tests and verifies builds
6. **Commit** creates detailed commit with ISO 8601 timestamp and pushes to `dev`

### Agent Files

Located in `.github/agents/`:
- `orchestrator.agent.md`
- `context.agent.md`
- `implementation.agent.md`
- `verification.agent.md`
- `runtime.agent.md`
- `commit.agent.md`

## Development Milestones

Development work is organised into discrete **Milestones**, each containing multiple **Phases**.

For detailed milestone creation guidance, use the `milestone-creator` skill located in `.github/skills/milestone-creator/`.

### Structure

- **Milestone**: A high-level development goal (e.g., "Implement Git Parser")
  - **Phase 1**: Discrete unit of work within the milestone
  - **Phase 2**: Next unit of work
  - ...

### Milestone Files

- Saved to `development/milestones/`
- Filename format: `<ISO-8601-datetime>-<milestone-name>.md`
- Example: `2026-01-17T14-30-00Z-git-parser.md`

### Phase Workflow

Each phase follows this pattern:
1. Work is performed by the relevant agents
2. Phase status is updated in the milestone file
3. Commit agent is called to commit and push to `dev`
4. Next phase begins

### Phase Status Values

- `not-started`: Phase has not begun
- `in-progress`: Currently being worked on
- `completed`: Successfully finished
- `blocked`: Waiting on external dependency or decision
