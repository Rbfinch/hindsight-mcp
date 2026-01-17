//! Test result types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a test execution result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestResult {
    /// Test name (full path including module)
    pub name: String,
    /// Test outcome
    pub outcome: TestOutcome,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Timestamp when the test was run
    pub timestamp: DateTime<Utc>,
    /// Test output (stdout/stderr)
    pub output: Option<String>,
}

impl TestResult {
    /// Check if the test passed
    #[must_use]
    pub fn passed(&self) -> bool {
        self.outcome == TestOutcome::Passed
    }

    /// Check if the test failed
    #[must_use]
    pub fn failed(&self) -> bool {
        self.outcome == TestOutcome::Failed
    }

    /// Get the duration as a human-readable string
    #[must_use]
    pub fn duration_display(&self) -> String {
        if self.duration_ms < 1000 {
            format!("{}ms", self.duration_ms)
        } else {
            format!("{:.2}s", self.duration_ms as f64 / 1000.0)
        }
    }

    /// Extract the test module path (everything before the last ::)
    #[must_use]
    pub fn module_path(&self) -> Option<&str> {
        self.name.rsplit_once("::").map(|(module, _)| module)
    }

    /// Extract the test function name (everything after the last ::)
    #[must_use]
    pub fn test_fn_name(&self) -> &str {
        self.name
            .rsplit_once("::")
            .map(|(_, name)| name)
            .unwrap_or(&self.name)
    }
}

/// Possible test outcomes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestOutcome {
    /// Test passed
    Passed,
    /// Test failed
    Failed,
    /// Test was ignored/skipped
    Ignored,
    /// Test timed out
    TimedOut,
}

impl TestOutcome {
    /// Returns true if this outcome represents a successful test
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Passed | Self::Ignored)
    }

    /// Returns a human-readable status symbol
    #[must_use]
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Passed => "✅",
            Self::Failed => "❌",
            Self::Ignored => "⏭️",
            Self::TimedOut => "⏰",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use similar_asserts::assert_eq;

    fn sample_result() -> TestResult {
        TestResult {
            name: "hindsight_git::commit::tests::test_is_valid_sha".to_string(),
            outcome: TestOutcome::Passed,
            duration_ms: 42,
            timestamp: Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap(),
            output: None,
        }
    }

    #[test]
    fn test_result_serialization_roundtrip() {
        let result = sample_result();
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: TestResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result, deserialized);
    }

    #[test]
    fn test_result_json_format() {
        let result = sample_result();
        let json = serde_json::to_string_pretty(&result).expect("serialize");
        assert!(json.contains("\"outcome\": \"passed\""));
        assert!(json.contains("\"duration_ms\": 42"));
    }

    #[test]
    fn test_passed_returns_true_for_passed() {
        let result = sample_result();
        assert!(result.passed());
        assert!(!result.failed());
    }

    #[test]
    fn test_failed_returns_true_for_failed() {
        let mut result = sample_result();
        result.outcome = TestOutcome::Failed;
        assert!(result.failed());
        assert!(!result.passed());
    }

    #[test]
    fn test_duration_display_milliseconds() {
        let result = sample_result();
        assert_eq!(result.duration_display(), "42ms");
    }

    #[test]
    fn test_duration_display_seconds() {
        let mut result = sample_result();
        result.duration_ms = 1500;
        assert_eq!(result.duration_display(), "1.50s");
    }

    #[test]
    fn test_module_path() {
        let result = sample_result();
        assert_eq!(result.module_path(), Some("hindsight_git::commit::tests"));
    }

    #[test]
    fn test_test_fn_name() {
        let result = sample_result();
        assert_eq!(result.test_fn_name(), "test_is_valid_sha");
    }

    #[test]
    fn test_test_fn_name_no_module() {
        let mut result = sample_result();
        result.name = "simple_test".to_string();
        assert_eq!(result.test_fn_name(), "simple_test");
    }

    #[test]
    fn test_outcome_is_success() {
        assert!(TestOutcome::Passed.is_success());
        assert!(TestOutcome::Ignored.is_success());
        assert!(!TestOutcome::Failed.is_success());
        assert!(!TestOutcome::TimedOut.is_success());
    }

    #[test]
    fn test_outcome_symbol() {
        assert_eq!(TestOutcome::Passed.symbol(), "✅");
        assert_eq!(TestOutcome::Failed.symbol(), "❌");
        assert_eq!(TestOutcome::Ignored.symbol(), "⏭️");
        assert_eq!(TestOutcome::TimedOut.symbol(), "⏰");
    }

    #[test]
    fn test_outcome_serialization() {
        let outcomes = vec![
            (TestOutcome::Passed, "\"passed\""),
            (TestOutcome::Failed, "\"failed\""),
            (TestOutcome::Ignored, "\"ignored\""),
            (TestOutcome::TimedOut, "\"timedout\""),
        ];

        for (outcome, expected) in outcomes {
            let json = serde_json::to_string(&outcome).expect("serialize");
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_result_with_output() {
        let mut result = sample_result();
        result.output = Some("assertion failed: expected 5, got 3".to_string());

        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: TestResult = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(
            deserialized.output,
            Some("assertion failed: expected 5, got 3".to_string())
        );
    }
}
