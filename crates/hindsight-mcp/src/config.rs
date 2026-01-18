// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Configuration for the hindsight-mcp server
//!
//! This module provides configuration types and utilities for the MCP server,
//! including database paths, workspace settings, and logging options.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Hindsight MCP Server - AI-assisted coding with development history
#[derive(Parser, Debug, Clone, Default)]
#[command(name = "hindsight-mcp")]
#[command(version, about, long_about = None)]
#[command(author = "Nicholas D. Crosbie")]
#[command(after_help = "Copyright (c) 2026 - present Nicholas D. Crosbie\nLicense: MIT")]
pub struct Config {
    /// Subcommand to run (defaults to MCP server mode)
    #[command(subcommand)]
    pub command: Option<Command>,
    /// Path to SQLite database file
    ///
    /// If the file doesn't exist, it will be created and initialized.
    /// Defaults to ~/.hindsight/hindsight.db (or platform equivalent).
    #[arg(short, long, env = "HINDSIGHT_DATABASE")]
    pub database: Option<PathBuf>,

    /// Default workspace path for queries
    ///
    /// This is used as the default when tools don't specify a workspace.
    /// Defaults to the current working directory.
    #[arg(short, long, env = "HINDSIGHT_WORKSPACE")]
    pub workspace: Option<PathBuf>,

    /// Enable verbose logging (debug level)
    ///
    /// When enabled, logs detailed request/response information and
    /// debug messages. Logs are written to stderr to avoid interfering
    /// with MCP stdio transport.
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    /// Quiet mode - suppress info-level logs
    ///
    /// Only errors and warnings will be logged.
    #[arg(short, long, default_value = "false")]
    pub quiet: bool,

    /// Skip database initialization/migration check
    ///
    /// Useful for testing or when connecting to an externally managed database.
    #[arg(long, default_value = "false")]
    pub skip_init: bool,
}

/// Available subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Ingest data from various sources
    ///
    /// Use this command to ingest test results from nextest output.
    /// Test output should be piped from stdin.
    ///
    /// Example:
    ///   NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --message-format libtest-json | hindsight-mcp ingest --tests
    Ingest {
        /// Ingest test results from stdin (nextest JSON format)
        #[arg(long)]
        tests: bool,

        /// Git commit SHA to associate with test results
        #[arg(long)]
        commit: Option<String>,
    },

    /// Run tests and ingest results in one command
    ///
    /// This command wraps cargo-nextest, runs your tests, and automatically
    /// ingests the results into the hindsight database. It auto-detects the
    /// workspace and current git commit.
    ///
    /// Examples:
    ///   hindsight-mcp test                    # Run all tests and ingest
    ///   hindsight-mcp test -p my-crate        # Test specific package
    ///   hindsight-mcp test --dry-run          # Preview without ingesting
    ///   hindsight-mcp test --stdin            # Read from stdin (CI mode)
    #[command(after_help = "Requires cargo-nextest to be installed.\n\
        Install with: cargo install cargo-nextest")]
    Test {
        /// Package(s) to test (passed to cargo nextest --package)
        #[arg(short, long)]
        package: Vec<String>,

        /// Test binary(ies) to run (passed to cargo nextest --bin)
        #[arg(long)]
        bin: Vec<String>,

        /// Run only tests matching this filter expression
        #[arg(short = 'E', long)]
        filter: Option<String>,

        /// Read nextest JSON from stdin instead of running tests
        ///
        /// Use this for custom nextest invocations or CI pipelines.
        /// When set, cargo-nextest is not spawned; JSON is read from stdin.
        #[arg(long)]
        stdin: bool,

        /// Don't actually ingest - just show what would be ingested
        ///
        /// Runs tests and parses output, but does not write to the database.
        #[arg(long)]
        dry_run: bool,

        /// Don't auto-detect and link to current git commit
        #[arg(long, conflicts_with = "commit")]
        no_commit: bool,

        /// Explicit commit SHA to associate with test run
        ///
        /// Overrides auto-detection of the current git HEAD.
        #[arg(long, conflicts_with = "no_commit")]
        commit: Option<String>,

        /// Show test output in terminal
        ///
        /// By default, test output is suppressed. Use this flag to see
        /// test progress and output in real-time.
        #[arg(long)]
        show_output: bool,

        /// Additional arguments passed to cargo nextest
        ///
        /// Everything after `--` is passed through to nextest.
        #[arg(last = true)]
        nextest_args: Vec<String>,
    },
}

