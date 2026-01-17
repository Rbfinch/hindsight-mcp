//! hindsight-tests: Test log processing for hindsight-mcp
//!
//! This library crate provides functionality to parse and process test results
//! (particularly from cargo-nextest) for consumption by the hindsight-mcp server.

pub mod error;
pub mod nextest;
pub mod result;

pub use error::TestsError;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::error::TestsError;
    pub use crate::result::TestResult;
}
