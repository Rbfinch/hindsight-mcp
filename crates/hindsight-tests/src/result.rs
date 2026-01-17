//! Test result types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a test execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test name
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

/// Possible test outcomes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

// TODO: Implement test result parsing
