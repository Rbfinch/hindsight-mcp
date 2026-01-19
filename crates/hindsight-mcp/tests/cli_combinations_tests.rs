// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! CLI tests for flag combinations and environment variable interactions
//!
//! These tests verify complex scenarios involving multiple flags,
//! environment variables, and their interactions with subcommands.

mod test_utils;

use clap::Parser;
use hindsight_mcp::config::Config;
use std::path::PathBuf;
use test_utils::{EnvGuard, TempTestDir};

// ============================================================================
// Short vs long flag equivalence tests
// ============================================================================

#[test]
fn test_short_and_long_database_equivalent() {
    let path = "/tmp/test.db";

    let short = Config::try_parse_from(["hindsight-mcp", "-d", path]).expect("short parse");
    let long = Config::try_parse_from(["hindsight-mcp", "--database", path]).expect("long parse");

    assert_eq!(short.database, long.database);
}

#[test]
fn test_short_and_long_workspace_equivalent() {
    let path = "/tmp";

    let short = Config::try_parse_from(["hindsight-mcp", "-w", path]).expect("short parse");
    let long = Config::try_parse_from(["hindsight-mcp", "--workspace", path]).expect("long parse");

    assert_eq!(short.workspace, long.workspace);
}

#[test]
fn test_short_and_long_verbose_equivalent() {
    let short = Config::try_parse_from(["hindsight-mcp", "-v"]).expect("short parse");
    let long = Config::try_parse_from(["hindsight-mcp", "--verbose"]).expect("long parse");

    assert_eq!(short.verbose, long.verbose);
    assert_eq!(short.log_level(), long.log_level());
}

#[test]
fn test_short_and_long_quiet_equivalent() {
    let short = Config::try_parse_from(["hindsight-mcp", "-q"]).expect("short parse");
    let long = Config::try_parse_from(["hindsight-mcp", "--quiet"]).expect("long parse");

    assert_eq!(short.quiet, long.quiet);
    assert_eq!(short.log_level(), long.log_level());
}

// ============================================================================
// Combined short flags tests
// ============================================================================

#[test]
fn test_combined_vq_flags() {
    // -vq should work
    let config = Config::try_parse_from(["hindsight-mcp", "-vq"]).expect("parse should succeed");
    assert!(config.verbose);
    assert!(config.quiet);
}

#[test]
fn test_combined_qv_flags() {
    // -qv should also work
    let config = Config::try_parse_from(["hindsight-mcp", "-qv"]).expect("parse should succeed");
    assert!(config.verbose);
    assert!(config.quiet);
}

// ============================================================================
// All global flags together
// ============================================================================

#[test]
fn test_all_global_flags_short_form() {
    let temp = TempTestDir::new("all_short");
    let db_path = temp.path().join("db.sqlite");

    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        db_path.to_str().unwrap(),
        "-w",
        temp.path().to_str().unwrap(),
        "-v",
    ])
    .expect("parse should succeed");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
    assert!(config.verbose);
}

#[test]
fn test_all_global_flags_long_form() {
    let temp = TempTestDir::new("all_long");
    let db_path = temp.path().join("db.sqlite");

    let config = Config::try_parse_from([
        "hindsight-mcp",
        "--database",
        db_path.to_str().unwrap(),
        "--workspace",
        temp.path().to_str().unwrap(),
        "--verbose",
        "--skip-init",
    ])
    .expect("parse should succeed");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
    assert!(config.verbose);
    assert!(config.skip_init);
}

#[test]
fn test_all_global_flags_mixed_form() {
    let temp = TempTestDir::new("all_mixed");
    let db_path = temp.path().join("db.sqlite");

    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        db_path.to_str().unwrap(),
        "--workspace",
        temp.path().to_str().unwrap(),
        "-q",
        "--skip-init",
    ])
    .expect("parse should succeed");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
    assert!(config.quiet);
    assert!(config.skip_init);
}

// ============================================================================
// Environment variable + CLI flag combinations
// ============================================================================

#[test]
fn test_env_database_and_cli_workspace() {
    let temp_db = TempTestDir::new("env_db");
    let temp_ws = TempTestDir::new("cli_ws");
    let db_path = temp_db.path().join("env.db");

    let _guard = EnvGuard::set("HINDSIGHT_DATABASE", db_path.to_str().unwrap());

    let config = Config::try_parse_from(["hindsight-mcp", "-w", temp_ws.path().to_str().unwrap()])
        .expect("parse");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(temp_ws.path().to_path_buf()));
}

