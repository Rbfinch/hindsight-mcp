// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Unified data ingestion API
//!
//! This module provides a high-level API for ingesting development data from all sources
//! (git, tests, Copilot) into the SQLite database.
//!
//! # Example
//!
//! ```no_run
//! use hindsight_mcp::db::Database;
//! use hindsight_mcp::ingest::{Ingestor, IngestOptions};
//!
//! let mut db = Database::in_memory().expect("create db");
//! db.initialize().expect("init");
//! let mut ingestor = Ingestor::new(db);
//!
//! // Ingest git commits
//! let stats = ingestor.ingest_git("/path/to/repo", &IngestOptions::default())
//!     .expect("ingest git");
//! println!("Ingested {} commits", stats.commits_inserted);
//! ```

use std::path::Path;

use thiserror::Error;
use tracing::{debug, info, warn};

use crate::db::{
    CommitRecord, CopilotMessageRecord, CopilotSessionRecord, Database, DbError, TestResultRecord,
    TestRunRecord,
};
use hindsight_tests::TestOutcome;

// ============================================================================
// Error Types
// ============================================================================

/// Ingestion errors
#[derive(Debug, Error)]
pub enum IngestError {
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] DbError),

    /// Git error
    #[error("Git error: {0}")]
    Git(#[from] hindsight_git::GitError),

    /// Copilot error
    #[error("Copilot error: {0}")]
    Copilot(#[from] hindsight_copilot::CopilotError),

    /// Tests error
    #[error("Tests error: {0}")]
    Tests(#[from] hindsight_tests::TestsError),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Workspace not found
    #[error("Workspace not found: {path}")]
    WorkspaceNotFound {
        /// The workspace path that could not be found
        path: String,
    },
}

// ============================================================================
// Progress Reporting
// ============================================================================

/// Progress callback signature
pub type ProgressCallback = Box<dyn Fn(&ProgressEvent) + Send + Sync>;

/// Progress event during ingestion
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Starting ingestion of a source
    Started {
        /// Name of the data source being ingested
        source: String,
        /// Total number of items to process, if known
        total_items: Option<usize>,
    },
    /// Item processed
    Progress {
        /// Name of the data source being ingested
        source: String,
        /// Number of items processed so far
        processed: usize,
        /// Total number of items, if known
        total: Option<usize>,
    },
    /// Non-fatal error occurred
    Warning {
        /// Name of the data source where the warning occurred
        source: String,
        /// Description of the warning
        message: String,
    },
    /// Ingestion completed
    Completed {
        /// Name of the data source that completed
        source: String,
        /// Statistics from the ingestion
        stats: IngestStats,
    },
}

// ============================================================================
// Options and Statistics
// ============================================================================

/// Options for git ingestion
#[derive(Debug, Clone, Default)]
pub struct IngestOptions {
    /// Maximum commits to ingest (None = all available)
    pub commit_limit: Option<usize>,
    /// Include diff information for commits
    pub include_diffs: bool,
    /// Skip commits already in database (incremental sync)
    pub incremental: bool,
}

impl IngestOptions {
    /// Create options for full ingestion
    #[must_use]
    pub fn full() -> Self {
        Self {
            commit_limit: None,
            include_diffs: true,
            incremental: false,
        }
    }

    /// Create options for incremental sync
    #[must_use]
    pub fn incremental() -> Self {
        Self {
            commit_limit: None,
            include_diffs: true,
            incremental: true,
        }
    }

    /// Limit to N most recent commits
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.commit_limit = Some(limit);
        self
    }

    /// Include diff information
    #[must_use]
    pub fn with_diffs(mut self) -> Self {
        self.include_diffs = true;
        self
    }
}

/// Statistics from an ingestion operation
#[derive(Debug, Clone, Default)]
pub struct IngestStats {
    /// Number of commits inserted
    pub commits_inserted: usize,
    /// Number of commits skipped (already exist)
    pub commits_skipped: usize,
    /// Number of test runs inserted
    pub test_runs_inserted: usize,
    /// Number of test results inserted
    pub test_results_inserted: usize,
    /// Number of Copilot sessions inserted
    pub sessions_inserted: usize,
    /// Number of Copilot messages inserted
    pub messages_inserted: usize,
    /// Number of warnings/errors encountered
    pub warnings: usize,
}

