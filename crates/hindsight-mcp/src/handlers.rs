//! Tool handlers for the MCP server
//!
//! This module implements the handlers for each MCP tool, bridging
//! MCP requests to database queries and returning formatted responses.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::db::Database;
use crate::ingest::{IngestError, IngestOptions, IngestStats, Ingestor};
use crate::queries::{
    self, ActivitySummary, CommitWithTests, FailingTest, QueryError, SearchResult, TimelineEvent,
};

// ============================================================================
// Error Types
// ============================================================================

/// Handler errors
#[derive(Debug, Error)]
pub enum HandlerError {
    /// Query error
    #[error("Database query failed: {0}. Try running 'hindsight_ingest' to populate the database.")]
    Query(#[from] QueryError),

    /// Ingestion error
    #[error("Data ingestion failed: {0}")]
    Ingest(#[from] IngestError),

    /// Invalid input - missing required field
    #[error("Invalid input: {0}. Check the tool's required parameters.")]
    InvalidInput(String),

    /// JSON serialization error
    #[error("Failed to process JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// Resource not found
    #[error("{0}. Use 'hindsight_search' to find available commits.")]
    NotFound(String),

    /// Workspace not found
    #[error("Workspace not found: {0}. Ensure the path exists and is accessible.")]
    WorkspaceNotFound(String),
}

// ============================================================================
// Input Types
// ============================================================================

/// Input for the timeline tool
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TimelineInput {
    /// Maximum events to return
    #[serde(default = "default_timeline_limit")]
    pub limit: usize,
    /// Filter by workspace path
    pub workspace: Option<String>,
}

fn default_timeline_limit() -> usize {
    50
}

/// Input for the search tool
#[derive(Debug, Clone, Deserialize)]
pub struct SearchInput {
    /// Search query (FTS5 syntax supported)
    pub query: String,
    /// Source to search: "all", "commits", or "messages"
    #[serde(default = "default_search_source")]
    pub source: String,
    /// Maximum results to return
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_source() -> String {
    "all".to_string()
}

fn default_search_limit() -> usize {
    20
}

/// Input for the failing_tests tool
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FailingTestsInput {
    /// Maximum failing tests to return
    #[serde(default = "default_failing_tests_limit")]
    pub limit: usize,
    /// Filter by workspace path
    pub workspace: Option<String>,
    /// Filter by commit SHA (full or partial)
    pub commit: Option<String>,
}

fn default_failing_tests_limit() -> usize {
    50
}

/// Input for the activity_summary tool
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ActivitySummaryInput {
    /// Number of days to summarize
    #[serde(default = "default_activity_days")]
    pub days: u32,
}

fn default_activity_days() -> u32 {
    7
}

/// Input for the commit_details tool
#[derive(Debug, Clone, Deserialize)]
pub struct CommitDetailsInput {
    /// Full or partial commit SHA
    pub sha: String,
}

/// Input for the ingest tool
#[derive(Debug, Clone, Deserialize)]
pub struct IngestInput {
    /// Source to ingest: "git", "tests", "copilot", or "all"
    #[serde(default = "default_ingest_source")]
    pub source: String,
    /// Workspace path to ingest
    pub workspace: String,
    /// Only ingest new data since last run
    #[serde(default = "default_incremental")]
    pub incremental: bool,
    /// Max items to ingest (optional)
    pub limit: Option<usize>,
}

fn default_ingest_source() -> String {
    "all".to_string()
}

fn default_incremental() -> bool {
    true
}

// ============================================================================
// Output Types
// ============================================================================

/// Response from the ingest tool
#[derive(Debug, Clone, Serialize)]
pub struct IngestResponse {
    /// Source that was ingested
    pub source: String,
    /// Statistics from the ingestion
    pub stats: IngestStatsResponse,
    /// Human-readable message
    pub message: String,
}

/// Serializable ingest stats
#[derive(Debug, Clone, Serialize)]
pub struct IngestStatsResponse {
    pub commits_inserted: usize,
    pub commits_skipped: usize,
    pub test_runs_inserted: usize,
    pub test_results_inserted: usize,
    pub sessions_inserted: usize,
    pub messages_inserted: usize,
    pub total_items: usize,
}

impl From<IngestStats> for IngestStatsResponse {
    fn from(stats: IngestStats) -> Self {
        Self {
            commits_inserted: stats.commits_inserted,
            commits_skipped: stats.commits_skipped,
            test_runs_inserted: stats.test_runs_inserted,
            test_results_inserted: stats.test_results_inserted,
            sessions_inserted: stats.sessions_inserted,
            messages_inserted: stats.messages_inserted,
            total_items: stats.total_items(),
        }
    }
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Parse input from MCP arguments into a typed struct
fn parse_input<T: for<'de> Deserialize<'de>>(
    args: Option<Map<String, Value>>,
) -> Result<T, HandlerError> {
    let value = args
        .map(Value::Object)
        .unwrap_or(Value::Object(serde_json::Map::new()));
    serde_json::from_value(value).map_err(|e| HandlerError::InvalidInput(e.to_string()))
}

/// Handle the hindsight_timeline tool
///
/// Returns a chronological view of development activity.
pub fn handle_timeline(
    db: &Database,
    args: Option<Map<String, Value>>,
    default_workspace: Option<&PathBuf>,
) -> Result<Vec<TimelineEvent>, HandlerError> {
    let input: TimelineInput = parse_input(args)?;

    // Use provided workspace or fall back to default
    let workspace_filter = input
        .workspace
        .as_deref()
        .or_else(|| default_workspace.and_then(|p| p.to_str()));

    let events = queries::get_timeline(db.connection(), input.limit, workspace_filter)?;

    Ok(events)
}

/// Handle the hindsight_search tool
///
/// Full-text search across commits and/or messages.
pub fn handle_search(
    db: &Database,
    args: Option<Map<String, Value>>,
) -> Result<Vec<SearchResult>, HandlerError> {
    let input: SearchInput = parse_input(args)?;

    if input.query.is_empty() {
        return Err(HandlerError::InvalidInput(
            "Search query cannot be empty. Provide a search term like 'refactor' or 'fix bug'."
                .to_string(),
        ));
    }

    let results = match input.source.as_str() {
        "commits" => queries::search_commits(db.connection(), &input.query, input.limit)?,
        "messages" => queries::search_messages(db.connection(), &input.query, input.limit)?,
        _ => queries::search_all(db.connection(), &input.query, input.limit)?,
    };

    Ok(results)
}

/// Handle the hindsight_failing_tests tool
///
/// Returns currently failing tests from the most recent test runs.
pub fn handle_failing_tests(
    db: &Database,
    args: Option<Map<String, Value>>,
    default_workspace: Option<&PathBuf>,
) -> Result<Vec<FailingTest>, HandlerError> {
    let input: FailingTestsInput = parse_input(args)?;

    // Use provided workspace or fall back to default
    let workspace_filter = input
        .workspace
        .as_deref()
        .or_else(|| default_workspace.and_then(|p| p.to_str()));

    let tests = queries::get_failing_tests(
        db.connection(),
        input.limit,
        workspace_filter,
        input.commit.as_deref(),
    )?;

    Ok(tests)
}

/// Handle the hindsight_activity_summary tool
///
/// Returns aggregate activity statistics for a time period.
pub fn handle_activity_summary(
    db: &Database,
    args: Option<Map<String, Value>>,
) -> Result<ActivitySummary, HandlerError> {
    let input: ActivitySummaryInput = parse_input(args)?;

    let summary = queries::get_activity_summary(db.connection(), input.days)?;

    Ok(summary)
}

/// Handle the hindsight_commit_details tool
///
/// Returns detailed information about a specific commit including linked test runs.
pub fn handle_commit_details(
    db: &Database,
    args: Option<Map<String, Value>>,
) -> Result<CommitWithTests, HandlerError> {
    let input: CommitDetailsInput = parse_input(args)?;

    if input.sha.is_empty() {
        return Err(HandlerError::InvalidInput(
            "Commit SHA is required. Provide a full or partial SHA like 'abc123' or 'abc123def456789'.".to_string(),
        ));
    }

    let commit = queries::get_commit_with_tests(db.connection(), &input.sha)?;

    commit.ok_or_else(|| HandlerError::NotFound(format!("Commit not found: {}", input.sha)))
}

/// Handle the hindsight_ingest tool
///
/// Triggers data ingestion from sources.
pub fn handle_ingest(
    db: Database,
    args: Option<Map<String, Value>>,
) -> Result<IngestResponse, HandlerError> {
    let input: IngestInput = parse_input(args)?;

    let workspace_path = PathBuf::from(&input.workspace);

    if !workspace_path.exists() {
        return Err(HandlerError::WorkspaceNotFound(input.workspace.clone()));
    }

    if !workspace_path.is_dir() {
        return Err(HandlerError::InvalidInput(format!(
            "Workspace path is not a directory: {}",
            input.workspace
        )));
    }

    // Build ingest options
    let options = if input.incremental {
        IngestOptions::incremental()
    } else {
        IngestOptions::full()
    };

    let options = if let Some(limit) = input.limit {
        options.with_limit(limit)
    } else {
        options
    };

    let mut ingestor = Ingestor::new(db);
    let mut total_stats = IngestStats::default();

    // Ingest based on source
    match input.source.as_str() {
        "git" => {
            let stats = ingestor.ingest_git(&workspace_path, &options)?;
            total_stats.merge(&stats);
        }
        "copilot" => {
            let stats = ingestor.ingest_copilot(&workspace_path)?;
            total_stats.merge(&stats);
        }
        _ => {
            // Ingest all sources (default), collecting stats
            // Note: test ingestion requires nextest output, which we don't have here
            // So we only ingest git and copilot in "all" mode
            if let Ok(stats) = ingestor.ingest_git(&workspace_path, &options) {
                total_stats.merge(&stats);
            }
            if let Ok(stats) = ingestor.ingest_copilot(&workspace_path) {
                total_stats.merge(&stats);
            }
        }
    }

    let message = format!(
        "Ingested {} items from '{}' source",
        total_stats.total_items(),
        input.source
    );

    Ok(IngestResponse {
        source: input.source,
        stats: total_stats.into(),
        message,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Helper to convert a JSON Value to a Map for testing
    fn to_map(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(map) => map,
            _ => panic!("Expected JSON object"),
        }
    }

    #[test]
    fn test_parse_timeline_input_defaults() {
        let input: TimelineInput = parse_input(None).expect("parse");
        assert_eq!(input.limit, 50);
        assert!(input.workspace.is_none());
    }

    #[test]
    fn test_parse_timeline_input_with_values() {
        let args = to_map(json!({
            "limit": 25,
            "workspace": "/path/to/workspace"
        }));
        let input: TimelineInput = parse_input(Some(args)).expect("parse");
        assert_eq!(input.limit, 25);
        assert_eq!(input.workspace, Some("/path/to/workspace".to_string()));
    }

    #[test]
    fn test_parse_search_input() {
        let args = to_map(json!({
            "query": "fix bug",
            "source": "commits",
            "limit": 10
        }));
        let input: SearchInput = parse_input(Some(args)).expect("parse");
        assert_eq!(input.query, "fix bug");
        assert_eq!(input.source, "commits");
        assert_eq!(input.limit, 10);
    }

    #[test]
    fn test_parse_search_input_defaults() {
        let args = to_map(json!({
            "query": "test"
        }));
        let input: SearchInput = parse_input(Some(args)).expect("parse");
        assert_eq!(input.source, "all");
        assert_eq!(input.limit, 20);
    }

    #[test]
    fn test_parse_activity_summary_input_defaults() {
        let input: ActivitySummaryInput = parse_input(None).expect("parse");
        assert_eq!(input.days, 7);
    }

    #[test]
    fn test_parse_commit_details_input() {
        let args = to_map(json!({
            "sha": "abc123"
        }));
        let input: CommitDetailsInput = parse_input(Some(args)).expect("parse");
        assert_eq!(input.sha, "abc123");
    }

    #[test]
    fn test_parse_ingest_input() {
        let args = to_map(json!({
            "workspace": "/path/to/repo",
            "source": "git",
            "incremental": false,
            "limit": 100
        }));
        let input: IngestInput = parse_input(Some(args)).expect("parse");
        assert_eq!(input.workspace, "/path/to/repo");
        assert_eq!(input.source, "git");
        assert!(!input.incremental);
        assert_eq!(input.limit, Some(100));
    }

    #[test]
    fn test_parse_ingest_input_defaults() {
        let args = to_map(json!({
            "workspace": "/path/to/repo"
        }));
        let input: IngestInput = parse_input(Some(args)).expect("parse");
        assert_eq!(input.source, "all");
        assert!(input.incremental);
        assert!(input.limit.is_none());
    }

    #[test]
    fn test_handle_timeline_empty_db() {
        let db = Database::in_memory().expect("create db");
        db.initialize().expect("init db");
        let events = handle_timeline(&db, None, None).expect("handle");
        assert!(events.is_empty());
    }

    #[test]
    fn test_handle_search_empty_query() {
        let db = Database::in_memory().expect("create db");
        db.initialize().expect("init db");
        let args = to_map(json!({ "query": "" }));
        let result = handle_search(&db, Some(args));
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_activity_summary_empty_db() {
        let db = Database::in_memory().expect("create db");
        db.initialize().expect("init db");
        let summary = handle_activity_summary(&db, None).expect("handle");
        assert_eq!(summary.commits, 0);
        assert_eq!(summary.test_runs, 0);
        assert_eq!(summary.copilot_sessions, 0);
    }

    #[test]
    fn test_handle_commit_details_not_found() {
        let db = Database::in_memory().expect("create db");
        db.initialize().expect("init db");
        let args = to_map(json!({ "sha": "nonexistent" }));
        let result = handle_commit_details(&db, Some(args));
        assert!(matches!(result, Err(HandlerError::NotFound(_))));
    }

    #[test]
    fn test_handle_commit_details_empty_sha() {
        let db = Database::in_memory().expect("create db");
        db.initialize().expect("init db");
        let args = to_map(json!({ "sha": "" }));
        let result = handle_commit_details(&db, Some(args));
        assert!(matches!(result, Err(HandlerError::InvalidInput(_))));
    }

    #[test]
    fn test_ingest_stats_response_conversion() {
        let stats = IngestStats {
            commits_inserted: 10,
            commits_skipped: 5,
            test_runs_inserted: 3,
            test_results_inserted: 30,
            sessions_inserted: 2,
            messages_inserted: 20,
            warnings: 0,
        };

        let response: IngestStatsResponse = stats.into();
        assert_eq!(response.commits_inserted, 10);
        assert_eq!(response.total_items, 65); // 10 + 3 + 30 + 2 + 20
    }
}