#[test]
fn test_cli_database_and_env_workspace() {
    let temp_db = TempTestDir::new("cli_db");
    let temp_ws = TempTestDir::new("env_ws");
    let db_path = temp_db.path().join("cli.db");

    let _guard = EnvGuard::set("HINDSIGHT_WORKSPACE", temp_ws.path().to_str().unwrap());

    let config =
        Config::try_parse_from(["hindsight-mcp", "-d", db_path.to_str().unwrap()]).expect("parse");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(temp_ws.path().to_path_buf()));
}

#[test]
fn test_both_env_vars_set() {
    let temp_db = TempTestDir::new("both_env_db");
    let temp_ws = TempTestDir::new("both_env_ws");
    let db_path = temp_db.path().join("env.db");

    let _db_guard = EnvGuard::set("HINDSIGHT_DATABASE", db_path.to_str().unwrap());
    let _ws_guard = EnvGuard::set("HINDSIGHT_WORKSPACE", temp_ws.path().to_str().unwrap());

    let config = Config::try_parse_from(["hindsight-mcp"]).expect("parse");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(temp_ws.path().to_path_buf()));
}

#[test]
fn test_cli_overrides_all_env_vars() {
    let env_db = TempTestDir::new("override_env_db");
    let env_ws = TempTestDir::new("override_env_ws");
    let cli_db = TempTestDir::new("override_cli_db");
    let cli_ws = TempTestDir::new("override_cli_ws");

    let env_db_path = env_db.path().join("env.db");
    let cli_db_path = cli_db.path().join("cli.db");

    let _db_guard = EnvGuard::set("HINDSIGHT_DATABASE", env_db_path.to_str().unwrap());
    let _ws_guard = EnvGuard::set("HINDSIGHT_WORKSPACE", env_ws.path().to_str().unwrap());

    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        cli_db_path.to_str().unwrap(),
        "-w",
        cli_ws.path().to_str().unwrap(),
    ])
    .expect("parse");

    // CLI should override both env vars
    assert_eq!(config.database, Some(cli_db_path));
    assert_eq!(config.workspace, Some(cli_ws.path().to_path_buf()));
}

// ============================================================================
// Flag order doesn't matter
// ============================================================================

#[test]
fn test_flag_order_variation_1() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "-v", "-q", "-d", "/tmp/a.db", "-w", "/tmp"])
            .expect("parse");

    assert!(config.verbose);
    assert!(config.quiet);
    assert!(config.database.is_some());
    assert!(config.workspace.is_some());
}

#[test]
fn test_flag_order_variation_2() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "-w", "/tmp", "-d", "/tmp/b.db", "-v", "-q"])
            .expect("parse");

    assert!(config.verbose);
    assert!(config.quiet);
    assert!(config.database.is_some());
    assert!(config.workspace.is_some());
}

#[test]
fn test_flag_order_variation_3() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "-d", "/tmp/c.db", "-v", "-w", "/tmp", "-q"])
            .expect("parse");

    assert!(config.verbose);
    assert!(config.quiet);
    assert!(config.database.is_some());
    assert!(config.workspace.is_some());
}

// ============================================================================
// Flags with subcommands - complete scenarios
// ============================================================================

#[test]
fn test_all_flags_with_ingest_tests() {
    let temp = TempTestDir::new("full_ingest");
    let db_path = temp.path().join("ingest.db");

    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        db_path.to_str().unwrap(),
        "-w",
        temp.path().to_str().unwrap(),
        "-v",
        "--skip-init",
        "ingest",
        "--tests",
    ])
    .expect("parse");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
    assert!(config.verbose);
    assert!(config.skip_init);
    assert!(config.command.is_some());
}

#[test]
fn test_all_flags_with_test_package() {
    let temp = TempTestDir::new("full_test");
    let db_path = temp.path().join("test.db");

    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        db_path.to_str().unwrap(),
        "-w",
        temp.path().to_str().unwrap(),
        "-q",
        "--skip-init",
        "test",
        "-p",
        "my-crate",
        "--dry-run",
    ])
    .expect("parse");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(temp.path().to_path_buf()));
    assert!(config.quiet);
    assert!(config.skip_init);
    assert!(config.command.is_some());
}

#[test]
fn test_env_vars_with_subcommand() {
    let temp_db = TempTestDir::new("env_subcmd_db");
    let temp_ws = TempTestDir::new("env_subcmd_ws");
    let db_path = temp_db.path().join("env.db");

    let _db_guard = EnvGuard::set("HINDSIGHT_DATABASE", db_path.to_str().unwrap());
    let _ws_guard = EnvGuard::set("HINDSIGHT_WORKSPACE", temp_ws.path().to_str().unwrap());

    let config =
        Config::try_parse_from(["hindsight-mcp", "-v", "test", "-p", "crate"]).expect("parse");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(temp_ws.path().to_path_buf()));
    assert!(config.verbose);
    assert!(config.command.is_some());
}

