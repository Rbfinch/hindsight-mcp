//! MCP server implementation for hindsight-mcp
//!
//! This module provides the core MCP server that exposes development history
//! data (git commits, test results, Copilot sessions) to LLMs via MCP tool calls.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, RpcError,
    TextContent, Tool, ToolInputSchema, schema_utils::CallToolError,
};
use serde_json::{Map, Value, json};
use tokio::sync::Mutex;
use tracing::{debug, error, trace};

use crate::db::Database;
use crate::handlers::{self, HandlerError};

/// Convert a JSON object into the properties format expected by ToolInputSchema.
///
/// ToolInputSchema expects `HashMap<String, Map<String, Value>>` for properties,
/// where each key maps to a JSON object describing that property's schema.
fn make_properties(json_obj: Value) -> HashMap<String, Map<String, Value>> {
    let mut properties = HashMap::new();
    if let Value::Object(obj) = json_obj {
        for (key, value) in obj {
            if let Value::Object(inner) = value {
                properties.insert(key, inner);
            }
        }
    }
    properties
}

/// The main hindsight MCP server handler
///
/// Exposes development history queries as MCP tools for LLM consumption.
/// The database is wrapped in a Mutex to satisfy the `Sync` requirement.
pub struct HindsightServer {
    /// The underlying SQLite database (wrapped for thread safety)
    db: Arc<Mutex<Database>>,
    /// Path to the database file (for creating new connections)
    db_path: Option<PathBuf>,
    /// Default workspace path for queries
    workspace: Option<PathBuf>,
}

