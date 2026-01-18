// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! hindsight-tests: Test log processing for hindsight-mcp
//!
//! This library crate provides functionality to parse and process test results
//! (particularly from cargo-nextest) for consumption by the hindsight-mcp server.
//!
//! # Example
//!
//! ```no_run
//! use hindsight_tests::nextest::{parse_run_output, StreamingParser};
//!
//! // Parse complete run output
//! let output = r#"{"type":"suite","event":"started","test_count":1}"#;
//! let summary = parse_run_output(output).unwrap();
//!
//! // Or use streaming parser for incremental parsing
//! let mut parser = StreamingParser::new();
//! parser.process_line(output).unwrap();
//! ```

pub mod error;
pub mod nextest;
pub mod result;

pub use error::TestsError;
pub use nextest::{
    LibtestEvent, StreamingParser, TestList, TestRunSummary, TestSuite, parse_list_output,
    parse_run_output,
};
pub use result::{TestOutcome, TestResult};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::error::TestsError;
    pub use crate::nextest::{StreamingParser, TestRunSummary, parse_run_output};
    pub use crate::result::{TestOutcome, TestResult};
}