impl Config {
    /// Get the database path, using a default if not specified
    ///
    /// Default location is platform-specific:
    /// - macOS: ~/Library/Application Support/hindsight/hindsight.db
    /// - Linux: ~/.local/share/hindsight/hindsight.db
    /// - Windows: %LOCALAPPDATA%\hindsight\hindsight.db
    #[must_use]
    pub fn database_path(&self) -> PathBuf {
        self.database.clone().unwrap_or_else(|| {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("hindsight")
                .join("hindsight.db")
        })
    }

    /// Get the workspace path, using current directory as default
    ///
    /// Returns `None` if no workspace is specified and the current
    /// directory cannot be determined.
    #[must_use]
    pub fn workspace_path(&self) -> Option<PathBuf> {
        self.workspace
            .clone()
            .or_else(|| std::env::current_dir().ok())
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The workspace path is specified but doesn't exist
    /// - The database parent directory cannot be created
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate workspace if specified
        if let Some(ref workspace) = self.workspace {
            if !workspace.exists() {
                return Err(ConfigError::WorkspaceNotFound(workspace.clone()));
            }
            if !workspace.is_dir() {
                return Err(ConfigError::WorkspaceNotDirectory(workspace.clone()));
            }
        }

        // Validate database path is writable (check parent exists or can be created)
        let db_path = self.database_path();
        if let Some(parent) = db_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::DatabaseDirectoryCreateFailed(parent.to_path_buf(), e))?;
        }

        Ok(())
    }

    /// Get the log level based on verbose/quiet flags
    #[must_use]
    pub fn log_level(&self) -> tracing::Level {
        if self.verbose {
            tracing::Level::DEBUG
        } else if self.quiet {
            tracing::Level::WARN
        } else {
            tracing::Level::INFO
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Workspace path not found
    #[error("Workspace path not found: {0}")]
    WorkspaceNotFound(PathBuf),

    /// Workspace path is not a directory
    #[error("Workspace path is not a directory: {0}")]
    WorkspaceNotDirectory(PathBuf),

    /// Failed to create database directory
    #[error("Failed to create database directory {0}: {1}")]
    DatabaseDirectoryCreateFailed(PathBuf, std::io::Error),

    /// Database initialization failed
    #[error("Database initialization failed: {0}")]
    DatabaseInitFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.command.is_none());
        assert!(config.database.is_none());
        assert!(config.workspace.is_none());
        assert!(!config.verbose);
        assert!(!config.quiet);
        assert!(!config.skip_init);
    }

    #[test]
    fn test_database_path_default() {
        let config = Config::default();
        let path = config.database_path();
        assert!(path.to_string_lossy().contains("hindsight"));
    }

    #[test]
    fn test_database_path_custom() {
        let custom = PathBuf::from("/custom/path/db.sqlite");
        let config = Config {
            database: Some(custom.clone()),
            ..Default::default()
        };
        assert_eq!(config.database_path(), custom);
    }

    #[test]
    fn test_workspace_path_default() {
        let config = Config::default();
        // Should fallback to current directory
        let workspace = config.workspace_path();
        assert!(workspace.is_some());
    }

    #[test]
    fn test_workspace_path_custom() {
        let custom = PathBuf::from("/tmp");
        let config = Config {
            workspace: Some(custom.clone()),
            ..Default::default()
        };
        assert_eq!(config.workspace_path(), Some(custom));
    }

    #[test]
    fn test_log_level_default() {
        let config = Config::default();
        assert_eq!(config.log_level(), tracing::Level::INFO);
    }

    #[test]
    fn test_log_level_verbose() {
        let config = Config {
            verbose: true,
            ..Default::default()
        };
        assert_eq!(config.log_level(), tracing::Level::DEBUG);
    }

    #[test]
    fn test_log_level_quiet() {
        let config = Config {
            quiet: true,
            ..Default::default()
        };
        assert_eq!(config.log_level(), tracing::Level::WARN);
    }

    #[test]
    fn test_validate_nonexistent_workspace() {
        let config = Config {
            workspace: Some(PathBuf::from("/nonexistent/path/12345")),
            ..Default::default()
        };
        let result = config.validate();
        assert!(matches!(result, Err(ConfigError::WorkspaceNotFound(_))));
    }

    #[test]
    fn test_validate_valid_workspace() {
        let config = Config {
            workspace: Some(PathBuf::from("/tmp")),
            ..Default::default()
        };
        // This should succeed on most systems
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Config::command().debug_assert();
    }

    // ========================================================================
    // Test subcommand CLI parsing tests
    // ========================================================================

    #[test]
    fn test_parse_test_command_minimal() {
        let config = Config::try_parse_from(["hindsight-mcp", "test"]).expect("parse");
        match config.command {
            Some(Command::Test {
                package,
                bin,
                filter,
                stdin,
                dry_run,
                no_commit,
                commit,
                show_output,
                nextest_args,
            }) => {
                assert!(package.is_empty());
                assert!(bin.is_empty());
                assert!(filter.is_none());
                assert!(!stdin);
                assert!(!dry_run);
                assert!(!no_commit);
                assert!(commit.is_none());
                assert!(!show_output);
                assert!(nextest_args.is_empty());
            }
            _ => panic!("Expected Test command"),
        }
    }

    #[test]
    fn test_parse_test_command_with_package() {
        let config =
            Config::try_parse_from(["hindsight-mcp", "test", "-p", "my-crate"]).expect("parse");
        match config.command {
            Some(Command::Test { package, .. }) => {
                assert_eq!(package, vec!["my-crate"]);
            }
            _ => panic!("Expected Test command"),
        }
    }

    #[test]
    fn test_parse_test_command_with_multiple_packages() {
        let config =
            Config::try_parse_from(["hindsight-mcp", "test", "-p", "crate-a", "-p", "crate-b"])
                .expect("parse");
        match config.command {
            Some(Command::Test { package, .. }) => {
                assert_eq!(package, vec!["crate-a", "crate-b"]);
            }
            _ => panic!("Expected Test command"),
        }
    }

    #[test]
    fn test_parse_test_command_with_flags() {
        let config = Config::try_parse_from([
            "hindsight-mcp",
            "test",
            "--dry-run",
            "--show-output",
            "--stdin",
        ])
        .expect("parse");
        match config.command {
            Some(Command::Test {
                dry_run,
                show_output,
                stdin,
                ..
            }) => {
                assert!(dry_run);
                assert!(show_output);
                assert!(stdin);
            }
            _ => panic!("Expected Test command"),
        }
    }

    #[test]
    fn test_parse_test_command_with_commit() {
        let config =
            Config::try_parse_from(["hindsight-mcp", "test", "--commit", "abc123"]).expect("parse");
        match config.command {
            Some(Command::Test { commit, .. }) => {
                assert_eq!(commit, Some("abc123".to_string()));
            }
            _ => panic!("Expected Test command"),
        }
    }

    #[test]
    fn test_parse_test_command_with_no_commit() {
        let config =
            Config::try_parse_from(["hindsight-mcp", "test", "--no-commit"]).expect("parse");
        match config.command {
            Some(Command::Test { no_commit, .. }) => {
                assert!(no_commit);
            }
            _ => panic!("Expected Test command"),
        }
    }

    #[test]
    fn test_parse_test_command_commit_conflicts_with_no_commit() {
        // --commit and --no-commit should conflict
        let result =
            Config::try_parse_from(["hindsight-mcp", "test", "--commit", "abc123", "--no-commit"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_test_command_with_filter() {
        let config = Config::try_parse_from(["hindsight-mcp", "test", "-E", "test(/integration/)"])
            .expect("parse");
        match config.command {
            Some(Command::Test { filter, .. }) => {
                assert_eq!(filter, Some("test(/integration/)".to_string()));
            }
            _ => panic!("Expected Test command"),
        }
    }

    #[test]
    fn test_parse_test_command_with_passthrough_args() {
        let config = Config::try_parse_from([
            "hindsight-mcp",
            "test",
            "-p",
            "my-crate",
            "--",
            "--retries",
            "3",
            "--no-fail-fast",
        ])
        .expect("parse");
        match config.command {
            Some(Command::Test {
                package,
                nextest_args,
                ..
            }) => {
                assert_eq!(package, vec!["my-crate"]);
                assert_eq!(nextest_args, vec!["--retries", "3", "--no-fail-fast"]);
            }
            _ => panic!("Expected Test command"),
        }
    }

    #[test]
    fn test_parse_test_command_with_bin() {
        let config =
            Config::try_parse_from(["hindsight-mcp", "test", "--bin", "my-bin"]).expect("parse");
        match config.command {
            Some(Command::Test { bin, .. }) => {
                assert_eq!(bin, vec!["my-bin"]);
            }
            _ => panic!("Expected Test command"),
        }
    }
}