impl HindsightServer {
    /// Create a new hindsight server with the given database
    ///
    /// # Arguments
    ///
    /// * `db` - The SQLite database containing development history
    /// * `workspace` - Optional default workspace path
    #[must_use]
    pub fn new(db: Database, workspace: Option<PathBuf>) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            db_path: None,
            workspace,
        }
    }

    /// Create a new hindsight server with a database file path
    ///
    /// This allows the server to create new database connections for
    /// operations that require database ownership (like ingestion).
    #[must_use]
    pub fn with_db_path(mut self, path: PathBuf) -> Self {
        self.db_path = Some(path);
        self
    }

    /// Get access to the database (async, requires await)
    pub async fn db(&self) -> tokio::sync::MutexGuard<'_, Database> {
        self.db.lock().await
    }

    /// Get the default workspace path
    #[must_use]
    pub fn workspace(&self) -> Option<&PathBuf> {
        self.workspace.as_ref()
    }

    /// Open a new database connection for operations requiring ownership
    fn open_db_for_ingest(&self) -> Option<Database> {
        self.db_path
            .as_ref()
            .and_then(|path| Database::open(path).ok())
    }

    /// Build the list of available tools
    fn build_tools() -> Vec<Tool> {
        vec![
            Self::timeline_tool(),
            Self::search_tool(),
            Self::failing_tests_tool(),
            Self::activity_summary_tool(),
            Self::commit_details_tool(),
            Self::ingest_tool(),
        ]
    }

    fn timeline_tool() -> Tool {
        Tool {
            name: "hindsight_timeline".into(),
            description: Some(
                "Get a chronological view of development activity including commits, \
                 test runs, and Copilot sessions."
                    .into(),
            ),
            input_schema: ToolInputSchema::new(
                vec![],
                Some(make_properties(json!({
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "description": "Maximum events to return"
                    },
                    "workspace": {
                        "type": "string",
                        "description": "Filter by workspace path (optional)"
                    }
                }))),
                None,
            ),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            output_schema: None,
            title: Some("Development Timeline".into()),
        }
    }

    fn search_tool() -> Tool {
        Tool {
            name: "hindsight_search".into(),
            description: Some(
                "Full-text search across all sources (commits, messages, tests). \
                 Supports FTS5 query syntax for advanced searches."
                    .into(),
            ),
            input_schema: ToolInputSchema::new(
                vec!["query".into()],
                Some(make_properties(json!({
                    "query": {
                        "type": "string",
                        "description": "Search query (FTS5 syntax supported)"
                    },
                    "source": {
                        "type": "string",
                        "enum": ["all", "commits", "messages"],
                        "default": "all",
                        "description": "Source to search"
                    },
                    "limit": {
                        "type": "integer",
                        "default": 20,
                        "description": "Maximum results to return"
                    }
                }))),
                None,
            ),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            output_schema: None,
            title: Some("Search Development History".into()),
        }
    }

    fn failing_tests_tool() -> Tool {
        Tool {
            name: "hindsight_failing_tests".into(),
            description: Some("Get currently failing tests from the most recent test runs, optionally filtered by commit.".into()),
            input_schema: ToolInputSchema::new(
                vec![],
                Some(make_properties(json!({
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "description": "Maximum failing tests to return"
                    },
                    "workspace": {
                        "type": "string",
                        "description": "Filter by workspace (optional)"
                    },
                    "commit": {
                        "type": "string",
                        "description": "Filter by commit SHA - full or partial (optional)"
                    }
                }))),
                None,
            ),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            output_schema: None,
            title: Some("Failing Tests".into()),
        }
    }

    fn activity_summary_tool() -> Tool {
        Tool {
            name: "hindsight_activity_summary".into(),
            description: Some("Get aggregate activity statistics for a time period.".into()),
            input_schema: ToolInputSchema::new(
                vec![],
                Some(make_properties(json!({
                    "days": {
                        "type": "integer",
                        "default": 7,
                        "description": "Number of days to summarize"
                    }
                }))),
                None,
            ),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            output_schema: None,
            title: Some("Activity Summary".into()),
        }
    }

    fn commit_details_tool() -> Tool {
        Tool {
            name: "hindsight_commit_details".into(),
            description: Some(
                "Get detailed information about a specific commit including linked test runs."
                    .into(),
            ),
            input_schema: ToolInputSchema::new(
                vec!["sha".into()],
                Some(make_properties(json!({
                    "sha": {
                        "type": "string",
                        "description": "Full or partial commit SHA"
                    }
                }))),
                None,
            ),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            output_schema: None,
            title: Some("Commit Details".into()),
        }
    }

    fn ingest_tool() -> Tool {
        Tool {
            name: "hindsight_ingest".into(),
            description: Some(
                "Trigger data ingestion from sources (git, copilot). \
                 Note: Test ingestion requires nextest output and should be done via CLI."
                    .into(),
            ),
            input_schema: ToolInputSchema::new(
                vec!["workspace".into()],
                Some(make_properties(json!({
                    "source": {
                        "type": "string",
                        "enum": ["git", "copilot", "all"],
                        "default": "all",
                        "description": "Source to ingest from (git, copilot, or all)"
                    },
                    "workspace": {
                        "type": "string",
                        "description": "Workspace path to ingest"
                    },
                    "incremental": {
                        "type": "boolean",
                        "default": true,
                        "description": "Only ingest new data since last run"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max items to ingest (optional)"
                    }
                }))),
                None,
            ),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            output_schema: None,
            title: Some("Ingest Data".into()),
        }
    }
}

