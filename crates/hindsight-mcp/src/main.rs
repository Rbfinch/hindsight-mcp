//! hindsight-mcp: MCP server for AI-assisted coding with development history
//!
//! This binary crate provides an MCP server that consolidates development data
//! (git logs, test results, GitHub Copilot logs) into a searchable SQLite database.
//!
//! # Usage
//!
//! ```bash
//! hindsight-mcp --database ~/.hindsight/dev.db --workspace /path/to/project
//! ```
//!
//! The server communicates over stdio using the MCP (Model Context Protocol).

use std::path::PathBuf;

use clap::Parser;
use rust_mcp_sdk::mcp_server::{McpServerOptions, ToMcpServerHandler, server_runtime};
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools,
};
use rust_mcp_sdk::{McpServer, StdioTransport, TransportOptions};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use hindsight_mcp::db::Database;
use hindsight_mcp::server::HindsightServer;

/// Hindsight MCP Server - AI-assisted coding with development history
#[derive(Parser, Debug)]
#[command(name = "hindsight-mcp")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to SQLite database file
    ///
    /// If the file doesn't exist, it will be created and initialized.
    #[arg(short, long, env = "HINDSIGHT_DATABASE")]
    database: Option<PathBuf>,

    /// Default workspace path for queries
    ///
    /// This is used as the default when tools don't specify a workspace.
    /// Defaults to the current working directory.
    #[arg(short, long, env = "HINDSIGHT_WORKSPACE")]
    workspace: Option<PathBuf>,

    /// Enable verbose logging (debug level)
    #[arg(short, long, default_value = "false")]
    verbose: bool,
}

impl Args {
    /// Get the database path, using a default if not specified
    fn database_path(&self) -> PathBuf {
        self.database.clone().unwrap_or_else(|| {
            // Default to ~/.hindsight/hindsight.db
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("hindsight")
                .join("hindsight.db")
        })
    }

    /// Get the workspace path, using current directory as default
    fn workspace_path(&self) -> Option<PathBuf> {
        self.workspace
            .clone()
            .or_else(|| std::env::current_dir().ok())
    }
}

fn init_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into())
    } else {
        EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into())
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging - must write to stderr to not interfere with MCP stdio
    init_logging(args.verbose);

    let db_path = args.database_path();
    let workspace = args.workspace_path();

    info!(
        database = %db_path.display(),
        workspace = ?workspace.as_ref().map(|p| p.display().to_string()),
        "Starting hindsight-mcp server"
    );

    // Ensure database directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open or create database
    let db = Database::open(&db_path).map_err(|e| {
        error!(error = %e, path = %db_path.display(), "Failed to open database");
        e
    })?;

    info!("Database initialized successfully");

    // Create handler instance with db path for ingestion support
    let handler = HindsightServer::new(db, workspace).with_db_path(db_path);

    // Define server details and capabilities
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "hindsight-mcp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Hindsight MCP Server".into()),
            description: Some(
                "MCP server providing access to development history including \
                 git commits, test results, and GitHub Copilot sessions."
                    .into(),
            ),
            icons: vec![],
            website_url: Some("https://github.com/nicrosby/hindsight-mcp".into()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        instructions: Some(
            "Use the available tools to search, explore, and analyze your development activity. \
             Available tools: hindsight_timeline, hindsight_search, hindsight_failing_tests, \
             hindsight_activity_summary, hindsight_commit_details, hindsight_ingest."
                .into(),
        ),
        meta: None,
    };

    // Create stdio transport
    let transport = StdioTransport::new(TransportOptions::default())
        .map_err(|e| anyhow::anyhow!("Failed to create transport: {}", e))?;

    // Create and start the MCP server
    let server = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
    });

    info!("MCP server started, waiting for requests...");

    // Start the server and wait for it to complete
    if let Err(e) = server.start().await {
        error!(error = %e, "MCP server error");
        return Err(anyhow::anyhow!(
            "MCP server error: {}",
            e.rpc_error_message().unwrap_or(&e.to_string())
        ));
    }

    info!("MCP server shutting down");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_default_database_path() {
        let args = Args {
            database: None,
            workspace: None,
            verbose: false,
        };

        let path = args.database_path();
        assert!(path.to_string_lossy().contains("hindsight"));
    }

    #[test]
    fn test_args_custom_database_path() {
        let custom_path = PathBuf::from("/custom/path/db.sqlite");
        let args = Args {
            database: Some(custom_path.clone()),
            workspace: None,
            verbose: false,
        };

        assert_eq!(args.database_path(), custom_path);
    }

    #[test]
    fn test_args_workspace_path_fallback() {
        let args = Args {
            database: None,
            workspace: None,
            verbose: false,
        };

        // Should fallback to current directory
        let workspace = args.workspace_path();
        assert!(workspace.is_some());
    }

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }
}
