// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! CLI tests for the --skip-init flag
//!
//! These tests verify the skip-init flag behavior for bypassing
//! database initialization during startup.

use clap::Parser;
use hindsight_mcp::config::Config;

// ============================================================================
// Basic --skip-init flag parsing tests
// ============================================================================

#[test]
fn test_skip_init_long_flag() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "--skip-init"]).expect("parse should succeed");
    assert!(config.skip_init);
}

#[test]
fn test_skip_init_default_is_false() {
    let config = Config::default();
    assert!(!config.skip_init);
}

#[test]
fn test_skip_init_not_set_parses_as_false() {
    let config = Config::try_parse_from(["hindsight-mcp"]).expect("parse should succeed");
    assert!(!config.skip_init);
}

#[test]
fn test_skip_init_value_syntax_not_supported() {
    // Boolean flags don't support =value syntax in clap by default
    let result = Config::try_parse_from(["hindsight-mcp", "--skip-init=true"]);
    assert!(result.is_err(), "Boolean flags don't support =value syntax");
}

#[test]
fn test_skip_init_value_false_not_supported() {
    // Boolean flags don't support --flag=false syntax
    let result = Config::try_parse_from(["hindsight-mcp", "--skip-init=false"]);
    assert!(result.is_err(), "Boolean flags don't support =value syntax");
}

// ============================================================================
// No short flag for --skip-init
// ============================================================================

#[test]
fn test_skip_init_has_no_short_flag() {
    // There is no -s or similar short flag for --skip-init
    // Trying to use -s should fail or be unrecognized
    let result = Config::try_parse_from(["hindsight-mcp", "-s"]);
    // This should fail because -s is not defined
    assert!(result.is_err(), "Expected -s to fail as unrecognized");
}

// ============================================================================
// --skip-init with subcommands
// ============================================================================

#[test]
fn test_skip_init_with_ingest_subcommand() {
    let config = Config::try_parse_from(["hindsight-mcp", "--skip-init", "ingest", "--tests"])
        .expect("parse should succeed");
    assert!(config.skip_init);
    assert!(config.command.is_some());
}

#[test]
fn test_skip_init_with_test_subcommand() {
    let config = Config::try_parse_from(["hindsight-mcp", "--skip-init", "test", "-p", "my-crate"])
        .expect("parse should succeed");
    assert!(config.skip_init);
}

#[test]
fn test_skip_init_after_subcommand_not_recognized() {
    // Global flags should come before subcommand
    let result = Config::try_parse_from(["hindsight-mcp", "test", "--skip-init"]);
    match result {
        Ok(config) => {
            // If it parses, skip_init shouldn't be set
            assert!(
                !config.skip_init,
                "skip_init should be false when after subcommand"
            );
        }
        Err(_) => {
            // Rejection is also acceptable
        }
    }
}

// ============================================================================
// --skip-init combined with other flags
// ============================================================================

#[test]
fn test_skip_init_with_database() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "--skip-init", "-d", "/path/to/db.sqlite"])
            .expect("parse should succeed");
    assert!(config.skip_init);
    assert!(config.database.is_some());
}

#[test]
fn test_skip_init_with_workspace() {
    let config = Config::try_parse_from(["hindsight-mcp", "--skip-init", "-w", "/tmp"])
        .expect("parse should succeed");
    assert!(config.skip_init);
    assert!(config.workspace.is_some());
}

#[test]
fn test_skip_init_with_verbose() {
    let config = Config::try_parse_from(["hindsight-mcp", "--skip-init", "-v"])
        .expect("parse should succeed");
    assert!(config.skip_init);
    assert!(config.verbose);
}

#[test]
fn test_skip_init_with_quiet() {
    let config = Config::try_parse_from(["hindsight-mcp", "--skip-init", "-q"])
        .expect("parse should succeed");
    assert!(config.skip_init);
    assert!(config.quiet);
}

#[test]
fn test_skip_init_with_all_global_flags() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "--skip-init",
        "-v",
        "-d",
        "/tmp/db.sqlite",
        "-w",
        "/tmp",
    ])
    .expect("parse should succeed");

    assert!(config.skip_init);
    assert!(config.verbose);
    assert!(config.database.is_some());
    assert!(config.workspace.is_some());
}

