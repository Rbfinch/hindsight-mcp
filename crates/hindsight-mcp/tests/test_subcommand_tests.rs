// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Tests for the `test` subcommand
//!
//! This module tests:
//! - Basic test command parsing
//! - Package/binary filtering flags
//! - Filter expression flag
//! - Stdin mode flag
//! - Dry-run mode flag
//! - Commit linking flags
//! - Output control flags
//! - Passthrough arguments

mod fixtures;
mod test_utils;

use clap::Parser;
use fixtures::test_database;
use hindsight_mcp::config::{Command, Config};
use hindsight_mcp::ingest::Ingestor;
use test_utils::{TempTestDir, sample_nextest_json};

// ============================================================================
// Basic Test Command Parsing
// ============================================================================

#[test]
fn test_subcommand_test_parses() {
    let config = Config::try_parse_from(["hindsight-mcp", "test"]).expect("parse should succeed");
    assert!(matches!(config.command, Some(Command::Test { .. })));
}

#[test]
fn test_subcommand_test_default_values() {
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
            assert!(package.is_empty(), "package should be empty by default");
            assert!(bin.is_empty(), "bin should be empty by default");
            assert!(filter.is_none(), "filter should be None by default");
            assert!(!stdin, "stdin should be false by default");
            assert!(!dry_run, "dry_run should be false by default");
            assert!(!no_commit, "no_commit should be false by default");
            assert!(commit.is_none(), "commit should be None by default");
            assert!(!show_output, "show_output should be false by default");
            assert!(nextest_args.is_empty(), "nextest_args should be empty");
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_test_with_global_flags() {
    let temp = TempTestDir::new("test_global");
    let db_path = temp.path().join("test.db");
    let ws_path = temp.path();

    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        db_path.to_str().unwrap(),
        "-w",
        ws_path.to_str().unwrap(),
        "-v",
        "test",
    ])
    .expect("parse");

    assert!(matches!(config.command, Some(Command::Test { .. })));
    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(ws_path.to_path_buf()));
    assert!(config.verbose);
}

#[test]
fn test_subcommand_test_global_flags_after_subcommand_fails() {
    // Global flags must come before the subcommand
    let result = Config::try_parse_from(["hindsight-mcp", "test", "-v"]);
    assert!(result.is_err(), "global flags after subcommand should fail");
}

// ============================================================================
// Package and Binary Filtering Tests
// ============================================================================

