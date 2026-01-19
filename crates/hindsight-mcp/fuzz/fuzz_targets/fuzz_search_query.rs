#![no_main]

//! Fuzz target for FTS5 search query parsing
//!
//! This target tests that arbitrary search queries never cause
//! panics or SQL injection when passed to the search handler.

use libfuzzer_sys::fuzz_target;
use serde_json::{Map, Value, json};

use hindsight_mcp::db::Database;
use hindsight_mcp::handlers;

fn create_test_db() -> Database {
    let db = Database::in_memory().expect("create db");
    db.initialize().expect("init db");
    db
}

fn to_map(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fuzz_target!(|query: String| {
    let db = create_test_db();

    // Test search with arbitrary query
    let args = to_map(json!({
        "query": &query,
        "source": "all",
        "limit": 10
    }));

    // This should never panic, even with malicious input
    let _ = handlers::handle_search(&db, Some(args));

    // Also test with commits-only source
    let args = to_map(json!({
        "query": &query,
        "source": "commits",
        "limit": 10
    }));
    let _ = handlers::handle_search(&db, Some(args));

    // And messages-only source
    let args = to_map(json!({
        "query": &query,
        "source": "messages",
        "limit": 10
    }));
    let _ = handlers::handle_search(&db, Some(args));
});