// ============================================================================
// Validation with combined flags
// ============================================================================

#[test]
fn test_validation_with_all_valid_flags() {
    let temp = TempTestDir::new("valid_combo");
    let db_path = temp.path().join("valid.db");

    let config = Config {
        database: Some(db_path),
        workspace: Some(temp.path().to_path_buf()),
        verbose: true,
        quiet: false,
        skip_init: true,
        command: None,
    };

    let result = config.validate();
    assert!(
        result.is_ok(),
        "All valid flags should validate: {:?}",
        result
    );
}

#[test]
fn test_validation_fails_with_bad_workspace_despite_other_valid_flags() {
    let temp = TempTestDir::new("invalid_ws");
    let db_path = temp.path().join("valid.db");

    let config = Config {
        database: Some(db_path),
        workspace: Some(PathBuf::from("/nonexistent/workspace")),
        verbose: true,
        quiet: false,
        skip_init: true,
        command: None,
    };

    let result = config.validate();
    assert!(result.is_err(), "Bad workspace should fail validation");
}

// ============================================================================
// Default config behavior
// ============================================================================

#[test]
fn test_default_config_all_fields() {
    let config = Config::default();

    assert!(config.command.is_none());
    assert!(config.database.is_none());
    assert!(config.workspace.is_none());
    assert!(!config.verbose);
    assert!(!config.quiet);
    assert!(!config.skip_init);
}

#[test]
fn test_empty_args_gives_default_like_config() {
    let config = Config::try_parse_from(["hindsight-mcp"]).expect("parse");

    assert!(config.command.is_none());
    assert!(config.database.is_none());
    assert!(config.workspace.is_none());
    assert!(!config.verbose);
    assert!(!config.quiet);
    assert!(!config.skip_init);
}

// ============================================================================
// Edge cases with combinations
// ============================================================================

#[test]
fn test_repeated_same_flag_conflicts() {
    // clap by default treats repeated arguments as conflicts
    let result = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        "/first/db.sqlite",
        "-d",
        "/second/db.sqlite",
    ]);

    // This should fail with ArgumentConflict
    assert!(result.is_err(), "Repeated flags should conflict by default");
}

#[test]
fn test_equals_syntax_for_path_flags() {
    let config = Config::try_parse_from(["hindsight-mcp", "-d=/path/to/db.sqlite", "-w=/tmp"])
        .expect("parse");

    assert_eq!(config.database, Some(PathBuf::from("/path/to/db.sqlite")));
    assert_eq!(config.workspace, Some(PathBuf::from("/tmp")));
}

#[test]
fn test_long_equals_syntax() {
    // Note: Boolean flags with default_value="false" don't accept =true/=false syntax
    // They are toggled by presence. Only path-based flags support = syntax.
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "--database=/path/to/db.sqlite",
        "--workspace=/tmp",
        "--verbose",
        "--skip-init",
    ])
    .expect("parse");

    assert_eq!(config.database, Some(PathBuf::from("/path/to/db.sqlite")));
    assert_eq!(config.workspace, Some(PathBuf::from("/tmp")));
    assert!(config.verbose);
    assert!(!config.quiet); // Not set
    assert!(config.skip_init);
}

// ============================================================================
// Help and version flags
// ============================================================================

#[test]
fn test_help_flag_short() {
    let result = Config::try_parse_from(["hindsight-mcp", "-h"]);
    // Help should exit with an error (but a special "help" error)
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
}

#[test]
fn test_help_flag_long() {
    let result = Config::try_parse_from(["hindsight-mcp", "--help"]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
}

#[test]
fn test_version_flag_short() {
    let result = Config::try_parse_from(["hindsight-mcp", "-V"]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
}

#[test]
fn test_version_flag_long() {
    let result = Config::try_parse_from(["hindsight-mcp", "--version"]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
}

// ============================================================================
// Unknown flags should error
// ============================================================================

#[test]
fn test_unknown_short_flag_errors() {
    let result = Config::try_parse_from(["hindsight-mcp", "-x"]);
    assert!(result.is_err());
}

#[test]
fn test_unknown_long_flag_errors() {
    let result = Config::try_parse_from(["hindsight-mcp", "--unknown-flag"]);
    assert!(result.is_err());
}

#[test]
fn test_typo_in_flag_name_errors() {
    let result = Config::try_parse_from(["hindsight-mcp", "--verbos"]); // typo
    assert!(result.is_err());
}
