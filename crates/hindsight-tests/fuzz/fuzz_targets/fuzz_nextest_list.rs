// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Fuzz target for nextest list output parsing
//!
//! This fuzzes `parse_list_output` which parses JSON from
//! `cargo nextest list --message-format json`.

#![no_main]

use libfuzzer_sys::fuzz_target;

use hindsight_tests::parse_list_output;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string - invalid UTF-8 should be handled gracefully
    if let Ok(input) = std::str::from_utf8(data) {
        // parse_list_output should never panic on any input
        let _ = parse_list_output(input);
    }
});
