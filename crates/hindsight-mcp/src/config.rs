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
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ConfigError::DatabaseDirectoryCreateFailed(parent.to_path_buf(), e)
                })?;
            }
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
}