/// ServerHandler implementation for the MCP protocol
#[async_trait]
impl ServerHandler for HindsightServer {
    /// Handle requests to list available tools
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        let tools = Self::build_tools();
        debug!(tool_count = tools.len(), "Listing available tools");
        trace!(tools = ?tools.iter().map(|t| &t.name).collect::<Vec<_>>(), "Tool names");
        Ok(ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        })
    }

    /// Handle requests to call a specific tool
    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        debug!(tool = %params.name, "Calling tool");
        trace!(tool = %params.name, args = ?params.arguments, "Tool arguments");

        let args = params.arguments;

        let result: Result<Value, HandlerError> = match params.name.as_str() {
            "hindsight_timeline" => {
                let db = self.db.lock().await;
                let workspace = self.workspace.clone();
                handlers::handle_timeline(&db, args, workspace.as_ref())
                    .map(|events| serde_json::to_value(events).unwrap_or_default())
            }
            "hindsight_search" => {
                let db = self.db.lock().await;
                handlers::handle_search(&db, args)
                    .map(|results| serde_json::to_value(results).unwrap_or_default())
            }
            "hindsight_failing_tests" => {
                let db = self.db.lock().await;
                let workspace = self.workspace.clone();
                handlers::handle_failing_tests(&db, args, workspace.as_ref())
                    .map(|tests| serde_json::to_value(tests).unwrap_or_default())
            }
            "hindsight_activity_summary" => {
                let db = self.db.lock().await;
                handlers::handle_activity_summary(&db, args)
                    .map(|summary| serde_json::to_value(summary).unwrap_or_default())
            }
            "hindsight_commit_details" => {
                let db = self.db.lock().await;
                handlers::handle_commit_details(&db, args)
                    .map(|commit| serde_json::to_value(commit).unwrap_or_default())
            }
            "hindsight_ingest" => {
                // Ingest requires ownership of database, so we open a new connection
                // This is safe because SQLite handles concurrent access
                match self.open_db_for_ingest() {
                    Some(db) => handlers::handle_ingest(db, args)
                        .map(|response| serde_json::to_value(response).unwrap_or_default()),
                    None => Err(HandlerError::InvalidInput(
                        "Cannot perform ingestion: database path not configured. \
                         Use --database flag when starting the server."
                            .to_string(),
                    )),
                }
            }
            _ => return Err(CallToolError::unknown_tool(&params.name)),
        };

        match result {
            Ok(value) => {
                let json_str = serde_json::to_string_pretty(&value).unwrap_or_default();
                debug!(tool = %params.name, response_len = json_str.len(), "Tool completed successfully");
                trace!(tool = %params.name, response = %json_str, "Tool response");
                Ok(CallToolResult::text_content(vec![TextContent::new(
                    json_str, None, None,
                )]))
            }
            Err(e) => {
                error!(error = %e, tool = %params.name, "Tool handler error");
                // Return error as content rather than failing the request
                // This allows the LLM to see and potentially handle the error
                Ok(CallToolResult::text_content(vec![TextContent::new(
                    format!("Error: {}", e),
                    None,
                    None,
                )]))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_server() -> HindsightServer {
        let db = Database::in_memory().expect("Failed to create in-memory database");
        HindsightServer::new(db, None)
    }

    #[test]
    fn test_server_creation() {
        let server = create_test_server();
        assert!(server.workspace().is_none());
    }

    #[test]
    fn test_server_with_workspace() {
        let db = Database::in_memory().expect("Failed to create in-memory database");
        let workspace = PathBuf::from("/test/workspace");
        let server = HindsightServer::new(db, Some(workspace.clone()));

        assert_eq!(server.workspace(), Some(&workspace));
    }

    #[test]
    fn test_build_tools() {
        let tools = HindsightServer::build_tools();
        assert_eq!(tools.len(), 6);

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"hindsight_timeline"));
        assert!(tool_names.contains(&"hindsight_search"));
        assert!(tool_names.contains(&"hindsight_failing_tests"));
        assert!(tool_names.contains(&"hindsight_activity_summary"));
        assert!(tool_names.contains(&"hindsight_commit_details"));
        assert!(tool_names.contains(&"hindsight_ingest"));
    }

    #[test]
    fn test_tool_schemas_have_properties() {
        let tools = HindsightServer::build_tools();
        for tool in tools {
            // Verify all tools have properties defined
            assert!(
                tool.input_schema.properties.is_some(),
                "Tool {} should have properties",
                tool.name
            );
        }
    }
}
