//! hindsight-git: Git log processing for hindsight-mcp
//!
//! This library crate provides functionality to parse and process git logs
//! for consumption by the hindsight-mcp server.

pub mod commit;
pub mod error;
pub mod parser;

pub use error::GitError;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::error::GitError;
}
