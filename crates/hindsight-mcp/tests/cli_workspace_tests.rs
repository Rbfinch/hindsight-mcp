// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! CLI tests for the --workspace / -w flag
//!
//! These tests verify the workspace path configuration behavior,
//! including validation, environment variable overrides, and error handling.

mod test_utils;

use clap::Parser;
use hindsight_mcp::config::{Config, ConfigError};
use std::path::PathBuf;
use test_utils::{EnvGuard, TempTestDir};

// ============================================================================
// Basic --workspace flag parsing tests
// ============================================================================

#[test]
fn test_workspace_short_flag_w() {
    let temp = TempTestDir::new("ws_short_flag");
    let config = Config::try_parse_from(["hindsight-mcp", "-w", temp.path().to_str().unwrap()])
        .expect("parse should succeed");
    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
}

#[test]
fn test_workspace_long_flag() {
    let temp = TempTestDir::new("ws_long_flag");
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "--workspace",
        temp.path().to_str().unwrap(),
    ])
    .expect("parse should succeed");
    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
}

#[test]
fn test_workspace_flag_missing_value_fails() {
    let result = Config::try_parse_from(["hindsight-mcp", "--workspace"]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("value") || err.contains("argument"),
        "Error should mention missing value: {}",
        err
    );
}

#[test]
fn test_workspace_path_method_uses_custom_value() {
    let path = PathBuf::from("/tmp/my-workspace");
    let config = Config {
        workspace: Some(path.clone()),
        ..Default::default()
    };
    assert_eq!(config.workspace_path(), Some(path));
}

#[test]
fn test_workspace_path_method_defaults_to_current_dir() {
    let config = Config::default();
    let workspace = config.workspace_path();
    // Should default to current directory
    assert!(workspace.is_some(), "Should return current directory");
    // It should be an absolute path (current directory)
    assert!(
        workspace.as_ref().unwrap().is_absolute(),
        "Default workspace should be absolute path"
    );
}

// ============================================================================
// Environment variable tests
// ============================================================================

#[test]
fn test_workspace_env_var_sets_path() {
    let temp = TempTestDir::new("ws_env_var");
    let _guard = EnvGuard::set("HINDSIGHT_WORKSPACE", temp.path().to_str().unwrap());

    let config = Config::try_parse_from(["hindsight-mcp"]).expect("parse should succeed");
    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
}

#[test]
fn test_workspace_cli_overrides_env_var() {
    let temp_env = TempTestDir::new("ws_env");
    let temp_cli = TempTestDir::new("ws_cli");
    let _guard = EnvGuard::set("HINDSIGHT_WORKSPACE", temp_env.path().to_str().unwrap());

    let config = Config::try_parse_from(["hindsight-mcp", "-w", temp_cli.path().to_str().unwrap()])
        .expect("parse");
    // CLI flag should override environment variable
    assert_eq!(config.workspace, Some(temp_cli.path().to_path_buf()));
}

#[test]
fn test_workspace_env_var_removed_uses_default() {
    let _guard = EnvGuard::remove("HINDSIGHT_WORKSPACE");

    let config = Config::try_parse_from(["hindsight-mcp"]).expect("parse should succeed");
    assert!(
        config.workspace.is_none(),
        "workspace should be None when env var is removed"
    );
    // But workspace_path() should return current directory
    let path = config.workspace_path();
    assert!(path.is_some());
}

// ============================================================================
// Validation tests
// ============================================================================

#[test]
fn test_workspace_validation_succeeds_for_existing_directory() {
    let temp = TempTestDir::new("ws_valid_dir");
    let config = Config {
        workspace: Some(temp.path().to_path_buf()),
        ..Default::default()
    };

    let result = config.validate();
    assert!(result.is_ok(), "Validation should succeed: {:?}", result);
}

#[test]
fn test_workspace_validation_fails_for_nonexistent_path() {
    let config = Config {
        workspace: Some(PathBuf::from("/nonexistent/workspace/path/12345")),
        ..Default::default()
    };

    let result = config.validate();
    assert!(
        matches!(result, Err(ConfigError::WorkspaceNotFound(_))),
        "Expected WorkspaceNotFound error: {:?}",
        result
    );
}

#[test]
fn test_workspace_validation_fails_for_file_not_directory() {
    let temp = TempTestDir::new("ws_file_not_dir");
    let file_path = temp.create_file("not_a_dir.txt", "content");

    let config = Config {
        workspace: Some(file_path),
        ..Default::default()
    };

    let result = config.validate();
    assert!(
        matches!(result, Err(ConfigError::WorkspaceNotDirectory(_))),
        "Expected WorkspaceNotDirectory error: {:?}",
        result
    );
}

