// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! CLI tests for the --verbose / -v and --quiet / -q flags
//!
//! These tests verify the logging level configuration behavior,
//! including flag interactions and level determination.

use clap::Parser;
use hindsight_mcp::config::Config;
use tracing::Level;

// ============================================================================
// --verbose flag tests
// ============================================================================

#[test]
fn test_verbose_short_flag_v() {
    let config = Config::try_parse_from(["hindsight-mcp", "-v"]).expect("parse should succeed");
    assert!(config.verbose);
    assert!(!config.quiet);
}

#[test]
fn test_verbose_long_flag() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "--verbose"]).expect("parse should succeed");
    assert!(config.verbose);
}

#[test]
fn test_verbose_sets_debug_log_level() {
    let config = Config {
        verbose: true,
        quiet: false,
        ..Default::default()
    };
    assert_eq!(config.log_level(), Level::DEBUG);
}

#[test]
fn test_verbose_flag_value_syntax_not_supported() {
    // Boolean flags with default_value="false" don't support --flag=true syntax
    // They are toggled by presence only
    let result = Config::try_parse_from(["hindsight-mcp", "--verbose=true"]);
    assert!(result.is_err(), "Boolean flags don't support =value syntax");
}

#[test]
fn test_verbose_flag_value_false_not_supported() {
    // Boolean flags don't support --flag=false syntax
    let result = Config::try_parse_from(["hindsight-mcp", "--verbose=false"]);
    assert!(result.is_err(), "Boolean flags don't support =value syntax");
}

// ============================================================================
// --quiet flag tests
// ============================================================================

#[test]
fn test_quiet_short_flag_q() {
    let config = Config::try_parse_from(["hindsight-mcp", "-q"]).expect("parse should succeed");
    assert!(config.quiet);
    assert!(!config.verbose);
}

#[test]
fn test_quiet_long_flag() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "--quiet"]).expect("parse should succeed");
    assert!(config.quiet);
}

#[test]
fn test_quiet_sets_warn_log_level() {
    let config = Config {
        verbose: false,
        quiet: true,
        ..Default::default()
    };
    assert_eq!(config.log_level(), Level::WARN);
}

#[test]
fn test_quiet_flag_value_syntax_not_supported() {
    // Boolean flags don't support =value syntax
    let result = Config::try_parse_from(["hindsight-mcp", "--quiet=true"]);
    assert!(result.is_err(), "Boolean flags don't support =value syntax");
}

#[test]
fn test_quiet_flag_value_false_not_supported() {
    // Boolean flags don't support --flag=false syntax
    let result = Config::try_parse_from(["hindsight-mcp", "--quiet=false"]);
    assert!(result.is_err(), "Boolean flags don't support =value syntax");
}

// ============================================================================
// Default behavior tests
// ============================================================================

#[test]
fn test_default_log_level_is_info() {
    let config = Config::default();
    assert!(!config.verbose);
    assert!(!config.quiet);
    assert_eq!(config.log_level(), Level::INFO);
}

#[test]
fn test_no_flags_means_info_level() {
    let config = Config::try_parse_from(["hindsight-mcp"]).expect("parse should succeed");
    assert!(!config.verbose);
    assert!(!config.quiet);
    assert_eq!(config.log_level(), Level::INFO);
}

// ============================================================================
// Flag interaction tests
// ============================================================================

#[test]
fn test_verbose_and_quiet_both_set_verbose_wins() {
    // When both are set, verbose takes precedence in log_level()
    let config = Config {
        verbose: true,
        quiet: true,
        ..Default::default()
    };
    // According to the implementation, verbose is checked first
    assert_eq!(config.log_level(), Level::DEBUG);
}

#[test]
fn test_verbose_and_quiet_flags_both_parse() {
    // clap allows both flags to be set
    let config =
        Config::try_parse_from(["hindsight-mcp", "-v", "-q"]).expect("parse should succeed");
    assert!(config.verbose);
    assert!(config.quiet);
    // Verbose wins
    assert_eq!(config.log_level(), Level::DEBUG);
}

#[test]
fn test_quiet_then_verbose_verbose_wins() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "-q", "-v"]).expect("parse should succeed");
    assert!(config.verbose);
    assert!(config.quiet);
    assert_eq!(config.log_level(), Level::DEBUG);
}

// ============================================================================
// Logging flags with subcommands
// ============================================================================