#[test]
fn test_subcommand_package_short_flag() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "-p", "my-package"]).expect("parse");
    match config.command {
        Some(Command::Test { package, .. }) => {
            assert_eq!(package, vec!["my-package".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_package_long_flag() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--package", "my-package"])
        .expect("parse");
    match config.command {
        Some(Command::Test { package, .. }) => {
            assert_eq!(package, vec!["my-package".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_multiple_packages() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "test",
        "-p",
        "package1",
        "-p",
        "package2",
        "-p",
        "package3",
    ])
    .expect("parse");
    match config.command {
        Some(Command::Test { package, .. }) => {
            assert_eq!(
                package,
                vec![
                    "package1".to_string(),
                    "package2".to_string(),
                    "package3".to_string()
                ]
            );
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_bin_flag() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "--bin", "my-binary"]).expect("parse");
    match config.command {
        Some(Command::Test { bin, .. }) => {
            assert_eq!(bin, vec!["my-binary".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_multiple_bins() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "--bin", "bin1", "--bin", "bin2"])
            .expect("parse");
    match config.command {
        Some(Command::Test { bin, .. }) => {
            assert_eq!(bin, vec!["bin1".to_string(), "bin2".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_package_and_bin_combined() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "test",
        "-p",
        "my-package",
        "--bin",
        "my-binary",
    ])
    .expect("parse");
    match config.command {
        Some(Command::Test { package, bin, .. }) => {
            assert_eq!(package, vec!["my-package".to_string()]);
            assert_eq!(bin, vec!["my-binary".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_package_missing_value_fails() {
    let result = Config::try_parse_from(["hindsight-mcp", "test", "-p"]);
    assert!(result.is_err(), "package without value should fail");
}

#[test]
fn test_subcommand_bin_missing_value_fails() {
    let result = Config::try_parse_from(["hindsight-mcp", "test", "--bin"]);
    assert!(result.is_err(), "bin without value should fail");
}

// ============================================================================
// Filter Expression Tests
// ============================================================================

#[test]
fn test_subcommand_filter_short_flag() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "-E", "test(my_test)"]).expect("parse");
    match config.command {
        Some(Command::Test { filter, .. }) => {
            assert_eq!(filter, Some("test(my_test)".to_string()));
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_filter_long_flag() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--filter", "package(foo)"])
        .expect("parse");
    match config.command {
        Some(Command::Test { filter, .. }) => {
            assert_eq!(filter, Some("package(foo)".to_string()));
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_filter_complex_expression() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "test",
        "-E",
        "test(/integration/) & !test(/slow/)",
    ])
    .expect("parse");
    match config.command {
        Some(Command::Test { filter, .. }) => {
            assert_eq!(
                filter,
                Some("test(/integration/) & !test(/slow/)".to_string())
            );
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_filter_missing_value_fails() {
    let result = Config::try_parse_from(["hindsight-mcp", "test", "-E"]);
    assert!(result.is_err(), "filter without value should fail");
}

#[test]
fn test_subcommand_filter_with_special_characters() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "-E", "test(/test_[0-9]+/)"])
        .expect("parse");
    match config.command {
        Some(Command::Test { filter, .. }) => {
            assert_eq!(filter, Some("test(/test_[0-9]+/)".to_string()));
        }
        _ => panic!("expected Test command"),
    }
}

// ============================================================================
// Stdin Mode Tests
// ============================================================================

#[test]
fn test_subcommand_stdin_flag() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--stdin"]).expect("parse");
    match config.command {
        Some(Command::Test { stdin, .. }) => {
            assert!(stdin, "stdin should be true");
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_stdin_with_dry_run() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "--stdin", "--dry-run"]).expect("parse");
    match config.command {
        Some(Command::Test { stdin, dry_run, .. }) => {
            assert!(stdin);
            assert!(dry_run);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_stdin_with_commit() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--stdin", "--commit", "abc123"])
        .expect("parse");
    match config.command {
        Some(Command::Test { stdin, commit, .. }) => {
            assert!(stdin);
            assert_eq!(commit, Some("abc123".to_string()));
        }
        _ => panic!("expected Test command"),
    }
}

// ============================================================================
// Dry-Run Mode Tests
// ============================================================================

#[test]
fn test_subcommand_dry_run_flag() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--dry-run"]).expect("parse");
    match config.command {
        Some(Command::Test { dry_run, .. }) => {
            assert!(dry_run, "dry_run should be true");
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_dry_run_with_package() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--dry-run", "-p", "my-pkg"])
        .expect("parse");
    match config.command {
        Some(Command::Test {
            dry_run, package, ..
        }) => {
            assert!(dry_run);
            assert_eq!(package, vec!["my-pkg".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_dry_run_with_show_output() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--dry-run", "--show-output"])
        .expect("parse");
    match config.command {
        Some(Command::Test {
            dry_run,
            show_output,
            ..
        }) => {
            assert!(dry_run);
            assert!(show_output);
        }
        _ => panic!("expected Test command"),
    }
}

// ============================================================================
// Commit Linking Tests
// ============================================================================

#[test]
fn test_subcommand_no_commit_flag() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--no-commit"]).expect("parse");
    match config.command {
        Some(Command::Test { no_commit, .. }) => {
            assert!(no_commit, "no_commit should be true");
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_commit_flag() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--commit", "abc123def456"])
        .expect("parse");
    match config.command {
        Some(Command::Test { commit, .. }) => {
            assert_eq!(commit, Some("abc123def456".to_string()));
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_commit_full_sha() {
    let full_sha = "0123456789abcdef0123456789abcdef01234567";
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "--commit", full_sha]).expect("parse");
    match config.command {
        Some(Command::Test { commit, .. }) => {
            assert_eq!(commit, Some(full_sha.to_string()));
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_commit_and_no_commit_conflict() {
    // --commit and --no-commit are mutually exclusive
    let result =
        Config::try_parse_from(["hindsight-mcp", "test", "--commit", "abc123", "--no-commit"]);
    assert!(result.is_err(), "--commit and --no-commit should conflict");
}

#[test]
fn test_subcommand_no_commit_and_commit_conflict_reversed() {
    let result =
        Config::try_parse_from(["hindsight-mcp", "test", "--no-commit", "--commit", "abc123"]);
    assert!(result.is_err(), "--no-commit and --commit should conflict");
}

#[test]
fn test_subcommand_commit_missing_value_fails() {
    let result = Config::try_parse_from(["hindsight-mcp", "test", "--commit"]);
    assert!(result.is_err(), "commit without value should fail");
}

// ============================================================================
// Show Output Tests
// ============================================================================

#[test]
fn test_subcommand_show_output_flag() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--show-output"]).expect("parse");
    match config.command {
        Some(Command::Test { show_output, .. }) => {
            assert!(show_output, "show_output should be true");
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_show_output_with_package() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--show-output", "-p", "pkg"])
        .expect("parse");
    match config.command {
        Some(Command::Test {
            show_output,
            package,
            ..
        }) => {
            assert!(show_output);
            assert_eq!(package, vec!["pkg".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

// ============================================================================
// Passthrough Arguments Tests
// ============================================================================

#[test]
fn test_subcommand_passthrough_single_arg() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "--", "--retries", "2"]).expect("parse");
    match config.command {
        Some(Command::Test { nextest_args, .. }) => {
            assert_eq!(nextest_args, vec!["--retries".to_string(), "2".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_passthrough_multiple_args() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "test",
        "--",
        "--retries",
        "2",
        "--fail-fast",
        "--jobs",
        "4",
    ])
    .expect("parse");
    match config.command {
        Some(Command::Test { nextest_args, .. }) => {
            assert_eq!(
                nextest_args,
                vec![
                    "--retries".to_string(),
                    "2".to_string(),
                    "--fail-fast".to_string(),
                    "--jobs".to_string(),
                    "4".to_string(),
                ]
            );
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_passthrough_with_local_flags() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "test",
        "-p",
        "my-pkg",
        "--dry-run",
        "--",
        "--no-capture",
    ])
    .expect("parse");
    match config.command {
        Some(Command::Test {
            package,
            dry_run,
            nextest_args,
            ..
        }) => {
            assert_eq!(package, vec!["my-pkg".to_string()]);
            assert!(dry_run);
            assert_eq!(nextest_args, vec!["--no-capture".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_passthrough_empty() {
    // Just -- with no following args
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--"]).expect("parse");
    match config.command {
        Some(Command::Test { nextest_args, .. }) => {
            assert!(nextest_args.is_empty());
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_passthrough_order_preserved() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--", "arg1", "arg2", "arg3"])
        .expect("parse");
    match config.command {
        Some(Command::Test { nextest_args, .. }) => {
            assert_eq!(
                nextest_args,
                vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()]
            );
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_passthrough_with_equals_syntax() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "--", "--threads=4"]).expect("parse");
    match config.command {
        Some(Command::Test { nextest_args, .. }) => {
            assert_eq!(nextest_args, vec!["--threads=4".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

// ============================================================================
// All Flags Combined Tests
// ============================================================================

#[test]
fn test_subcommand_all_flags_combined() {
    let temp = TempTestDir::new("test_all_flags");
    let db_path = temp.path().join("test.db");
    let ws_path = temp.path();

    let config = Config::try_parse_from([
        "hindsight-mcp",
        "-d",
        db_path.to_str().unwrap(),
        "-w",
        ws_path.to_str().unwrap(),
        "test",
        "-p",
        "pkg1",
        "-p",
        "pkg2",
        "--bin",
        "bin1",
        "-E",
        "test(/foo/)",
        "--dry-run",
        "--commit",
        "abc123",
        "--show-output",
        "--",
        "--retries",
        "3",
    ])
    .expect("parse");

    assert_eq!(config.database, Some(db_path));
    assert_eq!(config.workspace, Some(ws_path.to_path_buf()));

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
            assert_eq!(package, vec!["pkg1".to_string(), "pkg2".to_string()]);
            assert_eq!(bin, vec!["bin1".to_string()]);
            assert_eq!(filter, Some("test(/foo/)".to_string()));
            assert!(!stdin);
            assert!(dry_run);
            assert!(!no_commit);
            assert_eq!(commit, Some("abc123".to_string()));
            assert!(show_output);
            assert_eq!(nextest_args, vec!["--retries".to_string(), "3".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_stdin_mode_with_all_compat_flags() {
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "test",
        "--stdin",
        "--dry-run",
        "--no-commit",
        "--show-output",
    ])
    .expect("parse");

    match config.command {
        Some(Command::Test {
            stdin,
            dry_run,
            no_commit,
            show_output,
            ..
        }) => {
            assert!(stdin);
            assert!(dry_run);
            assert!(no_commit);
            assert!(show_output);
        }
        _ => panic!("expected Test command"),
    }
}

// ============================================================================
// Integration with Ingestor (stdin mode simulation)
// ============================================================================

#[test]
fn test_subcommand_stdin_mode_ingests_correctly() {
    // Simulate what happens when --stdin mode processes valid JSON
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("test_stdin_ingest");
    let json = sample_nextest_json(10, 2, 1);

    // This simulates the ingestion that would happen in stdin mode
    let stats = ingestor
        .ingest_tests(temp.path(), &json, Some("abc123"))
        .expect("ingestion should succeed");

    assert_eq!(stats.test_runs_inserted, 1);
    assert_eq!(stats.test_results_inserted, 13); // 10 + 2 + 1

    // Verify commit was linked
    let db_ref = ingestor.database();
    let conn = db_ref.connection();
    let linked_sha: Option<String> = conn
        .query_row("SELECT commit_sha FROM test_runs LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("query");
    assert_eq!(linked_sha.as_deref(), Some("abc123"));
}

#[test]
fn test_subcommand_stdin_mode_no_commit_variant() {
    // Simulate --stdin --no-commit mode
    let db = test_database();
    let mut ingestor = Ingestor::new(db);

    let temp = TempTestDir::new("test_stdin_no_commit");
    let json = sample_nextest_json(5, 0, 0);

    // When --no-commit is set, commit_sha is None
    let stats = ingestor
        .ingest_tests(temp.path(), &json, None)
        .expect("ingestion should succeed");

    assert_eq!(stats.test_runs_inserted, 1);
    assert_eq!(stats.test_results_inserted, 5);

    // Verify no commit was linked
    let db_ref = ingestor.database();
    let conn = db_ref.connection();
    let linked_sha: Option<String> = conn
        .query_row("SELECT commit_sha FROM test_runs LIMIT 1", [], |row| {
            row.get(0)
        })
        .expect("query");
    assert!(linked_sha.is_none());
}

#[test]
fn test_subcommand_dry_run_does_not_ingest() {
    // In dry-run mode, we parse but don't write to database
    // This test verifies the parsing works even if we don't write
    let json = sample_nextest_json(5, 1, 0);
    let summary = hindsight_tests::parse_run_output(&json).expect("parse should work");

    assert_eq!(summary.passed, 5);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.ignored, 0);
    assert_eq!(summary.results.len(), 6);

    // In dry-run mode, we'd just display this summary without writing to DB
}

// ============================================================================
// Edge Cases for Test Subcommand
// ============================================================================

#[test]
fn test_subcommand_package_with_hyphen() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "-p", "my-cool-package"]).expect("parse");
    match config.command {
        Some(Command::Test { package, .. }) => {
            assert_eq!(package, vec!["my-cool-package".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_package_with_underscore() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "-p", "my_cool_package"]).expect("parse");
    match config.command {
        Some(Command::Test { package, .. }) => {
            assert_eq!(package, vec!["my_cool_package".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_filter_empty_string() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "-E", ""]).expect("parse");
    match config.command {
        Some(Command::Test { filter, .. }) => {
            assert_eq!(filter, Some(String::new()));
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_commit_short_sha() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "--commit", "abc"]).expect("parse");
    match config.command {
        Some(Command::Test { commit, .. }) => {
            assert_eq!(commit, Some("abc".to_string()));
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_commit_empty_string() {
    let config = Config::try_parse_from(["hindsight-mcp", "test", "--commit", ""]).expect("parse");
    match config.command {
        Some(Command::Test { commit, .. }) => {
            assert_eq!(commit, Some(String::new()));
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_bin_with_special_name() {
    let config =
        Config::try_parse_from(["hindsight-mcp", "test", "--bin", "my-bin_v2.3"]).expect("parse");
    match config.command {
        Some(Command::Test { bin, .. }) => {
            assert_eq!(bin, vec!["my-bin_v2.3".to_string()]);
        }
        _ => panic!("expected Test command"),
    }
}

#[test]
fn test_subcommand_many_packages() {
    let args: Vec<&str> = vec![
        "hindsight-mcp",
        "test",
        "-p",
        "pkg1",
        "-p",
        "pkg2",
        "-p",
        "pkg3",
        "-p",
        "pkg4",
        "-p",
        "pkg5",
        "-p",
        "pkg6",
        "-p",
        "pkg7",
        "-p",
        "pkg8",
        "-p",
        "pkg9",
        "-p",
        "pkg10",
    ];
    let config = Config::try_parse_from(args).expect("parse");
    match config.command {
        Some(Command::Test { package, .. }) => {
            assert_eq!(package.len(), 10);
            for (i, pkg) in package.iter().enumerate() {
                assert_eq!(pkg, &format!("pkg{}", i + 1));
            }
        }
        _ => panic!("expected Test command"),
    }
}

// ============================================================================
// Help and Version Tests
// ============================================================================

#[test]
fn test_subcommand_help_flag() {
    // -h after subcommand should give subcommand help
    let result = Config::try_parse_from(["hindsight-mcp", "test", "-h"]);
    // clap exits with error on -h, which try_parse_from catches
    assert!(result.is_err());
    // The error should be about showing help, not a parse error
}

#[test]
fn test_subcommand_help_long_flag() {
    let result = Config::try_parse_from(["hindsight-mcp", "test", "--help"]);
    assert!(result.is_err());
}

// ============================================================================
// Error Message Quality Tests
// ============================================================================

#[test]
fn test_subcommand_unknown_flag_error() {
    let result = Config::try_parse_from(["hindsight-mcp", "test", "--unknown-flag"]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("unknown") || err.contains("unexpected"),
        "error should mention unknown flag: {}",
        err
    );
}

#[test]
fn test_subcommand_typo_in_flag_error() {
    // Common typo: --dryrun instead of --dry-run
    let result = Config::try_parse_from(["hindsight-mcp", "test", "--dryrun"]);
    assert!(result.is_err());
}

#[test]
fn test_subcommand_double_dash_required_for_passthrough() {
    // Args that look like flags after -- should be treated as passthrough
    let config = Config::try_parse_from([
        "hindsight-mcp",
        "test",
        "--",
        "--unknown-nextest-flag",
        "-x",
    ])
    .expect("parse");
    match config.command {
        Some(Command::Test { nextest_args, .. }) => {
            assert_eq!(
                nextest_args,
                vec!["--unknown-nextest-flag".to_string(), "-x".to_string()]
            );
        }
        _ => panic!("expected Test command"),
    }
}