impl IngestStats {
    /// Total items processed
    #[must_use]
    pub fn total_items(&self) -> usize {
        self.commits_inserted
            + self.test_runs_inserted
            + self.test_results_inserted
            + self.sessions_inserted
            + self.messages_inserted
    }

    /// Merge stats from another operation
    pub fn merge(&mut self, other: &IngestStats) {
        self.commits_inserted += other.commits_inserted;
        self.commits_skipped += other.commits_skipped;
        self.test_runs_inserted += other.test_runs_inserted;
        self.test_results_inserted += other.test_results_inserted;
        self.sessions_inserted += other.sessions_inserted;
        self.messages_inserted += other.messages_inserted;
        self.warnings += other.warnings;
    }
}

// ============================================================================
// Ingestor
// ============================================================================

/// High-level API for ingesting development data
pub struct Ingestor {
    db: Database,
    progress: Option<ProgressCallback>,
}

impl Ingestor {
    /// Create a new ingestor with the given database
    #[must_use]
    pub fn new(db: Database) -> Self {
        Self { db, progress: None }
    }

    /// Set a progress callback
    pub fn with_progress(mut self, callback: ProgressCallback) -> Self {
        self.progress = Some(callback);
        self
    }

    /// Report progress event
    fn report(&self, event: ProgressEvent) {
        if let Some(ref callback) = self.progress {
            callback(&event);
        }
    }

    /// Get mutable reference to the database
    pub fn database_mut(&mut self) -> &mut Database {
        &mut self.db
    }

    /// Get reference to the database
    pub fn database(&self) -> &Database {
        &self.db
    }

    // ========================================================================
    // Git Ingestion
    // ========================================================================

    /// Ingest git commits from a repository
    ///
    /// # Errors
    ///
    /// Returns an error if the repository cannot be opened or commits cannot be inserted.
    pub fn ingest_git(
        &mut self,
        repo_path: impl AsRef<Path>,
        options: &IngestOptions,
    ) -> Result<IngestStats, IngestError> {
        let repo_path = repo_path.as_ref();
        let repo_path_str = repo_path.display().to_string();

        info!(path = %repo_path_str, "Starting git ingestion");

        // Get or create workspace
        let workspace_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let workspace_id = self
            .db
            .get_or_create_workspace(workspace_name, &repo_path_str)?;

        // Open repository
        let git_repo = hindsight_git::GitRepo::open(repo_path)?;

        // Build walk options
        let mut walk_opts = if let Some(limit) = options.commit_limit {
            hindsight_git::WalkOptions::latest(limit)
        } else {
            hindsight_git::WalkOptions::default()
        };

        if options.include_diffs {
            walk_opts = walk_opts.with_diff();
        }

        // Get last ingested SHA for incremental sync
        let last_sha = if options.incremental {
            self.get_last_ingested_sha(&workspace_id)?
        } else {
            None
        };

        // Walk commits
        let commits = git_repo.walk_commits(&walk_opts)?;
        let total = commits.len();

        self.report(ProgressEvent::Started {
            source: "git".to_string(),
            total_items: Some(total),
        });

        let mut stats = IngestStats::default();
        let mut records = Vec::new();

        for (idx, commit_with_diff) in commits.into_iter().enumerate() {
            let commit = &commit_with_diff.commit;

            // Stop at last ingested SHA for incremental sync
            if let Some(ref last) = last_sha
                && &commit.sha == last
            {
                debug!(sha = %commit.sha, "Reached last ingested commit, stopping");
                break;
            }

            // Check if commit already exists
            if options.incremental
                && self
                    .db
                    .get_commit_by_sha(&workspace_id, &commit.sha)
                    .is_ok()
            {
                stats.commits_skipped += 1;
                continue;
            }

            // Convert to record
            let mut record = CommitRecord::new(
                workspace_id.clone(),
                commit.sha.clone(),
                commit.author.clone(),
                Some(commit.author_email.clone()),
                commit.message.clone(),
                commit.timestamp,
            )
            .with_parents(commit.parents.clone());

            // Add diff if available
            if let Some(ref diff) = commit_with_diff.diff {
                let diff_json = serde_json::to_string(diff)?;
                record = record.with_diff_json(diff_json);
            }

            records.push(record);

            // Report progress every 10 items
            if (idx + 1) % 10 == 0 {
                self.report(ProgressEvent::Progress {
                    source: "git".to_string(),
                    processed: idx + 1,
                    total: Some(total),
                });
            }
        }

        // Batch insert
        let inserted = self.db.insert_commits_batch(&records)?;
        stats.commits_inserted = inserted;

        info!(
            inserted = inserted,
            skipped = stats.commits_skipped,
            "Git ingestion complete"
        );

        self.report(ProgressEvent::Completed {
            source: "git".to_string(),
            stats: stats.clone(),
        });

        Ok(stats)
    }

