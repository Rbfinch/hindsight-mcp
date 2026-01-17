//! Nextest output parsing
//!
//! This module provides functionality to parse cargo-nextest output in various formats:
//! - `nextest list --message-format json` for test discovery
//! - `nextest run --message-format libtest-json` for test execution results
//!
//! # Example
//!
//! ```no_run
//! use hindsight_tests::nextest::{parse_list_output, parse_run_output};
//!
//! // Parse test list
//! let list_json = r#"{"test-count": 5, "rust-suites": {}}"#;
//! let list = parse_list_output(list_json).unwrap();
//!
//! // Parse run output (line-delimited JSON)
//! let run_output = r#"{"type":"suite","event":"started","test_count":1}"#;
//! let run = parse_run_output(run_output).unwrap();
//! ```

use crate::error::TestsError;
use crate::result::{TestOutcome, TestResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Test List Types (from `cargo nextest list --message-format json`)
// ============================================================================

/// Output from `cargo nextest list --message-format json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestList {
    /// Total number of tests
    #[serde(rename = "test-count")]
    pub test_count: usize,
    /// Test suites by binary ID
    #[serde(rename = "rust-suites")]
    pub rust_suites: HashMap<String, TestSuite>,
}

/// A test suite (binary containing tests)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    /// Package name
    #[serde(rename = "package-name")]
    pub package_name: String,
    /// Binary ID
    #[serde(rename = "binary-id")]
    pub binary_id: String,
    /// Binary name
    #[serde(rename = "binary-name")]
    pub binary_name: String,
    /// Kind of binary (lib, bin, test, etc.)
    pub kind: String,
    /// Test cases in this suite
    pub testcases: HashMap<String, TestCase>,
}

/// A single test case in a suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Kind of test (usually "test")
    pub kind: String,
    /// Whether the test is ignored
    pub ignored: bool,
}

impl TestList {
    /// Get all test names across all suites
    #[must_use]
    pub fn all_test_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for (suite_id, suite) in &self.rust_suites {
            for test_name in suite.testcases.keys() {
                names.push(format!("{}::{}", suite_id, test_name));
            }
        }
        names
    }

    /// Get all test names in a specific suite
    #[must_use]
    pub fn tests_in_suite(&self, suite_id: &str) -> Vec<&str> {
        self.rust_suites
            .get(suite_id)
            .map(|s| s.testcases.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Count ignored tests
    #[must_use]
    pub fn ignored_count(&self) -> usize {
        self.rust_suites
            .values()
            .flat_map(|s| s.testcases.values())
            .filter(|tc| tc.ignored)
            .count()
    }
}

// ============================================================================
// Test Run Types (from `cargo nextest run --message-format libtest-json`)
// ============================================================================

/// A single event from libtest JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LibtestEvent {
    /// Suite started event
    Suite(SuiteEvent),
    /// Test event (started, ok, failed, ignored)
    Test(TestEvent),
}

/// Suite-level event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteEvent {
    /// Event type: "started" or "ok"/"failed"
    pub event: String,
    /// Number of tests (only in "started" event)
    pub test_count: Option<usize>,
    /// Number of passed tests (in final event)
    pub passed: Option<usize>,
    /// Number of failed tests (in final event)
    pub failed: Option<usize>,
    /// Number of ignored tests (in final event)
    pub ignored: Option<usize>,
    /// Execution time (in final event)
    pub exec_time: Option<f64>,
}

/// Test-level event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEvent {
    /// Event type: "started", "ok", "failed", "ignored"
    pub event: String,
    /// Full test name including binary
    pub name: String,
    /// Execution time in seconds (only in finished events)
    pub exec_time: Option<f64>,
    /// Stdout output (only in failed events)
    pub stdout: Option<String>,
}

/// Aggregated results from a test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunSummary {
    /// Total tests run
    pub total: usize,
    /// Tests passed
    pub passed: usize,
    /// Tests failed
    pub failed: usize,
    /// Tests ignored
    pub ignored: usize,
    /// Total execution time in seconds
    pub exec_time_secs: f64,
    /// Individual test results
    pub results: Vec<TestResult>,
}