#[test]
fn test_verbose_with_ingest_subcommand() {
    let config = Config::try_parse_from(["hindsight-mcp", "-v", "ingest", "--tests"])
        .expect("parse should succeed");
    assert!(config.verbose);
    assert!(config.command.is_some());
}

#[test]
fn test_quiet_with_ingest_subcommand() {
    let config = Config::try_parse_from(["hindsight-mcp", "-q", "ingest", "--tests"])
        .expect("parse should succeed");
    assert!(config.quiet);
}

#[test]
fn test_verbose_with_test_subcommand() {
    let config = Config::try_parse_from(["hindsight-mcp", "--verbose", "test", "-p", "my-crate"])
        .expect("parse should succeed");
    assert!(config.verbose);
}

#[test]
fn test_quiet_with_test_subcommand() {
    let config = Config::try_parse_from(["hindsight-mcp", "--quiet", "test", "-p", "my-crate"])
        .expect("parse should succeed");
    assert!(config.quiet);
}

#[test]
fn test_verbose_after_subcommand_not_global() {
    // Verbose after subcommand should not be recognized as global flag
    let result = Config::try_parse_from(["hindsight-mcp", "test", "--verbose"]);
    match result {
        Ok(config) => {
            assert!(
                !config.verbose,
                "verbose should be false when after subcommand"
            );
        }
        Err(_) => {
            // Rejection is also acceptable
        }
    }
}

// ============================================================================
// Log level method behavior
// ============================================================================

#[test]
fn test_log_level_returns_correct_types() {
    // DEBUG level for verbose
    assert_eq!(
        Config {
            verbose: true,
            ..Default::default()
        }
        .log_level(),
        Level::DEBUG
    );

    // WARN level for quiet
    assert_eq!(
        Config {
            quiet: true,
            ..Default::default()
        }
        .log_level(),
        Level::WARN
    );

    // INFO level for default
    assert_eq!(Config::default().log_level(), Level::INFO);
}

#[test]
fn test_log_levels_are_distinct() {
    let verbose_config = Config {
        verbose: true,
        ..Default::default()
    };
    let quiet_config = Config {
        quiet: true,
        ..Default::default()
    };
    let default_config = Config::default();

    // Just verify the levels are what we expect and are distinct
    assert_eq!(verbose_config.log_level(), Level::DEBUG);
    assert_eq!(default_config.log_level(), Level::INFO);
    assert_eq!(quiet_config.log_level(), Level::WARN);

    // And they're all different from each other
    assert_ne!(verbose_config.log_level(), default_config.log_level());
    assert_ne!(default_config.log_level(), quiet_config.log_level());
    assert_ne!(verbose_config.log_level(), quiet_config.log_level());
}

// ============================================================================
// Combined with other flags
// ============================================================================

#[test]
fn test_verbose_with_database_and_workspace() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "-v", "-d", "/tmp/db.sqlite", "-w", "/tmp"])
            .expect("parse should succeed");

    assert!(config.verbose);
    assert!(config.database.is_some());
    assert!(config.workspace.is_some());
}

#[test]
fn test_quiet_with_database_and_workspace() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "-q", "-d", "/tmp/db.sqlite", "-w", "/tmp"])
            .expect("parse should succeed");

    assert!(config.quiet);
    assert!(config.database.is_some());
    assert!(config.workspace.is_some());
}

#[test]
fn test_all_flags_combined() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-v",
        "-d",
        "/tmp/db.sqlite",
        "-w",
        "/tmp",
        "--skip-init",
    ])
    .expect("parse should succeed");

    assert!(config.verbose);
    assert!(config.database.is_some());
    assert!(config.workspace.is_some());
    assert!(config.skip_init);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_multiple_verbose_flags_conflicts() {
    // clap by default treats repeated flags as conflicts
    let result = Config::try_parse_from(["hindsight-mcp", "-v", "-v", "-v"]);
    assert!(result.is_err(), "Repeated flags should conflict");
}

#[test]
fn test_multiple_quiet_flags_conflicts() {
    // clap by default treats repeated flags as conflicts
    let result = Config::try_parse_from(["hindsight-mcp", "-q", "-q", "-q"]);
    assert!(result.is_err(), "Repeated flags should conflict");
}

#[test]
fn test_verbose_quiet_both_set_once() {
    // -v -q should work (different flags)
    let config =
        Config::try_parse_from(["hindsight-mcp", "-v", "-q"]).expect("parse should succeed");
    assert!(config.verbose);
    assert!(config.quiet);
}
