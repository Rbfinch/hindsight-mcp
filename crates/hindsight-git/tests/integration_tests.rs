//! Integration tests for hindsight-git
//!
//! These tests use the actual git repository to verify parsing functionality.

use chrono::{DateTime, Utc};
use git2::Repository;
use hindsight_git::commit::Commit;
use std::path::Path;

/// Get the workspace root by finding the Cargo.toml with [workspace]
fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    Path::new(&manifest_dir)
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("Could not find workspace root")
        .to_path_buf()
}

/// Helper to create a Commit from a git2::Commit
fn commit_from_git2(git_commit: &git2::Commit<'_>) -> Commit {
    let timestamp =
        DateTime::from_timestamp(git_commit.time().seconds(), 0).unwrap_or_else(Utc::now);

    Commit {
        sha: git_commit.id().to_string(),
        message: git_commit.message().unwrap_or("").to_string(),
        author: git_commit.author().name().unwrap_or("Unknown").to_string(),
        author_email: git_commit.author().email().unwrap_or("").to_string(),
        timestamp,
        parents: git_commit.parent_ids().map(|id| id.to_string()).collect(),
    }
}

#[test]
fn test_parse_commits_from_real_repository() {
    let repo_path = workspace_root();
    let repo = Repository::open(&repo_path).expect("Failed to open git repository");

    // Get HEAD commit
    let head = repo.head().expect("Failed to get HEAD");
    let head_commit = head.peel_to_commit().expect("Failed to get HEAD commit");

    let commit = commit_from_git2(&head_commit);

    // Verify the commit has valid structure
    assert!(
        Commit::is_valid_sha(&commit.sha),
        "HEAD commit SHA should be valid"
    );
    assert!(
        !commit.message.is_empty(),
        "HEAD commit should have a message"
    );
    assert!(
        !commit.author.is_empty(),
        "HEAD commit should have an author"
    );

    // HEAD commit should have at least one parent (unless it's the initial commit)
    // For this repo, we know there are commits
    println!("HEAD commit: {} - {}", commit.short_sha(), commit.subject());
}

#[test]
fn test_commit_chain_traversal() {
    let repo_path = workspace_root();
    let repo = Repository::open(&repo_path).expect("Failed to open git repository");

    let head = repo.head().expect("Failed to get HEAD");
    let _head_commit = head.peel_to_commit().expect("Failed to get HEAD commit");

    // Walk the commit history (up to 10 commits for test speed)
    let mut revwalk = repo.revwalk().expect("Failed to create revwalk");
    revwalk.push_head().expect("Failed to push HEAD");

    let mut commits: Vec<Commit> = Vec::new();
    for (i, oid) in revwalk.enumerate() {
        if i >= 10 {
            break;
        }
        let oid = oid.expect("Failed to get OID");
        let git_commit = repo.find_commit(oid).expect("Failed to find commit");
        commits.push(commit_from_git2(&git_commit));
    }

    // Should have at least one commit
    assert!(!commits.is_empty(), "Should have at least one commit");

    // Each commit's parents should be valid SHAs (if present)
    for commit in &commits {
        for parent_sha in &commit.parents {
            assert!(
                Commit::is_valid_sha(parent_sha),
                "Parent SHA should be valid: {}",
                parent_sha
            );
        }
    }

    // Commits should be in chronological order (newest first)
    for window in commits.windows(2) {
        assert!(
            window[0].timestamp >= window[1].timestamp,
            "Commits should be ordered newest first"
        );
    }

    println!("Traversed {} commits", commits.len());
}

#[test]
fn test_commit_serialization_from_real_data() {
    let repo_path = workspace_root();
    let repo = Repository::open(&repo_path).expect("Failed to open git repository");

    let head = repo.head().expect("Failed to get HEAD");
    let head_commit = head.peel_to_commit().expect("Failed to get HEAD commit");

    let commit = commit_from_git2(&head_commit);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&commit).expect("Failed to serialize commit");

    // Verify JSON contains expected fields
    assert!(json.contains("\"sha\":"), "JSON should contain sha field");
    assert!(
        json.contains("\"message\":"),
        "JSON should contain message field"
    );
    assert!(
        json.contains("\"author\":"),
        "JSON should contain author field"
    );
    assert!(
        json.contains("\"timestamp\":"),
        "JSON should contain timestamp field"
    );

    // Deserialize and verify round-trip
    let deserialized: Commit = serde_json::from_str(&json).expect("Failed to deserialize commit");
    assert_eq!(commit, deserialized, "Round-trip should preserve commit");

    println!("Serialized commit:\n{}", json);
}

#[test]
fn test_find_merge_commits() {
    let repo_path = workspace_root();
    let repo = Repository::open(&repo_path).expect("Failed to open git repository");

    let mut revwalk = repo.revwalk().expect("Failed to create revwalk");
    revwalk.push_head().expect("Failed to push HEAD");

    let mut merge_commits: Vec<Commit> = Vec::new();
    let mut regular_commits: Vec<Commit> = Vec::new();

    for (i, oid) in revwalk.enumerate() {
        if i >= 50 {
            break; // Limit for test speed
        }
        let oid = oid.expect("Failed to get OID");
        let git_commit = repo.find_commit(oid).expect("Failed to find commit");
        let commit = commit_from_git2(&git_commit);

        if commit.is_merge() {
            merge_commits.push(commit);
        } else {
            regular_commits.push(commit);
        }
    }

    println!(
        "Found {} merge commits and {} regular commits",
        merge_commits.len(),
        regular_commits.len()
    );

    // Verify merge commits have multiple parents
    for merge in &merge_commits {
        assert!(
            merge.parents.len() > 1,
            "Merge commit should have multiple parents"
        );
    }

    // Verify regular commits have 0 or 1 parent
    for regular in &regular_commits {
        assert!(
            regular.parents.len() <= 1,
            "Regular commit should have at most one parent"
        );
    }
}

#[test]
fn test_commit_timestamps_are_iso8601() {
    let repo_path = workspace_root();
    let repo = Repository::open(&repo_path).expect("Failed to open git repository");

    let head = repo.head().expect("Failed to get HEAD");
    let head_commit = head.peel_to_commit().expect("Failed to get HEAD commit");

    let commit = commit_from_git2(&head_commit);
    let json = serde_json::to_string(&commit).expect("Failed to serialize commit");

    // chrono serializes to RFC 3339 / ISO 8601 format
    // Example: "2026-01-17T02:33:06Z"
    let json_value: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse JSON");
    let timestamp_str = json_value["timestamp"]
        .as_str()
        .expect("timestamp should be a string");

    // Verify ISO 8601 format (should contain T and end with Z or timezone offset)
    assert!(
        timestamp_str.contains('T'),
        "Timestamp should be in ISO 8601 format: {}",
        timestamp_str
    );

    // Should be parseable back to DateTime
    let parsed: DateTime<Utc> = serde_json::from_value(json_value["timestamp"].clone())
        .expect("Should be able to parse timestamp");
    assert_eq!(
        parsed, commit.timestamp,
        "Parsed timestamp should match original"
    );
}