impl TestRunSummary {
    /// Create an empty summary
    #[must_use]
    pub fn empty() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            ignored: 0,
            exec_time_secs: 0.0,
            results: Vec::new(),
        }
    }

    /// Check if all tests passed
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Get failing tests
    #[must_use]
    pub fn failing_tests(&self) -> Vec<&TestResult> {
        self.results.iter().filter(|r| r.failed()).collect()
    }
}

// ============================================================================
// Parsing Functions
// ============================================================================

/// Parse `cargo nextest list --message-format json` output
///
/// # Errors
///
/// Returns `TestsError::JsonParse` if the JSON is invalid.
pub fn parse_list_output(json: &str) -> Result<TestList, TestsError> {
    serde_json::from_str(json).map_err(TestsError::from)
}

/// Parse `cargo nextest run --message-format libtest-json` output
///
/// The output is newline-delimited JSON.
///
/// # Errors
///
/// Returns `TestsError::JsonParse` if any line is invalid JSON.
pub fn parse_run_output(output: &str) -> Result<TestRunSummary, TestsError> {
    let mut summary = TestRunSummary::empty();
    let mut pending_tests: HashMap<String, chrono::DateTime<Utc>> = HashMap::new();
    let now = Utc::now();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let event: LibtestEvent = serde_json::from_str(line)?;

        match event {
            LibtestEvent::Suite(suite) => {
                if suite.event == "started" {
                    if let Some(count) = suite.test_count {
                        summary.total = count;
                    }
                } else {
                    // Final summary event
                    if let Some(passed) = suite.passed {
                        summary.passed = passed;
                    }
                    if let Some(failed) = suite.failed {
                        summary.failed = failed;
                    }
                    if let Some(ignored) = suite.ignored {
                        summary.ignored = ignored;
                    }
                    if let Some(exec_time) = suite.exec_time {
                        summary.exec_time_secs = exec_time;
                    }
                }
            }
            LibtestEvent::Test(test) => {
                if test.event == "started" {
                    pending_tests.insert(test.name.clone(), now);
                } else {
                    // Test finished
                    let outcome = match test.event.as_str() {
                        "ok" => TestOutcome::Passed,
                        "failed" => TestOutcome::Failed,
                        "ignored" => TestOutcome::Ignored,
                        _ => TestOutcome::Failed,
                    };

                    let duration_ms = test.exec_time.map(|t| (t * 1000.0) as u64).unwrap_or(0);

                    // Parse the name - nextest format: "binary-id::binary_name$test::path"
                    let test_name = normalize_test_name(&test.name);

                    let result = TestResult {
                        name: test_name,
                        outcome,
                        duration_ms,
                        timestamp: now,
                        output: test.stdout,
                    };

                    summary.results.push(result);
                    pending_tests.remove(&test.name);
                }
            }
        }
    }

    Ok(summary)
}

/// Normalize a nextest test name to a clean format
///
/// Input: "hindsight-tests::hindsight_tests$result::tests::test_name"
/// Output: "result::tests::test_name"
fn normalize_test_name(name: &str) -> String {
    // Find the $ separator that nextest uses
    if let Some(idx) = name.find('$') {
        name[idx + 1..].to_string()
    } else if let Some(idx) = name.find("::") {
        // Fallback: skip binary prefix
        name[idx + 2..].to_string()
    } else {
        name.to_string()
    }
}

/// Parse a single libtest JSON event
///
/// # Errors
///
/// Returns `TestsError::JsonParse` if the JSON is invalid.
pub fn parse_event(json: &str) -> Result<LibtestEvent, TestsError> {
    serde_json::from_str(json).map_err(TestsError::from)
}

// ============================================================================
// Streaming Parser for incremental parsing
// ============================================================================

/// A streaming parser for libtest JSON output
pub struct StreamingParser {
    pending_tests: HashMap<String, chrono::DateTime<Utc>>,
    results: Vec<TestResult>,
    total: usize,
}

