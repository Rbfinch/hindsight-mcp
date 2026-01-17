//! Git commit types and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a parsed git commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// The commit SHA
    pub sha: String,
    /// Commit message
    pub message: String,
    /// Author name
    pub author: String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Parent commit SHAs
    pub parents: Vec<String>,
}

// TODO: Implement commit parsing from git2
