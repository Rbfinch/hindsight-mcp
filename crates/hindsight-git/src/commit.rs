//! Git commit types and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a parsed git commit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    /// The commit SHA (40 hex characters)
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

impl Commit {
    /// Validate that a SHA is a valid 40-character hex string
    #[must_use]
    pub fn is_valid_sha(sha: &str) -> bool {
        sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Get the short SHA (first 7 characters)
    #[must_use]
    pub fn short_sha(&self) -> &str {
        &self.sha[..7.min(self.sha.len())]
    }

    /// Check if this is a merge commit (has multiple parents)
    #[must_use]
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }

    /// Check if this is a root commit (has no parents)
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parents.is_empty()
    }

    /// Get the first line of the commit message (subject)
    #[must_use]
    pub fn subject(&self) -> &str {
        self.message.lines().next().unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use similar_asserts::assert_eq;

    fn sample_commit() -> Commit {
        Commit {
            sha: "1945ab9c752534e733c38ba0109dc3b741f0a6eb".to_string(),
            message: "feat(skills): add milestone-creator\n\nDetailed description here."
                .to_string(),
            author: "Test Author".to_string(),
            author_email: "test@example.com".to_string(),
            timestamp: Utc.with_ymd_and_hms(2026, 1, 17, 2, 33, 6).unwrap(),
            parents: vec!["c460aeb7fb2d109c17e43de0ce681faec0b7374d".to_string()],
        }
    }

    #[test]
    fn test_commit_serialization_roundtrip() {
        let commit = sample_commit();
        let json = serde_json::to_string(&commit).expect("serialize");
        let deserialized: Commit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(commit, deserialized);
    }

    #[test]
    fn test_commit_json_format() {
        let commit = sample_commit();
        let json = serde_json::to_string_pretty(&commit).expect("serialize");
        assert!(json.contains("\"sha\":"));
        assert!(json.contains("1945ab9c752534e733c38ba0109dc3b741f0a6eb"));
        assert!(json.contains("\"timestamp\":"));
    }

    #[test]
    fn test_is_valid_sha_valid() {
        assert!(Commit::is_valid_sha(
            "1945ab9c752534e733c38ba0109dc3b741f0a6eb"
        ));
        assert!(Commit::is_valid_sha(
            "0000000000000000000000000000000000000000"
        ));
        assert!(Commit::is_valid_sha(
            "ffffffffffffffffffffffffffffffffffffffff"
        ));
        assert!(Commit::is_valid_sha(
            "ABCDEF1234567890abcdef1234567890abcdef12"
        ));
    }

    #[test]
    fn test_is_valid_sha_invalid() {
        // Too short
        assert!(!Commit::is_valid_sha("1945ab9"));
        // Too long
        assert!(!Commit::is_valid_sha(
            "1945ab9c752534e733c38ba0109dc3b741f0a6eb0"
        ));
        // Invalid characters
        assert!(!Commit::is_valid_sha(
            "1945ab9c752534e733c38ba0109dc3b741f0a6eg"
        ));
        // Empty
        assert!(!Commit::is_valid_sha(""));
    }

    #[test]
    fn test_short_sha() {
        let commit = sample_commit();
        assert_eq!(commit.short_sha(), "1945ab9");
    }

    #[test]
    fn test_short_sha_handles_short_input() {
        let mut commit = sample_commit();
        commit.sha = "abc".to_string();
        assert_eq!(commit.short_sha(), "abc");
    }

    #[test]
    fn test_is_merge_with_multiple_parents() {
        let mut commit = sample_commit();
        commit.parents = vec![
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        ];
        assert!(commit.is_merge());
    }

    #[test]
    fn test_is_merge_with_single_parent() {
        let commit = sample_commit();
        assert!(!commit.is_merge());
    }

    #[test]
    fn test_is_root_with_no_parents() {
        let mut commit = sample_commit();
        commit.parents = vec![];
        assert!(commit.is_root());
    }

    #[test]
    fn test_is_root_with_parents() {
        let commit = sample_commit();
        assert!(!commit.is_root());
    }

    #[test]
    fn test_subject_multiline() {
        let commit = sample_commit();
        assert_eq!(commit.subject(), "feat(skills): add milestone-creator");
    }

    #[test]
    fn test_subject_single_line() {
        let mut commit = sample_commit();
        commit.message = "Simple message".to_string();
        assert_eq!(commit.subject(), "Simple message");
    }

    #[test]
    fn test_subject_empty_message() {
        let mut commit = sample_commit();
        commit.message = String::new();
        assert_eq!(commit.subject(), "");
    }

    #[test]
    fn test_timestamp_iso8601_serialization() {
        let commit = sample_commit();
        let json = serde_json::to_string(&commit).expect("serialize");
        // chrono serializes to RFC 3339/ISO 8601 format
        assert!(json.contains("2026-01-17"));
    }
}
