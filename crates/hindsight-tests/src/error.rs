// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Error types for hindsight-tests

use thiserror::Error;

/// Errors that can occur during test log processing
#[derive(Debug, Error)]
pub enum TestsError {
    /// Error parsing JSON
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Error reading test output file
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid test result format
    #[error("Invalid test result format: {message}")]
    InvalidFormat {
        /// Description of the format error
        message: String,
    },
}
