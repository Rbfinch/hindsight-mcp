# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Documentation**
  - Added comprehensive doc comments to all public struct fields across all crates
  - Enabled `#![warn(missing_docs)]` lint in all four crate lib.rs files
  - Fixed 25 missing_docs warnings (struct fields in error enums and data types)
  - Restructured README with dedicated Examples section showcasing all MCP tools
  - Added examples for `hindsight_timeline`, `hindsight_search`, `hindsight_activity_summary`, `hindsight_commit_details`, and `hindsight_ingest`
  - Moved test querying examples to Examples section for better discoverability
  - Removed Claude Desktop configuration (VS Code-only focus)
  - Updated fuzzing documentation with specific targets and usage instructions

- **Quality**
  - Replaced placeholder fuzz targets with real implementations
  - Added `fuzz_session_json` and `fuzz_lsp_trace` targets for hindsight-copilot
  - Added `fuzz_nextest_run`, `fuzz_nextest_list`, and `fuzz_streaming_parser` targets for hindsight-tests
  - Removed unused fuzz targets from hindsight-git and hindsight-mcp (not applicable)
  - Added development/scripts/fuzz.sh for running fuzz tests

## [0.1.0] - 2026-01-18

### Added

- **MCP Server** (`hindsight-mcp`)
  - Full MCP protocol implementation with stdio transport
  - Six tools for AI-assisted development:
    - `hindsight_timeline` - Chronological view of development activity
    - `hindsight_search` - Full-text search across commits and messages
    - `hindsight_failing_tests` - Query failing tests with optional commit filter
    - `hindsight_activity_summary` - Aggregate statistics for a time period
    - `hindsight_commit_details` - Detailed commit info with linked test runs
    - `hindsight_ingest` - Trigger data ingestion from git/copilot sources
  - CLI with `ingest` subcommand for piping nextest JSON output
  - SQLite database with FTS5 full-text search
  - Cross-platform database location support (macOS, Linux, Windows)
  - VS Code and Claude Desktop configuration support

- **Git Integration** (`hindsight-git`)
  - Git log parsing using libgit2
  - Commit extraction with author, message, timestamp, and parent refs
  - Diff statistics support

- **Test Integration** (`hindsight-tests`)
  - cargo-nextest JSON output parsing
  - Test run metadata extraction (pass/fail counts, durations)
  - Individual test outcome tracking with output capture
  - Commit linkage for test runs

- **Copilot Integration** (`hindsight-copilot`)
  - GitHub Copilot chat session parsing
  - User prompts and assistant responses extraction
  - Attached files and selections tracking
  - Session timestamp handling

- **Database Layer**
  - SQLite schema with automatic migrations
  - FTS5 full-text search for commits and messages
  - Workspace-scoped queries
  - Efficient indexing for timeline and search queries

- **Documentation**
  - Comprehensive README with quick start guide
  - VS Code MCP configuration examples
  - Claude Desktop configuration examples
  - Complete CLI reference
  - Test ingestion workflow documentation

- **Quality**
  - Property-based tests with proptest
  - Integration tests with real data fixtures
  - Benchmark suite for performance testing
  - Fuzz testing targets for each crate
  - SPDX license headers on all source files

- **CI/CD**
  - GitHub Actions workflow for Linux
  - Build and test jobs with caching
  - Clippy lints and rustfmt checks

### Dependencies

- `rust-mcp-sdk` for MCP protocol
- `rusqlite` with bundled SQLite
- `git2` with bundled libgit2
- `nextest-metadata` for test parsing
- `tokio` async runtime
- `clap` for CLI parsing
- `serde` / `serde_json` for serialization
- `chrono` for date/time handling
- `thiserror` for error types
- `tracing` for structured logging

[Unreleased]: https://github.com/Rbfinch/hindsight-mcp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Rbfinch/hindsight-mcp/releases/tag/v0.1.0
