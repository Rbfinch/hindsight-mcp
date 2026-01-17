//! Error types for hindsight-git

use thiserror::Error;

/// Errors that can occur during git operations
#[derive(Debug, Error)]
pub enum GitError {
    /// Error from git2 library
    #[error("Git error: {0}")]
    Git2(#[from] git2::Error),

    /// Repository not found
    #[error("Repository not found: {path}")]
    RepositoryNotFound { path: String },

    /// Invalid commit reference
    #[error("Invalid commit reference: {reference}")]
    InvalidReference { reference: String },
}