    /// Get the SHA of the most recently ingested commit for a workspace
    fn get_last_ingested_sha(&self, workspace_id: &str) -> Result<Option<String>, IngestError> {
        let result: Result<String, _> = self.db.connection().query_row(
            "SELECT sha FROM commits WHERE workspace_id = ?1 ORDER BY timestamp DESC LIMIT 1",
            [workspace_id],
            |row| row.get(0),
        );

        match result {
            Ok(sha) => Ok(Some(sha)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(IngestError::Database(DbError::Sqlite(e))),
        }
    }

    // ========================================================================
    // Test Result Ingestion
    // ========================================================================

    /// Ingest test results from nextest output
    ///
    /// # Errors
    ///
    /// Returns an error if the output cannot be parsed or results cannot be inserted.
    pub fn ingest_tests(
        &mut self,
        workspace_path: impl AsRef<Path>,
        nextest_output: &str,
        commit_sha: Option<&str>,
    ) -> Result<IngestStats, IngestError> {
        let workspace_path = workspace_path.as_ref();
        let workspace_path_str = workspace_path.display().to_string();

        info!(path = %workspace_path_str, "Starting test ingestion");

        // Get or create workspace
        let workspace_name = workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let workspace_id = self
            .db
            .get_or_create_workspace(workspace_name, &workspace_path_str)?;

        // Parse nextest output
        let summary = hindsight_tests::parse_run_output(nextest_output)?;

        self.report(ProgressEvent::Started {
            source: "tests".to_string(),
            total_items: Some(summary.results.len()),
        });

        let mut stats = IngestStats::default();

        // Create test run record
        let run_record = TestRunRecord::new(workspace_id.clone()).finished(
            summary.passed as i32,
            summary.failed as i32,
            summary.ignored as i32,
        );

        let run_record = if let Some(sha) = commit_sha {
            run_record.with_commit(sha)
        } else {
            run_record
        };

        let run_id = self.db.insert_test_run(&run_record)?;
        stats.test_runs_inserted = 1;

        // Convert results to records
        let result_records: Vec<TestResultRecord> = summary
            .results
            .iter()
            .map(|r| {
                let (suite_name, test_name) = split_test_name(&r.name);
                let duration_ms = Some(r.duration_ms as i64);
                let outcome = outcome_to_string(&r.outcome);

                let mut record = TestResultRecord::new(
                    run_id.clone(),
                    suite_name,
                    test_name,
                    outcome,
                    duration_ms,
                );

                if let Some(ref output) = r.output {
                    record = record.with_output(Some(output.as_str()), None);
                }

                record
            })
            .collect();

        // Batch insert results
        let inserted = self.db.insert_test_results_batch(&result_records)?;
        stats.test_results_inserted = inserted;

        info!(
            run_id = %run_id,
            results = inserted,
            "Test ingestion complete"
        );

        self.report(ProgressEvent::Completed {
            source: "tests".to_string(),
            stats: stats.clone(),
        });

        Ok(stats)
    }

    // ========================================================================
    // Copilot Ingestion
    // ========================================================================

    /// Ingest Copilot sessions from VS Code storage
    ///
    /// # Errors
    ///
    /// Returns an error if sessions cannot be discovered or inserted.
    pub fn ingest_copilot(
        &mut self,
        workspace_path: impl AsRef<Path>,
    ) -> Result<IngestStats, IngestError> {
        let workspace_path = workspace_path.as_ref();
        let workspace_path_str = workspace_path.display().to_string();

        info!(path = %workspace_path_str, "Starting Copilot ingestion");

        // Get or create workspace
        let workspace_name = workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let workspace_id = self
            .db
            .get_or_create_workspace(workspace_name, &workspace_path_str)?;

        // Discover sessions
        let discovery = hindsight_copilot::SessionDiscovery::new()?;
        let sessions = discovery.discover_sessions_for_workspace(workspace_path)?;

        self.report(ProgressEvent::Started {
            source: "copilot".to_string(),
            total_items: Some(sessions.len()),
        });

        let mut stats = IngestStats::default();

        for (idx, discovered) in sessions.iter().enumerate() {
            match self.ingest_single_session(&workspace_id, discovered) {
                Ok(session_stats) => {
                    stats.merge(&session_stats);
                }
                Err(e) => {
                    warn!(
                        path = %discovered.path.display(),
                        error = %e,
                        "Failed to ingest session"
                    );
                    stats.warnings += 1;
                    self.report(ProgressEvent::Warning {
                        source: "copilot".to_string(),
                        message: format!("Failed to ingest {}: {}", discovered.session_id, e),
                    });
                }
            }

            // Report progress
            if (idx + 1) % 5 == 0 || idx == sessions.len() - 1 {
                self.report(ProgressEvent::Progress {
                    source: "copilot".to_string(),
                    processed: idx + 1,
                    total: Some(sessions.len()),
                });
            }
        }

        info!(
            sessions = stats.sessions_inserted,
            messages = stats.messages_inserted,
            "Copilot ingestion complete"
        );

        self.report(ProgressEvent::Completed {
            source: "copilot".to_string(),
            stats: stats.clone(),
        });

        Ok(stats)
    }

    /// Ingest a single Copilot session
    fn ingest_single_session(
        &mut self,
        workspace_id: &str,
        discovered: &hindsight_copilot::DiscoveredSession,
    ) -> Result<IngestStats, IngestError> {
        let mut stats = IngestStats::default();

        // Parse session file
        let session = hindsight_copilot::session::parse_session_file(
            &discovered.path,
            &discovered.workspace_storage_id,
        )?;

        // Create session record
        let session_record =
            CopilotSessionRecord::new(workspace_id.to_string(), session.id.clone())
                .with_metadata(session.model.as_deref(), session.mode.as_deref());

        let db_session_id = self.db.insert_copilot_session(&session_record)?;

        // Check if session was newly inserted (not a duplicate)
        let existing_count = self.db.get_session_message_count(&db_session_id)?;
        if existing_count > 0 {
            debug!(session_id = %session.id, "Session already ingested, skipping");
            return Ok(stats);
        }

        stats.sessions_inserted = 1;

        // Convert messages to records
        let message_records: Vec<CopilotMessageRecord> = session
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    hindsight_copilot::MessageRole::User => "user",
                    hindsight_copilot::MessageRole::Assistant => "assistant",
                    hindsight_copilot::MessageRole::System => "system",
                };

                let mut record = CopilotMessageRecord::new(
                    db_session_id.clone(),
                    role.to_string(),
                    m.content.clone(),
                    m.timestamp,
                );

                // Add variables if present
                if !m.variables.is_empty()
                    && let Ok(json) = serde_json::to_string(&m.variables)
                {
                    record = record.with_variables_json(json);
                }

                record
            })
            .collect();

        // Batch insert messages
        let inserted = self.db.insert_copilot_messages_batch(&message_records)?;
        stats.messages_inserted = inserted;

        Ok(stats)
    }

    // ========================================================================
    // Unified Ingestion
    // ========================================================================

    /// Ingest all available data sources for a workspace
    ///
    /// This combines git, tests (if available), and Copilot sessions.
    ///
    /// # Errors
    ///
    /// Returns an error if any ingestion fails critically. Non-fatal errors
    /// are reported as warnings in the stats.
    pub fn ingest_all(
        &mut self,
        workspace_path: impl AsRef<Path>,
        options: &IngestOptions,
    ) -> Result<IngestStats, IngestError> {
        let workspace_path = workspace_path.as_ref();
        let mut total_stats = IngestStats::default();

        info!(path = %workspace_path.display(), "Starting unified ingestion");

        // Ingest git
        match self.ingest_git(workspace_path, options) {
            Ok(stats) => total_stats.merge(&stats),
            Err(e) => {
                warn!(error = %e, "Git ingestion failed");
                total_stats.warnings += 1;
            }
        }

        // Ingest Copilot sessions
        match self.ingest_copilot(workspace_path) {
            Ok(stats) => total_stats.merge(&stats),
            Err(e) => {
                warn!(error = %e, "Copilot ingestion failed");
                total_stats.warnings += 1;
            }
        }

        info!(
            commits = total_stats.commits_inserted,
            sessions = total_stats.sessions_inserted,
            messages = total_stats.messages_inserted,
            warnings = total_stats.warnings,
            "Unified ingestion complete"
        );

        Ok(total_stats)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert TestOutcome to a string representation
fn outcome_to_string(outcome: &TestOutcome) -> String {
    match outcome {
        TestOutcome::Passed => "passed".to_string(),
        TestOutcome::Failed => "failed".to_string(),
        TestOutcome::Ignored => "ignored".to_string(),
        TestOutcome::TimedOut => "timed_out".to_string(),
    }
}

/// Split a test name into suite and test parts
///
/// Nextest names are like "crate_name::module::test_name"
fn split_test_name(name: &str) -> (String, String) {
    if let Some(pos) = name.find("::") {
        let suite = name[..pos].to_string();
        let test = name[pos + 2..].to_string();
        (suite, test)
    } else {
        (String::new(), name.to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_test_name_with_module() {
        let (suite, test) = split_test_name("hindsight_git::parser::tests::test_open");
        assert_eq!(suite, "hindsight_git");
        assert_eq!(test, "parser::tests::test_open");
    }

    #[test]
    fn test_split_test_name_no_module() {
        let (suite, test) = split_test_name("test_simple");
        assert_eq!(suite, "");
        assert_eq!(test, "test_simple");
    }

    #[test]
    fn test_ingest_options_default() {
        let opts = IngestOptions::default();
        assert!(opts.commit_limit.is_none());
        assert!(!opts.include_diffs);
        assert!(!opts.incremental);
    }

    #[test]
    fn test_ingest_options_full() {
        let opts = IngestOptions::full();
        assert!(opts.commit_limit.is_none());
        assert!(opts.include_diffs);
        assert!(!opts.incremental);
    }

    #[test]
    fn test_ingest_options_incremental() {
        let opts = IngestOptions::incremental();
        assert!(opts.commit_limit.is_none());
        assert!(opts.include_diffs);
        assert!(opts.incremental);
    }

    #[test]
    fn test_ingest_options_with_limit() {
        let opts = IngestOptions::default().with_limit(50);
        assert_eq!(opts.commit_limit, Some(50));
    }

    #[test]
    fn test_ingest_stats_merge() {
        let mut stats1 = IngestStats {
            commits_inserted: 10,
            commits_skipped: 5,
            ..Default::default()
        };
        let stats2 = IngestStats {
            commits_inserted: 3,
            sessions_inserted: 2,
            messages_inserted: 20,
            ..Default::default()
        };

        stats1.merge(&stats2);

        assert_eq!(stats1.commits_inserted, 13);
        assert_eq!(stats1.commits_skipped, 5);
        assert_eq!(stats1.sessions_inserted, 2);
        assert_eq!(stats1.messages_inserted, 20);
    }

    #[test]
    fn test_ingest_stats_total_items() {
        let stats = IngestStats {
            commits_inserted: 10,
            test_runs_inserted: 1,
            test_results_inserted: 50,
            sessions_inserted: 3,
            messages_inserted: 30,
            ..Default::default()
        };

        assert_eq!(stats.total_items(), 94);
    }

    #[test]
    fn test_ingestor_new() {
        let db = Database::in_memory().expect("create db");
        let ingestor = Ingestor::new(db);
        assert!(ingestor.progress.is_none());
    }

    #[test]
    fn test_ingestor_with_progress() {
        let db = Database::in_memory().expect("create db");
        let ingestor = Ingestor::new(db).with_progress(Box::new(|_| {}));
        assert!(ingestor.progress.is_some());
    }

    #[test]
    fn test_progress_event_variants() {
        let started = ProgressEvent::Started {
            source: "git".to_string(),
            total_items: Some(100),
        };
        assert!(matches!(started, ProgressEvent::Started { .. }));

        let progress = ProgressEvent::Progress {
            source: "git".to_string(),
            processed: 50,
            total: Some(100),
        };
        assert!(matches!(progress, ProgressEvent::Progress { .. }));

        let warning = ProgressEvent::Warning {
            source: "copilot".to_string(),
            message: "test warning".to_string(),
        };
        assert!(matches!(warning, ProgressEvent::Warning { .. }));

        let completed = ProgressEvent::Completed {
            source: "tests".to_string(),
            stats: IngestStats::default(),
        };
        assert!(matches!(completed, ProgressEvent::Completed { .. }));
    }
}