impl StreamingParser {
    /// Create a new streaming parser
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending_tests: HashMap::new(),
            results: Vec::new(),
            total: 0,
        }
    }

    /// Process a single line of output
    ///
    /// # Errors
    ///
    /// Returns `TestsError::JsonParse` if the line is invalid JSON.
    pub fn process_line(&mut self, line: &str) -> Result<Option<TestResult>, TestsError> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let event: LibtestEvent = serde_json::from_str(line)?;
        let now = Utc::now();

        match event {
            LibtestEvent::Suite(suite) => {
                if suite.event == "started" {
                    if let Some(count) = suite.test_count {
                        self.total = count;
                    }
                }
                Ok(None)
            }
            LibtestEvent::Test(test) => {
                if test.event == "started" {
                    self.pending_tests.insert(test.name.clone(), now);
                    Ok(None)
                } else {
                    let outcome = match test.event.as_str() {
                        "ok" => TestOutcome::Passed,
                        "failed" => TestOutcome::Failed,
                        "ignored" => TestOutcome::Ignored,
                        _ => TestOutcome::Failed,
                    };

                    let duration_ms = test.exec_time.map(|t| (t * 1000.0) as u64).unwrap_or(0);

                    let test_name = normalize_test_name(&test.name);

                    let result = TestResult {
                        name: test_name,
                        outcome,
                        duration_ms,
                        timestamp: now,
                        output: test.stdout,
                    };

                    self.results.push(result.clone());
                    self.pending_tests.remove(&test.name);
                    Ok(Some(result))
                }
            }
        }
    }

    /// Get all accumulated results
    #[must_use]
    pub fn results(&self) -> &[TestResult] {
        &self.results
    }

    /// Finalize and return summary
    #[must_use]
    pub fn into_summary(self) -> TestRunSummary {
        let passed = self.results.iter().filter(|r| r.passed()).count();
        let failed = self.results.iter().filter(|r| r.failed()).count();
        let ignored = self
            .results
            .iter()
            .filter(|r| r.outcome == TestOutcome::Ignored)
            .count();
        let exec_time_secs =
            self.results.iter().map(|r| r.duration_ms).sum::<u64>() as f64 / 1000.0;

        TestRunSummary {
            total: self.total,
            passed,
            failed,
            ignored,
            exec_time_secs,
            results: self.results,
        }
    }
}

