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

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy to generate valid 40-character hex SHA strings
    fn sha_strategy() -> impl Strategy<Value = String> {
        proptest::string::string_regex("[0-9a-f]{40}")
            .expect("valid regex")
            .prop_map(|s| s.to_lowercase())
    }

    /// Strategy to generate arbitrary Commit values
    fn commit_strategy() -> impl Strategy<Value = Commit> {
        (
            sha_strategy(),
            ".*",                    // message
            "[A-Za-z ]{1,50}",       // author name
            "[a-z]+@[a-z]+\\.[a-z]+", // author email
            0i64..2_000_000_000i64,  // timestamp as unix seconds
            proptest::collection::vec(sha_strategy(), 0..3), // parents
        )
            .prop_map(|(sha, message, author, author_email, ts, parents)| {
                let timestamp = DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now());
                Commit {
                    sha,
                    message,
                    author,
                    author_email,
                    timestamp,
                    parents,
                }
            })
    }

    proptest! {
        /// Property: Any generated Commit should have a valid SHA
        #[test]
        fn prop_commit_sha_is_valid(commit in commit_strategy()) {
            prop_assert!(
                Commit::is_valid_sha(&commit.sha),
                "Generated SHA should be valid: {}",
                commit.sha
            );
        }

        /// Property: Round-trip JSON serialization preserves all fields
        #[test]
        fn prop_commit_roundtrip_serialization(commit in commit_strategy()) {
            let json = serde_json::to_string(&commit).expect("serialize");
            let deserialized: Commit = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(commit, deserialized);
        }

        /// Property: short_sha returns at most 7 characters
        #[test]
        fn prop_short_sha_length(commit in commit_strategy()) {
            let short = commit.short_sha();
            prop_assert!(short.len() <= 7);
            prop_assert!(short.len() >= 1);
        }

        /// Property: is_merge is true iff parents.len() > 1
        #[test]
        fn prop_is_merge_iff_multiple_parents(commit in commit_strategy()) {
            prop_assert_eq!(commit.is_merge(), commit.parents.len() > 1);
        }

        /// Property: is_root is true iff parents is empty
        #[test]
        fn prop_is_root_iff_no_parents(commit in commit_strategy()) {
            prop_assert_eq!(commit.is_root(), commit.parents.is_empty());
        }

        /// Property: subject is always a substring of message
        #[test]
        fn prop_subject_is_prefix_of_message(commit in commit_strategy()) {
            let subject = commit.subject();
            prop_assert!(
                commit.message.starts_with(subject),
                "Subject '{}' should be prefix of message '{}'",
                subject,
                commit.message
            );
        }

        /// Property: All parent SHAs should be valid
        #[test]
        fn prop_all_parent_shas_valid(commit in commit_strategy()) {
            for parent in &commit.parents {
                prop_assert!(
                    Commit::is_valid_sha(parent),
                    "Parent SHA should be valid: {}",
                    parent
                );
            }
        }

        /// Property: is_valid_sha accepts only 40-char lowercase hex
        #[test]
        fn prop_valid_sha_format(sha in sha_strategy()) {
            prop_assert!(Commit::is_valid_sha(&sha));
            prop_assert_eq!(sha.len(), 40);
            prop_assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
        }

        /// Property: is_valid_sha rejects strings of wrong length
        #[test]
        fn prop_invalid_sha_wrong_length(
            prefix in "[0-9a-f]{0,39}",
            suffix in "[0-9a-f]{0,10}"
        ) {
            let combined = format!("{}{}", prefix, suffix);
            if combined.len() != 40 {
                prop_assert!(!Commit::is_valid_sha(&combined));
            }
        }
    }
}
