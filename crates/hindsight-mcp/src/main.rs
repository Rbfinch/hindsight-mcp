// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

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

use std::io::{self, BufRead};

use clap::Parser;
use rust_mcp_sdk::mcp_server::{McpServerOptions, ToMcpServerHandler, server_runtime};
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools,
};
use rust_mcp_sdk::{McpServer, StdioTransport, TransportOptions};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

use hindsight_mcp::config::{Command, Config};
use hindsight_mcp::db::Database;
use hindsight_mcp::ingest::Ingestor;
use hindsight_mcp::server::HindsightServer;

/// Initialize the tracing/logging subsystem
///
/// Logs are written to stderr to avoid interfering with MCP stdio transport.
fn init_logging(config: &Config) {
    let level = config.log_level();

    let filter = EnvFilter::from_default_env().add_directive(level.into());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_target(config.verbose)
        .with_thread_ids(config.verbose)
        .init();
}

/// Initialize the database, running migrations if needed
fn init_database(config: &Config) -> anyhow::Result<Database> {
    let db_path = config.database_path();

    debug!(path = %db_path.display(), "Opening database");

    // Open or create database
    let db = Database::open(&db_path).map_err(|e| {
        error!(error = %e, path = %db_path.display(), "Failed to open database");
        anyhow::anyhow!("Failed to open database: {}", e)
    })?;

    // Check if initialization is needed
    if config.skip_init {
        debug!("Skipping database initialization (--skip-init)");
        if !db.is_initialized() {
            warn!("Database may not be initialized - queries may fail");
        }
    } else if !db.is_initialized() {
        info!("Initializing database schema...");
        db.initialize().map_err(|e| {
            error!(error = %e, "Failed to initialize database schema");
            anyhow::anyhow!("Failed to initialize database: {}", e)
        })?;
        info!("Database schema initialized successfully");
    } else {
        let version = db.schema_version().unwrap_or(0);
        debug!(version = version, "Database schema up to date");
    }

    Ok(db)
}

/// Build the MCP server details and capabilities
fn build_server_details() -> InitializeResult {
    InitializeResult {
        server_info: Implementation {
            name: "hindsight-mcp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Hindsight MCP Server".into()),
            description: Some(
                "MCP server providing access to development history including \
                 git commits, test results, and GitHub Copilot sessions. \
                 Use the available tools to search, explore, and analyze \
                 your development activity."
                    .into(),
            ),
            icons: vec![],
            website_url: Some("https://github.com/Rbfinch/hindsight-mcp".into()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        instructions: Some(
            "Hindsight MCP provides tools to explore your development history:\n\n\
             - hindsight_timeline: View chronological development activity\n\
             - hindsight_search: Full-text search across commits and messages\n\
             - hindsight_failing_tests: Get currently failing tests\n\
             - hindsight_activity_summary: Aggregate activity statistics\n\
             - hindsight_commit_details: Detailed commit information\n\
             - hindsight_ingest: Trigger data ingestion from sources\n\n\
             All tools support optional workspace filtering."
                .into(),
        ),
        meta: None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse configuration from CLI arguments and environment
    let config = Config::parse();

    // Handle subcommands
    match &config.command {
        Some(Command::Ingest { tests, commit }) => {
            run_ingest(&config, *tests, commit.clone()).await
        }
        None => {
            // Default: run MCP server
            run_server(config).await
        }
    }
}

/// Run the test ingestion command
async fn run_ingest(config: &Config, tests: bool, commit: Option<String>) -> anyhow::Result<()> {
    if !tests {
        eprintln!("Error: No ingestion source specified. Use --tests to ingest test results.");
        std::process::exit(1);
    }

    // Initialize simple logging for CLI mode
    let filter = EnvFilter::from_default_env().add_directive(config.log_level().into());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .init();

    // Get workspace path
    let workspace = config.workspace_path().ok_or_else(|| {
        anyhow::anyhow!("Workspace path is required. Use --workspace or set HINDSIGHT_WORKSPACE")
    })?;

    info!(workspace = %workspace.display(), "Starting test ingestion");

    // Read stdin
    let stdin = io::stdin();
    let mut input = String::new();
    for line in stdin.lock().lines() {
        let line = line?;
        input.push_str(&line);
        input.push('\n');
    }

    if input.trim().is_empty() {
        eprintln!("Error: No input received from stdin. Pipe nextest JSON output.");
        eprintln!(
            "Example: NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --message-format libtest-json | hindsight-mcp ingest --tests"
        );
        std::process::exit(1);
    }

    // Initialize database
    let db = init_database(config)?;

    // Run ingestion
    let mut ingestor = Ingestor::new(db);
    let stats = ingestor.ingest_tests(&workspace, &input, commit.as_deref())?;

    info!(
        tests_inserted = stats.test_results_inserted,
        runs_inserted = stats.test_runs_inserted,
        "Test ingestion complete"
    );

    println!(
        "Ingested {} test results in {} test run(s)",
        stats.test_results_inserted, stats.test_runs_inserted
    );

    Ok(())
}

/// Run the MCP server
async fn run_server(config: Config) -> anyhow::Result<()> {
    // Initialize logging - must write to stderr to not interfere with MCP stdio
    init_logging(&config);

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting hindsight-mcp server"
    );

    // Validate configuration
    if let Err(e) = config.validate() {
        error!(error = %e, "Configuration validation failed");
        return Err(anyhow::anyhow!("Configuration error: {}", e));
    }

    let db_path = config.database_path();
    let workspace = config.workspace_path();

    debug!(
        database = %db_path.display(),
        workspace = ?workspace.as_ref().map(|p| p.display().to_string()),
        verbose = config.verbose,
        "Configuration loaded"
    );

    // Initialize database with migrations
    let db = init_database(&config)?;

    info!(
        database = %db_path.display(),
        "Database ready"
    );

    // Create handler instance with db path for ingestion support
    let handler = HindsightServer::new(db, workspace).with_db_path(db_path);

    // Build server details and capabilities
    let server_details = build_server_details();

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

    info!("MCP server ready, waiting for requests...");

    // Start the server and wait for it to complete
    if let Err(e) = server.start().await {
        error!(error = %e, "MCP server error");
        return Err(anyhow::anyhow!(
            "MCP server error: {}",
            e.rpc_error_message().unwrap_or(&e.to_string())
        ));
    }

    info!("MCP server shutting down gracefully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_config_default_database_path() {
        let config = Config {
            command: None,
            database: None,
            workspace: None,
            verbose: false,
            quiet: false,
            skip_init: false,
        };

        let path = config.database_path();
        assert!(path.to_string_lossy().contains("hindsight"));
    }

    #[test]
    fn test_config_custom_database_path() {
        let custom_path = PathBuf::from("/custom/path/db.sqlite");
        let config = Config {
            command: None,
            database: Some(custom_path.clone()),
            workspace: None,
            verbose: false,
            quiet: false,
            skip_init: false,
        };

        assert_eq!(config.database_path(), custom_path);
    }

    #[test]
    fn test_config_workspace_path_fallback() {
        let config = Config {
            command: None,
            database: None,
            workspace: None,
            verbose: false,
            quiet: false,
            skip_init: false,
        };

        // Should fallback to current directory
        let workspace = config.workspace_path();
        assert!(workspace.is_some());
    }

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Config::command().debug_assert();
    }

    #[test]
    fn test_build_server_details() {
        let details = build_server_details();
        assert_eq!(details.server_info.name, "hindsight-mcp");
        assert!(details.capabilities.tools.is_some());
        assert!(details.instructions.is_some());
    }
}
