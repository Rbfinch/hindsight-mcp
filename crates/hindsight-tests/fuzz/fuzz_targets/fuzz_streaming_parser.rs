// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Fuzz target for streaming parser
//!
//! This fuzzes the `StreamingParser` which processes nextest output
//! line-by-line incrementally.

#![no_main]

use libfuzzer_sys::fuzz_target;

use hindsight_tests::StreamingParser;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let mut parser = StreamingParser::new();

        // Process each line - parser should never panic
        for line in input.lines() {
            let _ = parser.process_line(line);
        }

        // Finalize should never panic
        let _ = parser.finish();
    }
});
