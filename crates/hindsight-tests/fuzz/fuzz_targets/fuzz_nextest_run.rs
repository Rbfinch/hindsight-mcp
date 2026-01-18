// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Fuzz target for nextest run output parsing
//!
//! This fuzzes `parse_run_output` which parses line-delimited JSON from
//! `cargo nextest run --message-format libtest-json`.

#![no_main]

use libfuzzer_sys::fuzz_target;

use hindsight_tests::parse_run_output;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string - invalid UTF-8 should be handled gracefully
    if let Ok(input) = std::str::from_utf8(data) {
        // parse_run_output should never panic on any input
        let _ = parse_run_output(input);
    }
});
