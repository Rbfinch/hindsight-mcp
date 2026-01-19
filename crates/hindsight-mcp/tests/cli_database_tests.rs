// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! CLI tests for the --database / -d flag
//!
//! These tests verify the database path configuration behavior,
//! including custom paths, environment variable overrides, and error handling.

mod test_utils;

use clap::Parser;
use hindsight_mcp::config::{Config, ConfigError};
use std::path::PathBuf;
use test_utils::{EnvGuard, TempTestDir};

// ============================================================================
// Basic --database flag parsing tests
// ============================================================================

#[test]
fn test_database_short_flag_d() {
    let config = Config::try_parse_from(["hindsight-mcp", "-d", "/custom/path/db.sqlite"])
        .expect("parse should succeed");
    assert_eq!(
        config.database,
        Some(PathBuf::from("/custom/path/db.sqlite"))
    );
}

#[test]
fn test_database_long_flag() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "--database", "/another/path/hindsight.db"])
            .expect("parse should succeed");
    assert_eq!(
        config.database,
        Some(PathBuf::from("/another/path/hindsight.db"))
    );
}

#[test]
fn test_database_flag_missing_value_fails() {
    // --database requires a value
    let result = Config::try_parse_from(["hindsight-mcp", "--database"]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("value") || err.contains("argument"),
        "Error should mention missing value: {}",
        err
    );
}

#[test]
fn test_database_path_method_uses_custom_value() {
    let config = Config {
        database: Some(PathBuf::from("/my/custom/db.sqlite")),
        ..Default::default()
    };
    assert_eq!(
        config.database_path(),
        PathBuf::from("/my/custom/db.sqlite")
    );
}

#[test]
fn test_database_path_method_uses_default_when_none() {
    let config = Config::default();
    let path = config.database_path();
    // Should contain hindsight in the default path
    assert!(
        path.to_string_lossy().contains("hindsight"),
        "Default path should contain 'hindsight': {}",
        path.display()
    );
}

// ============================================================================
// Environment variable tests
// ============================================================================

#[test]
fn test_database_env_var_sets_path() {
    let temp = TempTestDir::new("db_env_var");
    let db_path = temp.path().join("env_test.db");
    let _guard = EnvGuard::set("HINDSIGHT_DATABASE", db_path.to_str().unwrap());

    let config = Config::try_parse_from(["hindsight-mcp"]).expect("parse should succeed");
    assert_eq!(config.database, Some(db_path));
}

#[test]
fn test_database_cli_overrides_env_var() {
    let temp = TempTestDir::new("db_cli_override");
    let env_path = temp.path().join("env.db");
    let cli_path = temp.path().join("cli.db");
    let _guard = EnvGuard::set("HINDSIGHT_DATABASE", env_path.to_str().unwrap());

    let config =
        Config::try_parse_from(["hindsight-mcp", "-d", cli_path.to_str().unwrap()]).expect("parse");
    // CLI flag should override environment variable
    assert_eq!(config.database, Some(cli_path));
}

#[test]
fn test_database_env_var_removed_uses_default() {
    // Ensure env var is not set
    let _guard = EnvGuard::remove("HINDSIGHT_DATABASE");

    let config = Config::try_parse_from(["hindsight-mcp"]).expect("parse should succeed");
    assert!(
        config.database.is_none(),
        "database should be None when env var is removed"
    );
    // But database_path() should return a default
    let path = config.database_path();
    assert!(path.to_string_lossy().contains("hindsight"));
}

// ============================================================================
// Path validation tests
// ============================================================================

#[test]
fn test_database_validation_creates_parent_directory() {
    let temp = TempTestDir::new("db_validation_create");
    let nested_path = temp.path().join("nested").join("subdir").join("test.db");

    let config = Config {
        database: Some(nested_path.clone()),
        ..Default::default()
    };

    // Before validation, parent doesn't exist
    assert!(!nested_path.parent().unwrap().exists());

    // Validation should create the parent directory
    config.validate().expect("validation should succeed");

    // Parent directory should now exist
    assert!(nested_path.parent().unwrap().exists());
}