impl Default for StreamingParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;

    #[test]
    fn test_parse_list_output() {
        let json = r#"{
            "test-count": 2,
            "rust-suites": {
                "my-crate": {
                    "package-name": "my-crate",
                    "binary-id": "my-crate",
                    "binary-name": "my_crate",
                    "kind": "lib",
                    "testcases": {
                        "tests::test_one": {"kind": "test", "ignored": false},
                        "tests::test_two": {"kind": "test", "ignored": true}
                    }
                }
            }
        }"#;

        let list = parse_list_output(json).expect("Should parse");
        assert_eq!(list.test_count, 2);
        assert_eq!(list.rust_suites.len(), 1);
        assert_eq!(list.ignored_count(), 1);
    }

    #[test]
    fn test_parse_run_output_single_test() {
        let output = r#"{"type":"suite","event":"started","test_count":1}
{"type":"test","event":"started","name":"my-crate::my_crate$tests::test_one"}
{"type":"test","event":"ok","name":"my-crate::my_crate$tests::test_one","exec_time":0.015}
{"type":"suite","event":"ok","passed":1,"failed":0,"ignored":0,"exec_time":0.015}"#;

        let summary = parse_run_output(output).expect("Should parse");
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.results.len(), 1);
        assert_eq!(summary.results[0].name, "tests::test_one");
        assert!(summary.results[0].passed());
    }

    #[test]
    fn test_parse_run_output_failed_test() {
        let output = r#"{"type":"suite","event":"started","test_count":1}
{"type":"test","event":"started","name":"crate::bin$mod::test_fail"}
{"type":"test","event":"failed","name":"crate::bin$mod::test_fail","exec_time":0.005,"stdout":"assertion failed"}
{"type":"suite","event":"failed","passed":0,"failed":1,"ignored":0,"exec_time":0.005}"#;

        let summary = parse_run_output(output).expect("Should parse");
        assert_eq!(summary.failed, 1);
        assert!(summary.results[0].failed());
        assert_eq!(
            summary.results[0].output,
            Some("assertion failed".to_string())
        );
    }

    #[test]
    fn test_parse_run_output_multiple_tests() {
        let output = r#"{"type":"suite","event":"started","test_count":3}
{"type":"test","event":"started","name":"c::b$test_a"}
{"type":"test","event":"ok","name":"c::b$test_a","exec_time":0.001}
{"type":"test","event":"started","name":"c::b$test_b"}
{"type":"test","event":"ignored","name":"c::b$test_b","exec_time":0.0}
{"type":"test","event":"started","name":"c::b$test_c"}
{"type":"test","event":"ok","name":"c::b$test_c","exec_time":0.002}
{"type":"suite","event":"ok","passed":2,"failed":0,"ignored":1,"exec_time":0.003}"#;

        let summary = parse_run_output(output).expect("Should parse");
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.ignored, 1);
        assert_eq!(summary.results.len(), 3);
    }

    #[test]
    fn test_normalize_test_name() {
        assert_eq!(
            normalize_test_name("hindsight-tests::hindsight_tests$result::tests::test_passed"),
            "result::tests::test_passed"
        );
        assert_eq!(
            normalize_test_name("crate::binary$module::test"),
            "module::test"
        );
        assert_eq!(normalize_test_name("simple_test"), "simple_test");
    }

    #[test]
    fn test_streaming_parser() {
        let mut parser = StreamingParser::new();

        let result = parser
            .process_line(r#"{"type":"suite","event":"started","test_count":1}"#)
            .expect("Should parse");
        assert!(result.is_none());

        let result = parser
            .process_line(r#"{"type":"test","event":"started","name":"c::b$t"}"#)
            .expect("Should parse");
        assert!(result.is_none());

        let result = parser
            .process_line(r#"{"type":"test","event":"ok","name":"c::b$t","exec_time":0.01}"#)
            .expect("Should parse");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "t");

        let summary = parser.into_summary();
        assert_eq!(summary.passed, 1);
    }

    #[test]
    fn test_test_run_summary_helpers() {
        let summary = TestRunSummary {
            total: 3,
            passed: 2,
            failed: 1,
            ignored: 0,
            exec_time_secs: 0.1,
            results: vec![
                TestResult {
                    name: "test_pass".to_string(),
                    outcome: TestOutcome::Passed,
                    duration_ms: 50,
                    timestamp: Utc::now(),
                    output: None,
                },
                TestResult {
                    name: "test_fail".to_string(),
                    outcome: TestOutcome::Failed,
                    duration_ms: 50,
                    timestamp: Utc::now(),
                    output: Some("error".to_string()),
                },
            ],
        };

        assert!(!summary.all_passed());
        assert_eq!(summary.failing_tests().len(), 1);
        assert_eq!(summary.failing_tests()[0].name, "test_fail");
    }

    #[test]
    fn test_test_list_helpers() {
        let json = r#"{
            "test-count": 3,
            "rust-suites": {
                "suite-a": {
                    "package-name": "a",
                    "binary-id": "suite-a",
                    "binary-name": "a",
                    "kind": "lib",
                    "testcases": {
                        "test_1": {"kind": "test", "ignored": false},
                        "test_2": {"kind": "test", "ignored": false}
                    }
                },
                "suite-b": {
                    "package-name": "b",
                    "binary-id": "suite-b",
                    "binary-name": "b",
                    "kind": "lib",
                    "testcases": {
                        "test_3": {"kind": "test", "ignored": true}
                    }
                }
            }
        }"#;

        let list = parse_list_output(json).expect("Should parse");
        let all_names = list.all_test_names();
        assert_eq!(all_names.len(), 3);

        let suite_a_tests = list.tests_in_suite("suite-a");
        assert_eq!(suite_a_tests.len(), 2);

        assert_eq!(list.ignored_count(), 1);
    }

    #[test]
    fn test_parse_empty_output() {
        let summary = parse_run_output("").expect("Should parse empty");
        assert_eq!(summary.total, 0);
        assert_eq!(summary.results.len(), 0);
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_run_output("not json");
        assert!(result.is_err());
    }
}
