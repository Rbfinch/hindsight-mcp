# Copilot Instructions for hindsight-mcp

This document provides coding guidelines and context for GitHub Copilot when working in this workspace.

## Project Overview

hindsight-mcp is an MCP (Model Context Protocol) server for AI-assisted coding that leverages development history. It consolidates git logs, test results, and GitHub Copilot logs into a searchable SQLite database, exposed via MCP tool calls.

## Rust Edition and Toolchain

- **Edition**: Rust 2024
- **Minimum Rust Version**: 1.85
- **Resolver**: Workspace resolver version 2

Always use Rust 2024 edition idioms and features. This includes:
- The new `gen` keyword for generators (when stabilized)
- Updated prelude with `Future` and `IntoFuture`
- New lifetime capture rules in `impl Trait`
- RPIT (return position impl trait) lifetime capture changes

## Workspace Structure

This is a Cargo workspace with multiple crates:

| Crate | Type | Purpose |
|-------|------|---------|
| `hindsight-mcp` | binary | Main MCP server application |
| `hindsight-git` | library | Git log parsing and processing |
| `hindsight-tests` | library | Test result processing (nextest) |
| `hindsight-copilot` | library | Copilot log and session parsing |

## Coding Conventions

### Error Handling

- Use `thiserror` for defining error types
- Each crate has its own `error.rs` module with a crate-specific error enum
- Use structured error variants with named fields for context:

```rust
#[derive(Debug, Error)]
pub enum MyError {
    #[error("Description: {field_name}")]
    VariantName { field_name: String },
    
    #[error("Wrapped error: {0}")]
    Wrapped(#[from] OtherError),
}
```

### Module Organization

- Each crate follows a consistent structure:
  - `src/lib.rs` or `src/main.rs` - entry point with module declarations
  - `src/error.rs` - error types
  - Feature-specific modules (e.g., `parser.rs`, `commit.rs`)
- Use `pub mod` for public modules, re-export key types from `lib.rs`
- Provide a `prelude` module for commonly used types when appropriate

### Documentation

- All public items must have doc comments (`///` or `//!`)
- Module-level documentation using `//!` at the top of each file
- Include usage examples in doc comments for public APIs
- Use `# Examples` sections in doc comments

### Testing

- Unit tests go in the same file with `#[cfg(test)]` module
- Integration tests in `tests/integration_tests.rs`
- Benchmarks in `benches/<crate>_bench.rs` using Criterion
- Fuzz tests in `fuzz/` directory using cargo-fuzz
- Use `similar-asserts` for readable diff output in test failures
- Use `proptest` for property-based testing where appropriate

### Dependencies

- All shared dependencies are defined in the workspace root `Cargo.toml`
- Use `workspace = true` in crate `Cargo.toml` files to reference workspace dependencies
- Prefer workspace-level dependency management for version consistency

### Logging and Tracing

- Use `tracing` crate for structured logging
- Use `tracing-subscriber` with `env-filter` for runtime log level control
- Log levels:
  - `error!` - Unrecoverable errors
  - `warn!` - Recoverable issues, deprecated usage
  - `info!` - High-level operational information
  - `debug!` - Detailed diagnostic information
  - `trace!` - Very detailed tracing

### Serialization

- Use `serde` with `derive` feature for serialization
- Use `serde_json` for JSON parsing
- Prefer strongly-typed deserialization over dynamic `Value` types

### Async Code

- The MCP server uses async Rust via `rust-mcp-sdk`
- Use `async`/`await` syntax consistently
- Avoid mixing blocking and async code without proper handling

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
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run benchmarks
cargo bench --workspace

# Run a specific crate's fuzz target
cd crates/<crate-name>
cargo +nightly fuzz run <target-name>

# Check formatting
cargo fmt --all --check

# Run clippy lints
cargo clippy --workspace --all-targets
```

## Best Practices

1. **Prefer iterators** over explicit loops where it improves clarity
2. **Use `?` operator** for error propagation instead of explicit matching
3. **Avoid `unwrap()`** in library code; use `expect()` with context or proper error handling
4. **Use `impl Trait`** in return position for cleaner APIs
5. **Prefer `&str`** over `String` in function parameters when ownership isn't needed
6. **Use `#[must_use]`** on functions that return important values
7. **Keep functions small** and focused on a single responsibility
8. **Use type aliases** to improve readability of complex types

## Platform Considerations

The project supports macOS, Windows, and Linux. Be mindful of:
- File path separators (use `std::path::Path` abstractions)
- Platform-specific storage locations (see ARCHITECTURE.md for Copilot log paths)
- Line endings (use `.gitattributes` if needed)

## Code Review Checklist

When generating or reviewing code, ensure:
- [ ] All public items have documentation
- [ ] Error types use `thiserror` with descriptive messages
- [ ] Tests are included for new functionality
- [ ] No `unwrap()` in library code (binary entry points are acceptable)
- [ ] Logging uses appropriate `tracing` macros and levels
- [ ] Dependencies use workspace versions

## Agent Workflow

This workspace includes a multi-agent system for complex development tasks.

### Available Agents

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
