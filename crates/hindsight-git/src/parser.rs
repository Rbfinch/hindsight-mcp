// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Git log parsing utilities
//!
//! This module provides functionality to parse git commits from a repository
//! using the `git2` crate.

use crate::commit::Commit;
use crate::error::GitError;
use chrono::{DateTime, TimeZone, Utc};
use git2::{DiffOptions, Repository, Sort};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for walking commits
#[derive(Debug, Clone, Default)]
pub struct WalkOptions {
    /// Maximum number of commits to retrieve
    pub limit: Option<usize>,
    /// Start from this commit (defaults to HEAD)
    pub from_ref: Option<String>,
    /// Only include commits after this date
    pub since: Option<DateTime<Utc>>,
    /// Only include commits before this date
    pub until: Option<DateTime<Utc>>,
    /// Include diff information for each commit
    pub include_diff: bool,
}

impl WalkOptions {
    /// Create options for walking the N most recent commits
    #[must_use]
    pub fn latest(n: usize) -> Self {
        Self {
            limit: Some(n),
            ..Default::default()
        }
    }

    /// Create options with diff extraction enabled
    #[must_use]
    pub fn with_diff(mut self) -> Self {
        self.include_diff = true;
        self
    }

    /// Set the starting reference
    #[must_use]
    pub fn from(mut self, reference: &str) -> Self {
        self.from_ref = Some(reference.to_string());
        self
    }

    /// Filter commits since a date
    #[must_use]
    pub fn since(mut self, date: DateTime<Utc>) -> Self {
        self.since = Some(date);
        self
    }

    /// Filter commits until a date
    #[must_use]
    pub fn until(mut self, date: DateTime<Utc>) -> Self {
        self.until = Some(date);
        self
    }
}

/// Represents file changes in a commit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiff {
    /// Path to the file
    pub path: String,
    /// Change status: "added", "modified", "deleted", "renamed"
    pub status: String,
    /// Number of lines added
    pub insertions: usize,
    /// Number of lines deleted
    pub deletions: usize,
}

/// Summary of all changes in a commit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of files changed
    pub files_changed: usize,
    /// Total lines added
    pub insertions: usize,
    /// Total lines deleted
    pub deletions: usize,
    /// Per-file changes
    pub files: Vec<FileDiff>,
}

impl DiffSummary {
    /// Create an empty diff summary
    #[must_use]
    pub fn empty() -> Self {
        Self {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            files: Vec::new(),
        }
    }
}

/// A commit with optional diff information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitWithDiff {
    /// The commit data
    #[serde(flatten)]
    pub commit: Commit,
    /// Diff summary (if requested)
    pub diff: Option<DiffSummary>,
}

