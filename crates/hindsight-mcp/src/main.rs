//! hindsight-mcp: MCP server for AI-assisted coding with development history
//!
//! This binary crate provides an MCP server that consolidates development data
//! (git logs, test results, GitHub Copilot logs) into a searchable SQLite database.

use tracing::info;

mod db;
mod server;

fn main() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Starting hindsight-mcp server...");

    // TODO: Initialize SQLite database
    // TODO: Start MCP server
    // TODO: Register tools for querying development history
}
