// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! hindsight-git: Git log processing for hindsight-mcp
//!
//! This library crate provides functionality to parse and process git logs
//! for consumption by the hindsight-mcp server.

#![warn(missing_docs)]

//! # Example
//!
//! ```no_run
//! use hindsight_git::{GitRepo, WalkOptions};
//!
//! let repo = GitRepo::open(".").expect("open repo");
//! let commits = repo.walk_commits(&WalkOptions::latest(10).with_diff())
//!     .expect("walk commits");
//!
//! for c in commits {
//!     println!("{} - {}", c.commit.short_sha(), c.commit.subject());
//! }
//! ```

pub mod commit;
pub mod error;
pub mod parser;

pub use commit::Commit;
pub use error::GitError;
pub use parser::{CommitWithDiff, DiffSummary, FileDiff, GitRepo, WalkOptions};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::commit::Commit;
    pub use crate::error::GitError;
    pub use crate::parser::{CommitWithDiff, DiffSummary, GitRepo, WalkOptions};
}
