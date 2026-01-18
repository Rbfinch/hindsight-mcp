// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! hindsight-mcp library
//!
//! This module exports the core functionality of hindsight-mcp for use in
//! integration tests and as a library.

mod migrations;

pub mod config;
pub mod db;
pub mod handlers;
pub mod ingest;
pub mod queries;
pub mod server;
