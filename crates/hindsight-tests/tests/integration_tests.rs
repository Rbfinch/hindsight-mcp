// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Integration tests for hindsight-tests
//!
//! These tests verify parsing of real nextest output and test result handling.

use chrono::{TimeZone, Utc};
use hindsight_tests::result::{TestOutcome, TestResult};
use std::path::Path;
use std::process::Command;

/// Get the workspace root by finding the Cargo.toml with [workspace]
fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    Path::new(&manifest_dir)
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("Could not find workspace root")
        .to_path_buf()
}

/// Get the fixtures directory for test data
fn fixtures_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    Path::new(&manifest_dir).join("tests/fixtures")
}

#[test]
fn test_parse_sample_nextest_list_output() {
    let fixture_path = fixtures_dir().join("nextest-sample.json");
    let content =
        std::fs::read_to_string(&fixture_path).expect("Failed to read nextest-sample.json fixture");

    // Parse as generic JSON to verify structure
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Failed to parse nextest JSON");

    // Verify expected structure
    assert!(json.get("test-count").is_some(), "Should have test-count");
    assert!(json.get("rust-suites").is_some(), "Should have rust-suites");

    let suites = json["rust-suites"]
        .as_object()
        .expect("rust-suites should be an object");
    assert!(
        suites.contains_key("hindsight-git"),
        "Should have hindsight-git suite"
    );
    assert!(
        suites.contains_key("hindsight-tests"),
        "Should have hindsight-tests suite"
    );

    // Check test cases in hindsight-git suite
    let git_suite = &suites["hindsight-git"];
    let test_cases = git_suite["test-cases"]
        .as_object()
        .expect("Should have test-cases");
    assert!(test_cases.contains_key("commit::tests::test_is_valid_sha"));

    println!("Parsed {} suites from fixture", suites.len());
}

#[test]
fn test_create_test_results_from_parsed_data() {
    let fixture_path = fixtures_dir().join("nextest-sample.json");
    let content =
        std::fs::read_to_string(&fixture_path).expect("Failed to read nextest-sample.json fixture");

    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Failed to parse nextest JSON");

    let suites = json["rust-suites"]
        .as_object()
        .expect("rust-suites should be an object");
    let timestamp = Utc::now();

    let mut results: Vec<TestResult> = Vec::new();

    for (suite_name, suite_data) in suites {
        if let Some(test_cases) = suite_data["test-cases"].as_object() {
            for (test_name, test_info) in test_cases {
                let ignored = test_info["ignored"].as_bool().unwrap_or(false);
                let outcome = if ignored {
                    TestOutcome::Ignored
                } else {
                    TestOutcome::Passed // Assume passed for list output
                };

                results.push(TestResult {
                    name: format!("{}::{}", suite_name, test_name),
                    outcome,
                    duration_ms: 0, // Not available in list output
                    timestamp,
                    output: None,
                });
            }
        }
    }

    assert_eq!(results.len(), 4, "Should have 4 test results from fixture");

    // Check that we have both passed and ignored tests
    let ignored_count = results
        .iter()
        .filter(|r| r.outcome == TestOutcome::Ignored)
        .count();
    let passed_count = results
        .iter()
        .filter(|r| r.outcome == TestOutcome::Passed)
        .count();

    assert_eq!(ignored_count, 1, "Should have 1 ignored test");
    assert_eq!(passed_count, 3, "Should have 3 passed tests");

    println!("Created {} test results", results.len());
}

#[test]
fn test_result_json_serialization() {
    let timestamp = Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap();

    let results = vec![
        TestResult {
            name: "hindsight_git::commit::tests::test_is_valid_sha".to_string(),
            outcome: TestOutcome::Passed,
            duration_ms: 12,
            timestamp,
            output: None,
        },
        TestResult {
            name: "hindsight_git::commit::tests::test_short_sha".to_string(),
            outcome: TestOutcome::Failed,
            duration_ms: 5,
            timestamp,
            output: Some("assertion failed: expected 7, got 8".to_string()),
        },
        TestResult {
            name: "hindsight_tests::result::tests::test_ignored".to_string(),
            outcome: TestOutcome::Ignored,
            duration_ms: 0,
            timestamp,
            output: None,
        },
    ];

    // Serialize to JSON array
    let json = serde_json::to_string_pretty(&results).expect("Failed to serialize results");

    // Verify structure
    assert!(json.contains("\"outcome\": \"passed\""));
    assert!(json.contains("\"outcome\": \"failed\""));
    assert!(json.contains("\"outcome\": \"ignored\""));
    assert!(json.contains("\"duration_ms\": 12"));
    assert!(json.contains("assertion failed"));

    // Round-trip
    let deserialized: Vec<TestResult> =
        serde_json::from_str(&json).expect("Failed to deserialize results");
    assert_eq!(results, deserialized);

    println!("Serialized {} test results to JSON", results.len());
}

#[test]
fn test_run_actual_nextest_and_parse_output() {
    let workspace = workspace_root();

    // Run nextest list to get test information (faster than running tests)
    let output = Command::new("cargo")
        .args(["nextest", "list", "--message-format", "json"])
        .current_dir(&workspace)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Parse the JSON output
            let json: serde_json::Value =
                serde_json::from_str(&stdout).expect("Failed to parse nextest list output");

            // Verify we have test information
            assert!(json.get("test-count").is_some(), "Should have test-count");

            let test_count = json["test-count"].as_u64().unwrap_or(0);
            assert!(test_count > 0, "Should have at least one test");

            println!("Found {} tests in workspace via nextest list", test_count);
        }
        Ok(output) => {
            // nextest might not be installed or failed
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!(
                "Skipping: cargo nextest not available or failed: {}",
                stderr
            );
        }
        Err(e) => {
            println!("Skipping: Failed to run cargo nextest: {}", e);
        }
    }
}

#[test]
fn test_result_module_path_extraction() {
    let result = TestResult {
        name: "hindsight_git::commit::property_tests::prop_commit_sha_is_valid".to_string(),
        outcome: TestOutcome::Passed,
        duration_ms: 150,
        timestamp: Utc::now(),
        output: None,
    };

    assert_eq!(
        result.module_path(),
        Some("hindsight_git::commit::property_tests")
    );
    assert_eq!(result.test_fn_name(), "prop_commit_sha_is_valid");

    // Test with nested modules
    let result2 = TestResult {
        name: "crate::a::b::c::d::test_fn".to_string(),
        outcome: TestOutcome::Passed,
        duration_ms: 1,
        timestamp: Utc::now(),
        output: None,
    };

    assert_eq!(result2.module_path(), Some("crate::a::b::c::d"));
    assert_eq!(result2.test_fn_name(), "test_fn");
}

#[test]
fn test_duration_display_formatting() {
    let timestamp = Utc::now();

    let fast_test = TestResult {
        name: "test::fast".to_string(),
        outcome: TestOutcome::Passed,
        duration_ms: 5,
        timestamp,
        output: None,
    };
    assert_eq!(fast_test.duration_display(), "5ms");

    let slow_test = TestResult {
        name: "test::slow".to_string(),
        outcome: TestOutcome::Passed,
        duration_ms: 2500,
        timestamp,
        output: None,
    };
    assert_eq!(slow_test.duration_display(), "2.50s");

    let edge_test = TestResult {
        name: "test::edge".to_string(),
        outcome: TestOutcome::Passed,
        duration_ms: 1000,
        timestamp,
        output: None,
    };
    assert_eq!(edge_test.duration_display(), "1.00s");
}