/// A git repository wrapper for parsing commits
pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    /// Open a git repository at the given path
    ///
    /// # Errors
    ///
    /// Returns `GitError::RepositoryNotFound` if the path is not a git repository.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let path = path.as_ref();
        let repo = Repository::open(path).map_err(|_| GitError::RepositoryNotFound {
            path: path.display().to_string(),
        })?;
        Ok(Self { repo })
    }

    /// Discover and open a git repository containing the given path
    ///
    /// This walks up the directory tree to find a `.git` directory.
    ///
    /// # Errors
    ///
    /// Returns `GitError::RepositoryNotFound` if no repository is found.
    pub fn discover(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let path = path.as_ref();
        let repo = Repository::discover(path).map_err(|_| GitError::RepositoryNotFound {
            path: path.display().to_string(),
        })?;
        Ok(Self { repo })
    }

    /// Check if the repository is bare
    #[must_use]
    pub fn is_bare(&self) -> bool {
        self.repo.is_bare()
    }

    /// Get the repository path
    #[must_use]
    pub fn path(&self) -> &Path {
        self.repo.path()
    }

    /// Get the working directory path (None for bare repos)
    #[must_use]
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Walk commits according to the given options
    ///
    /// # Errors
    ///
    /// Returns `GitError` if the repository cannot be walked.
    pub fn walk_commits(&self, options: &WalkOptions) -> Result<Vec<CommitWithDiff>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)?;

        // Start from specified ref or HEAD
        if let Some(ref from_ref) = options.from_ref {
            let oid = self.repo.revparse_single(from_ref)?.id();
            revwalk.push(oid)?;
        } else {
            revwalk.push_head()?;
        }

        let mut commits = Vec::new();
        let limit = options.limit.unwrap_or(usize::MAX);

        for oid_result in revwalk {
            if commits.len() >= limit {
                break;
            }

            let oid = oid_result?;
            let git_commit = self.repo.find_commit(oid)?;

            // Convert timestamp
            let time = git_commit.time();
            let timestamp = Utc
                .timestamp_opt(time.seconds(), 0)
                .single()
                .unwrap_or_else(Utc::now);

            // Apply date filters
            if let Some(since) = options.since {
                if timestamp < since {
                    continue;
                }
            }
            if let Some(until) = options.until {
                if timestamp > until {
                    continue;
                }
            }

            // Extract commit data
            let commit = self.extract_commit(&git_commit, timestamp)?;

            // Extract diff if requested
            let diff = if options.include_diff {
                Some(self.extract_diff(&git_commit)?)
            } else {
                None
            };

            commits.push(CommitWithDiff { commit, diff });
        }

        Ok(commits)
    }

    /// Extract commit metadata from a git2 commit
    fn extract_commit(
        &self,
        git_commit: &git2::Commit<'_>,
        timestamp: DateTime<Utc>,
    ) -> Result<Commit, GitError> {
        let sha = git_commit.id().to_string();
        let message = git_commit.message().unwrap_or("").to_string();
        let author = git_commit.author().name().unwrap_or("Unknown").to_string();
        let author_email = git_commit.author().email().unwrap_or("").to_string();

        let parents: Vec<String> = git_commit.parents().map(|p| p.id().to_string()).collect();

        Ok(Commit {
            sha,
            message,
            author,
            author_email,
            timestamp,
            parents,
        })
    }

    /// Extract diff summary for a commit
    fn extract_diff(&self, git_commit: &git2::Commit<'_>) -> Result<DiffSummary, GitError> {
        let tree = git_commit.tree()?;

        // Get parent tree (or empty for root commits)
        let parent_tree = if git_commit.parent_count() > 0 {
            Some(git_commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut opts = DiffOptions::new();
        opts.ignore_whitespace(false);

        let diff =
            self.repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;

        let stats = diff.stats()?;
        let mut files = Vec::new();

        for delta in diff.deltas() {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<unknown>".to_string());

            let status = match delta.status() {
                git2::Delta::Added => "added",
                git2::Delta::Deleted => "deleted",
                git2::Delta::Modified => "modified",
                git2::Delta::Renamed => "renamed",
                git2::Delta::Copied => "copied",
                _ => "unknown",
            }
            .to_string();

            // Note: per-file stats require iterating hunks, which is expensive
            // We'll use 0 for per-file and rely on total stats
            files.push(FileDiff {
                path,
                status,
                insertions: 0,
                deletions: 0,
            });
        }

        Ok(DiffSummary {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            files,
        })
    }

    /// Get a single commit by SHA or reference
    ///
    /// # Errors
    ///
    /// Returns `GitError::InvalidReference` if the reference cannot be resolved.
    pub fn get_commit(&self, reference: &str) -> Result<CommitWithDiff, GitError> {
        let obj = self
            .repo
            .revparse_single(reference)
            .map_err(|_| GitError::InvalidReference {
                reference: reference.to_string(),
            })?;

        let git_commit = obj
            .peel_to_commit()
            .map_err(|_| GitError::InvalidReference {
                reference: reference.to_string(),
            })?;

        let time = git_commit.time();
        let timestamp = Utc
            .timestamp_opt(time.seconds(), 0)
            .single()
            .unwrap_or_else(Utc::now);

        let commit = self.extract_commit(&git_commit, timestamp)?;
        let diff = Some(self.extract_diff(&git_commit)?);

        Ok(CommitWithDiff { commit, diff })
    }

    /// Get the HEAD commit SHA
    ///
    /// # Errors
    ///
    /// Returns `GitError` if HEAD cannot be resolved.
    pub fn head_sha(&self) -> Result<String, GitError> {
        let head = self.repo.head()?;
        let oid = head.target().ok_or_else(|| GitError::InvalidReference {
            reference: "HEAD".to_string(),
        })?;
        Ok(oid.to_string())
    }

    /// Count commits in the repository
    ///
    /// # Errors
    ///
    /// Returns `GitError` if the repository cannot be walked.
    pub fn commit_count(&self) -> Result<usize, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        Ok(revwalk.count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;
    use std::env;

    fn get_repo() -> GitRepo {
        // Find the hindsight-mcp repo root
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let repo_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
        GitRepo::open(repo_root).expect("Should open repo")
    }

    #[test]
    fn test_open_repository() {
        let repo = get_repo();
        assert!(!repo.is_bare());
        assert!(repo.workdir().is_some());
    }

    #[test]
    fn test_discover_repository() {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let repo = GitRepo::discover(&manifest_dir).expect("Should discover repo");
        assert!(!repo.is_bare());
    }

    #[test]
    fn test_open_nonexistent_repository() {
        let result = GitRepo::open("/nonexistent/path");
        assert!(result.is_err());
        match result {
            Err(GitError::RepositoryNotFound { path }) => {
                assert!(path.contains("nonexistent"));
            }
            _ => panic!("Expected RepositoryNotFound error"),
        }
    }

    #[test]
    fn test_head_sha() {
        let repo = get_repo();
        let sha = repo.head_sha().expect("Should get HEAD");
        assert!(Commit::is_valid_sha(&sha), "HEAD SHA should be valid");
    }

    #[test]
    fn test_walk_commits_limit() {
        let repo = get_repo();
        let options = WalkOptions::latest(5);
        let commits = repo.walk_commits(&options).expect("Should walk commits");
        assert!(commits.len() <= 5);
        assert!(!commits.is_empty());
    }

    #[test]
    fn test_walk_commits_with_diff() {
        let repo = get_repo();
        let options = WalkOptions::latest(3).with_diff();
        let commits = repo.walk_commits(&options).expect("Should walk commits");

        assert!(!commits.is_empty());
        for cwc in &commits {
            assert!(cwc.diff.is_some(), "Diff should be included");
        }
    }

    #[test]
    fn test_commit_extraction_fields() {
        let repo = get_repo();
        let options = WalkOptions::latest(1);
        let commits = repo.walk_commits(&options).expect("Should walk commits");

        assert_eq!(commits.len(), 1);
        let commit = &commits[0].commit;

        assert!(Commit::is_valid_sha(&commit.sha));
        assert!(!commit.message.is_empty());
        assert!(!commit.author.is_empty());
    }

    #[test]
    fn test_get_commit_by_sha() {
        let repo = get_repo();
        let head_sha = repo.head_sha().expect("Should get HEAD");

        let commit = repo.get_commit(&head_sha).expect("Should get commit");
        assert_eq!(commit.commit.sha, head_sha);
        assert!(commit.diff.is_some());
    }

    #[test]
    fn test_get_commit_by_ref() {
        let repo = get_repo();
        let commit = repo.get_commit("HEAD").expect("Should get HEAD commit");
        assert!(Commit::is_valid_sha(&commit.commit.sha));
    }

    #[test]
    fn test_get_invalid_reference() {
        let repo = get_repo();
        let result = repo.get_commit("nonexistent-ref-12345");
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_count() {
        let repo = get_repo();
        let count = repo.commit_count().expect("Should count commits");
        assert!(count > 0, "Repository should have commits");
    }

    #[test]
    fn test_walk_options_builder() {
        let options = WalkOptions::latest(10).with_diff().from("main");

        assert_eq!(options.limit, Some(10));
        assert!(options.include_diff);
        assert_eq!(options.from_ref, Some("main".to_string()));
    }

    #[test]
    fn test_diff_summary_serialization() {
        let diff = DiffSummary {
            files_changed: 3,
            insertions: 42,
            deletions: 7,
            files: vec![FileDiff {
                path: "src/lib.rs".to_string(),
                status: "modified".to_string(),
                insertions: 30,
                deletions: 5,
            }],
        };

        let json = serde_json::to_string(&diff).expect("Should serialize");
        assert!(json.contains("files_changed"));
        assert!(json.contains("insertions"));

        let deserialized: DiffSummary = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(diff, deserialized);
    }

    #[test]
    fn test_commit_with_diff_serialization() {
        let cwc = CommitWithDiff {
            commit: Commit {
                sha: "a".repeat(40),
                message: "Test".to_string(),
                author: "Author".to_string(),
                author_email: "author@example.com".to_string(),
                timestamp: Utc::now(),
                parents: vec![],
            },
            diff: Some(DiffSummary::empty()),
        };

        let json = serde_json::to_string(&cwc).expect("Should serialize");
        assert!(json.contains("sha"));
        assert!(json.contains("diff"));
    }
}
