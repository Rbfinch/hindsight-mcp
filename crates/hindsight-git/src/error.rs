// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Error types for hindsight-git

use thiserror::Error;

/// Errors that can occur during git operations
#[derive(Debug, Error)]
pub enum GitError {
    /// Error from git2 library
    #[error("Git error: {0}")]
    Git2(#[from] git2::Error),

    /// Repository not found at the specified path
    #[error("Repository not found: {path}")]
    RepositoryNotFound {
        /// The path that was searched for a repository
        path: String,
    },

    /// Invalid commit reference (branch, tag, or SHA)
    #[error("Invalid commit reference: {reference}")]
    InvalidReference {
        /// The reference string that could not be resolved
        reference: String,
    },
}