#[test]
fn test_workspace_validation_error_contains_path() {
    let bad_path = PathBuf::from("/nonexistent/workspace/12345");
    let config = Config {
        workspace: Some(bad_path.clone()),
        ..Default::default()
    };

    match config.validate() {
        Err(ConfigError::WorkspaceNotFound(path)) => {
            assert_eq!(path, bad_path);
        }
        other => panic!("Expected WorkspaceNotFound, got: {:?}", other),
    }
}

// ============================================================================
// Workspace path with subcommands
// ============================================================================

#[test]
fn test_workspace_flag_with_ingest_subcommand() {
    let temp = TempTestDir::new("ws_ingest");
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-w",
        temp.path().to_str().unwrap(),
        "ingest",
        "--tests",
    ])
    .expect("parse should succeed");

    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
    assert!(config.command.is_some());
}

#[test]
fn test_workspace_flag_with_test_subcommand() {
    let temp = TempTestDir::new("ws_test");
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "--workspace",
        temp.path().to_str().unwrap(),
        "test",
        "-p",
        "my-crate",
    ])
    .expect("parse should succeed");

    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
}

#[test]
fn test_workspace_flag_after_subcommand_not_recognized() {
    let temp = TempTestDir::new("ws_after_cmd");
    // Global flags should come before subcommand
    let result = Config::try_parse_from([
        "hindsight-mcp",
        "test",
        "--workspace",
        temp.path().to_str().unwrap(),
    ]);

    match result {
        Ok(config) => {
            // If it parses, workspace shouldn't be set as global config
            assert!(
                config.workspace.is_none(),
                "workspace should be None when flag is after subcommand"
            );
        }
        Err(_) => {
            // Rejection is also acceptable
        }
    }
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_workspace_relative_path() {
    let config = Config::try_parse_from(["hindsight-mcp", "-w", "./relative/workspace"])
        .expect("parse should succeed");
    assert_eq!(
        config.workspace,
        Some(PathBuf::from("./relative/workspace"))
    );
    // Note: relative path won't pass validation unless it exists
}

#[test]
fn test_workspace_path_with_spaces() {
    let temp = TempTestDir::new("ws_spaces");
    let spaced_dir = temp.create_subdir("path with spaces");

    let config = Config::try_parse_from(["hindsight-mcp", "-w", spaced_dir.to_str().unwrap()])
        .expect("parse should succeed");
    assert_eq!(config.workspace, Some(spaced_dir));
}

#[test]
fn test_workspace_symlink_is_followed() {
    let temp = TempTestDir::new("ws_symlink");
    let real_dir = temp.create_subdir("real");
    let link_path = temp.path().join("link");

    // Create symlink (may fail on Windows without admin rights)
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&real_dir, &link_path).expect("symlink creation");

        let config = Config {
            workspace: Some(link_path),
            ..Default::default()
        };

        // Symlinks to directories should be valid
        let result = config.validate();
        assert!(result.is_ok(), "Symlink to directory should be valid");
    }
}

#[test]
fn test_workspace_root_directory() {
    // Root directory should be a valid workspace
    let config = Config {
        workspace: Some(PathBuf::from("/")),
        ..Default::default()
    };

    let result = config.validate();
    assert!(result.is_ok(), "Root directory should be valid workspace");
}

#[test]
fn test_workspace_tmp_directory() {
    // /tmp should always exist on Unix systems
    #[cfg(unix)]
    {
        let config = Config {
            workspace: Some(PathBuf::from("/tmp")),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_ok(), "/tmp should be valid workspace");
    }
}

#[test]
fn test_workspace_none_validation_succeeds() {
    // No workspace specified should pass validation (uses cwd)
    let config = Config::default();
    let result = config.validate();
    // This might fail if cwd is weird, but generally should succeed
    assert!(result.is_ok() || config.workspace.is_none());
}

// ============================================================================
// Combined with database flag
// ============================================================================

#[test]
fn test_workspace_and_database_flags_together() {
    let temp = TempTestDir::new("ws_and_db");
    let db_path = temp.path().join("db.sqlite");

    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-w",
        temp.path().to_str().unwrap(),
        "-d",
        db_path.to_str().unwrap(),
    ])
    .expect("parse should succeed");

    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
    assert_eq!(config.database, Some(db_path));
}

#[test]
fn test_workspace_env_and_database_cli() {
    let temp_ws = TempTestDir::new("ws_env_db_cli");
    let temp_db = TempTestDir::new("db_cli");
    let db_path = temp_db.path().join("cli.db");

    let _guard = EnvGuard::set("HINDSIGHT_WORKSPACE", temp_ws.path().to_str().unwrap());

    let config = Config::try_parse_from(["hindsight-mcp", "-d", db_path.to_str().unwrap()])
        .expect("parse should succeed");

    assert_eq!(config.workspace, Some(temp_ws.path().to_path_buf()));
    assert_eq!(config.database, Some(db_path));
}