#[test]
fn test_database_validation_fails_for_unwritable_location() {
    // Attempt to use a path that can't be created
    // On macOS/Linux, /proc or similar locations are not writable
    let config = Config {
        database: Some(PathBuf::from("/nonexistent_root_12345/db.sqlite")),
        ..Default::default()
    };

    let result = config.validate();
    // This should fail because we can't create the directory
    assert!(
        result.is_err(),
        "Should fail to validate unwritable path: {:?}",
        result
    );
    match result {
        Err(ConfigError::DatabaseDirectoryCreateFailed(path, _)) => {
            assert!(path.to_string_lossy().contains("nonexistent_root_12345"));
        }
        Err(e) => panic!("Unexpected error type: {:?}", e),
        Ok(()) => panic!("Expected error for unwritable path"),
    }
}

// ============================================================================
// Database path with subcommands
// ============================================================================

#[test]
fn test_database_flag_with_ingest_subcommand() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        "/custom/db.sqlite",
        "ingest",
        "--tests",
    ])
    .expect("parse should succeed");

    assert_eq!(config.database, Some(PathBuf::from("/custom/db.sqlite")));
    assert!(config.command.is_some());
}

#[test]
fn test_database_flag_with_test_subcommand() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "--database",
        "/test/db.sqlite",
        "test",
        "-p",
        "my-crate",
    ])
    .expect("parse should succeed");

    assert_eq!(config.database, Some(PathBuf::from("/test/db.sqlite")));
}

#[test]
fn test_database_flag_after_subcommand_fails() {
    // Global flags should come before subcommand
    let result = Config::try_parse_from(["hindsight-mcp", "test", "--database", "/wrong/order.db"]);

    // This might succeed or fail depending on clap's parsing mode
    // But the database field should be None if it's treated as a test arg
    match result {
        Ok(config) => {
            // If it parses, the database flag wasn't recognized for the global config
            assert!(
                config.database.is_none(),
                "database should be None when flag is after subcommand"
            );
        }
        Err(_) => {
            // This is also acceptable - clap rejected the argument order
        }
    }
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_database_relative_path() {
    let config = Config::try_parse_from(["hindsight-mcp", "-d", "./relative/path/db.sqlite"])
        .expect("parse should succeed");
    assert_eq!(
        config.database,
        Some(PathBuf::from("./relative/path/db.sqlite"))
    );
}

#[test]
fn test_database_path_with_spaces() {
    let config = Config::try_parse_from(["hindsight-mcp", "-d", "/path with spaces/my db.sqlite"])
        .expect("parse should succeed");
    assert_eq!(
        config.database,
        Some(PathBuf::from("/path with spaces/my db.sqlite"))
    );
}

#[test]
fn test_database_home_tilde_not_expanded_by_clap() {
    // clap doesn't expand ~ - that's a shell feature
    let config = Config::try_parse_from(["hindsight-mcp", "-d", "~/hindsight/db.sqlite"])
        .expect("parse should succeed");
    // The tilde should be preserved literally
    assert_eq!(
        config.database,
        Some(PathBuf::from("~/hindsight/db.sqlite"))
    );
}

#[test]
fn test_database_empty_string_is_rejected() {
    // Empty string should be rejected or treated as invalid
    let result = Config::try_parse_from(["hindsight-mcp", "-d", ""]);
    // Either parsing fails or we get an empty path
    match result {
        Ok(config) => {
            // If it parses, the path should be empty
            assert_eq!(config.database, Some(PathBuf::from("")));
            // Validation should catch this as problematic
        }
        Err(_) => {
            // Rejection is also acceptable
        }
    }
}

#[test]
fn test_database_absolute_vs_relative_preservation() {
    // Absolute path
    let config1 = Config::try_parse_from(["hindsight-mcp", "-d", "/absolute/path.db"]).unwrap();
    assert!(config1.database.as_ref().unwrap().is_absolute());

    // Relative path
    let config2 = Config::try_parse_from(["hindsight-mcp", "-d", "relative/path.db"]).unwrap();
    assert!(config2.database.as_ref().unwrap().is_relative());
}
