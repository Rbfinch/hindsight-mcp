#![no_main]

//! Fuzz target for JSON parsing
//!
//! This target tests that arbitrary bytes never cause panics
//! when parsed as JSON for tool arguments.

use libfuzzer_sys::fuzz_target;
use serde_json::{Map, Value};

use hindsight_mcp::handlers::{
    ActivitySummaryInput, CommitDetailsInput, FailingTestsInput, IngestInput, SearchInput,
    TimelineInput,
};

fuzz_target!(|data: &[u8]| {
    // Try to parse as UTF-8 string first
    if let Ok(s) = std::str::from_utf8(data) {
        // Try to parse as generic JSON
        let _: Result<Value, _> = serde_json::from_str(s);

        // Try to parse as each input type
        let _: Result<TimelineInput, _> = serde_json::from_str(s);
        let _: Result<SearchInput, _> = serde_json::from_str(s);
        let _: Result<FailingTestsInput, _> = serde_json::from_str(s);
        let _: Result<ActivitySummaryInput, _> = serde_json::from_str(s);
        let _: Result<CommitDetailsInput, _> = serde_json::from_str(s);
        let _: Result<IngestInput, _> = serde_json::from_str(s);

        // Try to parse as Map<String, Value>
        let _: Result<Map<String, Value>, _> = serde_json::from_str(s);
    }

    // Also try parsing raw bytes (should fail gracefully)
    let _: Result<Value, _> = serde_json::from_slice(data);
});
