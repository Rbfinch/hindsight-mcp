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

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy to generate valid TestOutcome values
    fn outcome_strategy() -> impl Strategy<Value = TestOutcome> {
        prop_oneof![
            Just(TestOutcome::Passed),
            Just(TestOutcome::Failed),
            Just(TestOutcome::Ignored),
            Just(TestOutcome::TimedOut),
        ]
    }

    /// Strategy to generate test names in the format "crate::module::test_fn"
    fn test_name_strategy() -> impl Strategy<Value = String> {
        (
            "[a-z_]{1,20}", // crate name
            "[a-z_]{1,20}", // module name
            "[a-z_]{1,30}", // test function name
        )
            .prop_map(|(crate_name, module, test_fn)| {
                format!("{}::{}::tests::{}", crate_name, module, test_fn)
            })
    }

    /// Strategy to generate arbitrary TestResult values
    fn test_result_strategy() -> impl Strategy<Value = TestResult> {
        (
            test_name_strategy(),
            outcome_strategy(),
            0u64..1_000_000u64,         // duration_ms
            0i64..2_000_000_000i64,     // timestamp as unix seconds
            proptest::option::of(".*"), // output
        )
            .prop_map(|(name, outcome, duration_ms, ts, output)| {
                let timestamp = DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
                TestResult {
                    name,
                    outcome,
                    duration_ms,
                    timestamp,
                    output,
                }
            })
    }

    proptest! {
        /// Property: Round-trip JSON serialization preserves all fields
        #[test]
        fn prop_test_result_roundtrip_serialization(result in test_result_strategy()) {
            let json = serde_json::to_string(&result).expect("serialize");
            let deserialized: TestResult = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(result, deserialized);
        }

        /// Property: passed() and failed() are mutually exclusive when Passed or Failed
        #[test]
        fn prop_passed_failed_exclusive(result in test_result_strategy()) {
            // Can't be both passed and failed
            prop_assert!(!(result.passed() && result.failed()));

            // If passed, outcome must be Passed
            if result.passed() {
                prop_assert_eq!(result.outcome, TestOutcome::Passed);
            }

            // If failed, outcome must be Failed
            if result.failed() {
                prop_assert_eq!(result.outcome, TestOutcome::Failed);
            }
        }

        /// Property: duration_display format is consistent
        #[test]
        fn prop_duration_display_format(result in test_result_strategy()) {
            let display = result.duration_display();
            // Should end with ms or s
            prop_assert!(
                display.ends_with("ms") || display.ends_with('s'),
                "Display '{}' should end with 'ms' or 's'",
                display
            );

            // If < 1000ms, should show ms
            if result.duration_ms < 1000 {
                prop_assert!(display.ends_with("ms"));
            } else {
                prop_assert!(display.ends_with('s') && !display.ends_with("ms"));
            }
        }

        /// Property: test_fn_name is always a suffix of name
        #[test]
        fn prop_test_fn_name_is_suffix(result in test_result_strategy()) {
            let fn_name = result.test_fn_name();
            prop_assert!(
                result.name.ends_with(fn_name),
                "Function name '{}' should be suffix of '{}'",
                fn_name,
                result.name
            );
        }

        /// Property: module_path + "::" + test_fn_name == name (when module_path exists)
        #[test]
        fn prop_module_path_plus_fn_equals_name(result in test_result_strategy()) {
            if let Some(module) = result.module_path() {
                let reconstructed = format!("{}::{}", module, result.test_fn_name());
                prop_assert_eq!(result.name, reconstructed);
            }
        }

        /// Property: TestOutcome::is_success is true only for Passed and Ignored
        #[test]
        fn prop_outcome_is_success_consistency(outcome in outcome_strategy()) {
            let expected = matches!(outcome, TestOutcome::Passed | TestOutcome::Ignored);
            prop_assert_eq!(outcome.is_success(), expected);
        }

        /// Property: All outcomes have non-empty symbols
        #[test]
        fn prop_outcome_has_symbol(outcome in outcome_strategy()) {
            prop_assert!(!outcome.symbol().is_empty());
        }

        /// Property: Outcome serialization is lowercase
        #[test]
        fn prop_outcome_serialization_lowercase(outcome in outcome_strategy()) {
            let json = serde_json::to_string(&outcome).expect("serialize");
            // Remove quotes and check lowercase
            let value = json.trim_matches('"');
            prop_assert_eq!(value, value.to_lowercase());
        }
    }
}