// ============================================================================
// --skip-init order variations
// ============================================================================

#[test]
fn test_skip_init_first_among_flags() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "--skip-init", "-v", "-d", "/tmp/db.sqlite"])
            .expect("parse should succeed");
    assert!(config.skip_init);
}

#[test]
fn test_skip_init_last_among_flags() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "-v", "-d", "/tmp/db.sqlite", "--skip-init"])
            .expect("parse should succeed");
    assert!(config.skip_init);
}

#[test]
fn test_skip_init_middle_of_flags() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "-v", "--skip-init", "-d", "/tmp/db.sqlite"])
            .expect("parse should succeed");
    assert!(config.skip_init);
}

// ============================================================================
// --skip-init with full command scenarios
// ============================================================================

#[test]
fn test_skip_init_full_ingest_command() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        "/tmp/test.db",
        "-w",
        "/my/workspace",
        "--skip-init",
        "-v",
        "ingest",
        "--tests",
    ])
    .expect("parse should succeed");

    assert!(config.skip_init);
    assert!(config.verbose);
    assert!(!config.quiet);
    assert!(config.database.is_some());
    assert!(config.workspace.is_some());
    assert!(config.command.is_some());
}

#[test]
fn test_skip_init_full_test_command() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        "/tmp/test.db",
        "--skip-init",
        "-q",
        "test",
        "-p",
        "my-crate",
        "--dry-run",
    ])
    .expect("parse should succeed");

    assert!(config.skip_init);
    assert!(!config.verbose);
    assert!(config.quiet);
    assert!(config.database.is_some());
    assert!(config.command.is_some());
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_skip_init_repeated_conflicts() {
    // Repeated boolean flags conflict by default in clap
    let result = Config::try_parse_from(["hindsight-mcp", "--skip-init", "--skip-init"]);
    // This might succeed (just setting true twice) or fail depending on clap config
    // For flags with default_value, repeating may work but is not typical usage
    // We test that at least it doesn't panic
    if let Ok(config) = result {
        assert!(config.skip_init);
    }
    // Err is also acceptable
}

#[test]
fn test_skip_init_value_syntax_conflicts() {
    // =true/=false syntax not supported, so this should error
    let result = Config::try_parse_from(["hindsight-mcp", "--skip-init=true", "--skip-init=false"]);
    assert!(result.is_err());
}

#[test]
fn test_skip_init_value_syntax_conflicts_reverse() {
    // =false/=true syntax not supported either
    let result = Config::try_parse_from(["hindsight-mcp", "--skip-init=false", "--skip-init=true"]);
    assert!(result.is_err());
}

// ============================================================================
// Semantic tests (what skip-init means)
// ============================================================================

#[test]
fn test_skip_init_does_not_affect_validation() {
    // skip_init only affects database initialization, not config validation
    let config = Config {
        skip_init: true,
        workspace: Some(std::path::PathBuf::from("/nonexistent/path")),
        ..Default::default()
    };

    // Validation should still check workspace
    let result = config.validate();
    assert!(
        result.is_err(),
        "Validation should still fail for bad workspace"
    );
}

#[test]
fn test_skip_init_preserves_database_path() {
    // skip_init should not affect how database_path() works
    let db_path = std::path::PathBuf::from("/custom/db.sqlite");
    let config = Config {
        skip_init: true,
        database: Some(db_path.clone()),
        ..Default::default()
    };

    assert_eq!(config.database_path(), db_path);
}

#[test]
fn test_skip_init_preserves_workspace_path() {
    // skip_init should not affect how workspace_path() works
    let ws_path = std::path::PathBuf::from("/custom/workspace");
    let config = Config {
        skip_init: true,
        workspace: Some(ws_path.clone()),
        ..Default::default()
    };

    assert_eq!(config.workspace_path(), Some(ws_path));
}

#[test]
fn test_skip_init_preserves_log_level() {
    // skip_init should not affect log_level()
    let config = Config {
        skip_init: true,
        verbose: true,
        ..Default::default()
    };

    assert_eq!(config.log_level(), tracing::Level::DEBUG);
}
