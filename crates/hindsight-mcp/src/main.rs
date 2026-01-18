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
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

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
        Some(Command::Test {
            package,
            bin,
            filter,
            stdin,
            dry_run,
            no_commit,
            commit,
            show_output,
            nextest_args,
        }) => {
            run_test(
                &config,
                package.clone(),
                bin.clone(),
                filter.clone(),
                *stdin,
                *dry_run,
                *no_commit,
                commit.clone(),
                *show_output,
                nextest_args.clone(),
            )
            .await
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

/// Check if cargo-nextest is installed
///
/// Returns Ok(()) if nextest is available, or an error with install instructions.
fn check_nextest_installed() -> anyhow::Result<()> {
    let output = ProcessCommand::new("cargo")
        .args(["nextest", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match output {
        Ok(status) if status.success() => Ok(()),
        _ => Err(anyhow::anyhow!(
            "cargo-nextest is not installed.\n\n\
                 Install it with:\n  \
                 cargo install cargo-nextest\n\n\
                 Or see: https://nexte.st/book/installation.html"
        )),
    }
}

/// Result of running nextest
#[derive(Debug)]
struct NextestResult {
    /// JSON output from nextest (stdout)
    json_output: String,
    /// Whether all tests passed
    success: bool,
}

/// Run cargo nextest and capture JSON output
///
/// # Arguments
/// * `workspace` - Path to the workspace/project directory
/// * `package` - Package(s) to test
/// * `bin` - Binary(ies) to test
/// * `filter` - Filter expression for tests
/// * `show_output` - Whether to stream stderr to terminal
/// * `nextest_args` - Additional arguments to pass to nextest
///
/// # Returns
/// The captured JSON output from nextest
fn run_nextest(
    workspace: &Path,
    package: &[String],
    bin: &[String],
    filter: Option<&str>,
    show_output: bool,
    nextest_args: &[String],
) -> anyhow::Result<NextestResult> {
    let mut cmd = ProcessCommand::new("cargo");

    // Base command
    cmd.arg("nextest")
        .arg("run")
        .arg("--message-format")
        .arg("libtest-json");

    // Set required environment variable for JSON output
    cmd.env("NEXTEST_EXPERIMENTAL_LIBTEST_JSON", "1");

    // Add package filters
    for pkg in package {
        cmd.arg("--package").arg(pkg);
    }

    // Add binary filters
    for b in bin {
        cmd.arg("--bin").arg(b);
    }

    // Add filter expression
    if let Some(f) = filter {
        cmd.arg("-E").arg(f);
    }

    // Add passthrough args
    for arg in nextest_args {
        cmd.arg(arg);
    }

    // Set working directory
    cmd.current_dir(workspace);

    // Configure output handling
    cmd.stdout(Stdio::piped());

    if show_output {
        // Stream stderr to terminal
        cmd.stderr(Stdio::inherit());
    } else {
        // Suppress stderr
        cmd.stderr(Stdio::null());
    }

    debug!(command = ?cmd, "Spawning nextest");

    let output = cmd.output().map_err(|e| {
        anyhow::anyhow!(
            "Failed to spawn cargo nextest: {}\n\n\
             Make sure cargo-nextest is installed:\n  \
             cargo install cargo-nextest",
            e
        )
    })?;

    let json_output = String::from_utf8_lossy(&output.stdout).to_string();

    if json_output.trim().is_empty() {
        warn!("No JSON output received from nextest - tests may have failed to run");
    }

    Ok(NextestResult {
        json_output,
        success: output.status.success(),
    })
}

/// Get the current git HEAD commit SHA
///
/// # Arguments
/// * `workspace` - Path to the workspace/project directory
///
/// # Returns
/// * `Some(sha)` - The full 40-character SHA of HEAD
/// * `None` - If not in a git repository or HEAD is unborn
fn get_current_commit(workspace: &Path) -> Option<String> {
    // Try to open the repository
    let repo = match git2::Repository::discover(workspace) {
        Ok(repo) => repo,
        Err(e) => {
            debug!(error = %e, "Not a git repository or git error");
            return None;
        }
    };

    // Get HEAD reference
    let head = match repo.head() {
        Ok(head) => head,
        Err(e) => {
            // Unborn HEAD (new repo with no commits) is not an error for us
            debug!(error = %e, "Could not resolve HEAD");
            return None;
        }
    };

    // Resolve to commit SHA
    match head.peel_to_commit() {
        Ok(commit) => {
            let sha = commit.id().to_string();
            debug!(commit = %sha, "Detected git HEAD commit");
            Some(sha)
        }
        Err(e) => {
            debug!(error = %e, "Could not peel HEAD to commit");
            None
        }
    }
}

/// Run tests and ingest results
///
/// This command wraps cargo-nextest, runs tests, and ingests results.
#[allow(clippy::too_many_arguments)]
async fn run_test(
    config: &Config,
    package: Vec<String>,
    bin: Vec<String>,
    filter: Option<String>,
    stdin: bool,
    dry_run: bool,
    no_commit: bool,
    commit: Option<String>,
    show_output: bool,
    nextest_args: Vec<String>,
) -> anyhow::Result<()> {
    // Initialize logging for CLI mode
    let filter_directive = EnvFilter::from_default_env().add_directive(config.log_level().into());

    tracing_subscriber::fmt()
        .with_env_filter(filter_directive)
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .init();

    info!("hindsight-mcp test subcommand");

    // Log the parsed options for debugging
    debug!(
        packages = ?package,
        binaries = ?bin,
        filter = ?filter,
        stdin = stdin,
        dry_run = dry_run,
        no_commit = no_commit,
        commit = ?commit,
        show_output = show_output,
        nextest_args = ?nextest_args,
        "Test command options"
    );

    // Get workspace path
    let workspace = config.workspace_path().ok_or_else(|| {
        anyhow::anyhow!("Workspace path is required. Use --workspace or set HINDSIGHT_WORKSPACE")
    })?;

    // Get JSON output - either from stdin or by running nextest
    let json_output = if stdin {
        // Read from stdin (CI mode)
        info!("Reading test results from stdin");
        let stdin_handle = io::stdin();
        let mut input = String::new();
        for line in stdin_handle.lock().lines() {
            let line = line?;
            input.push_str(&line);
            input.push('\n');
        }

        if input.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "No input received from stdin.\n\n\
                 Pipe nextest JSON output:\n  \
                 NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run \
                 --message-format libtest-json | hindsight-mcp test --stdin"
            ));
        }

        input
    } else {
        // Check nextest is installed
        check_nextest_installed()?;

        // Run nextest
        info!(workspace = %workspace.display(), "Running tests");
        let result = run_nextest(
            &workspace,
            &package,
            &bin,
            filter.as_deref(),
            show_output,
            &nextest_args,
        )?;

        if !result.success {
            warn!("Some tests failed");
        }

        result.json_output
    };

    // Determine commit SHA
    let commit_sha = if no_commit {
        debug!("Commit linking disabled (--no-commit)");
        None
    } else if let Some(ref sha) = commit {
        debug!(commit = %sha, "Using explicit commit SHA");
        Some(sha.clone())
    } else {
        // Auto-detect from git HEAD
        match get_current_commit(&workspace) {
            Some(sha) => {
                info!(commit = %sha, "Auto-detected git commit");
                Some(sha)
            }
            None => {
                debug!("Not in a git repository or no commits - proceeding without commit link");
                None
            }
        }
    };

    // Parse and display results
    let summary = hindsight_tests::parse_run_output(&json_output)?;

    if dry_run {
        // Dry-run mode: display what would be ingested
        println!("Dry-run mode - no data will be written to database\n");
        println!("Test Summary:");
        println!("  Total:   {}", summary.results.len());
        println!("  Passed:  {}", summary.passed);
        println!("  Failed:  {}", summary.failed);
        println!("  Ignored: {}", summary.ignored);
        if let Some(ref sha) = commit_sha {
            println!("  Commit:  {}", sha);
        } else {
            println!("  Commit:  (none)");
        }
        println!("\nWorkspace: {}", workspace.display());
        return Ok(());
    }

    // TODO: Phase 3 - Ingest to database
    // For now, just print summary
    println!("Test run completed:");
    println!("  Total:   {}", summary.results.len());
    println!("  Passed:  {}", summary.passed);
    println!("  Failed:  {}", summary.failed);
    println!("  Ignored: {}", summary.ignored);
    println!();
    eprintln!("Note: Database ingestion not yet implemented (Phase 3)");
    eprintln!("      Results were captured but not persisted.");

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

    #[test]
    fn test_get_current_commit_in_git_repo() {
        // This test runs from within the hindsight-mcp repo
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        // Should detect a commit SHA since we're in a git repo
        let commit = get_current_commit(&repo_path);
        assert!(commit.is_some(), "Expected to detect git commit in repo");

        // SHA should be 40 hex characters
        let sha = commit.unwrap();
        assert_eq!(sha.len(), 40, "SHA should be 40 characters");
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "SHA should be hex"
        );
    }

    #[test]
    fn test_get_current_commit_non_git_directory() {
        // /tmp is unlikely to be a git repository
        let non_git_path = PathBuf::from("/tmp");

        let commit = get_current_commit(&non_git_path);
        assert!(
            commit.is_none(),
            "Expected None for non-git directory, got {:?}",
            commit
        );
    }

    #[test]
    fn test_get_current_commit_nonexistent_directory() {
        let nonexistent = PathBuf::from("/nonexistent/path/12345");

        let commit = get_current_commit(&nonexistent);
        assert!(commit.is_none(), "Expected None for nonexistent path");
    }
}
