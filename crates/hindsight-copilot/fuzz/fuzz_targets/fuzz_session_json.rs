// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Fuzz target for Copilot chat session JSON parsing
//!
//! This fuzzes `parse_session_json` which parses VS Code's chat session
//! JSON format from `~/.config/Code/User/workspaceStorage/<id>/chatSessions/*.json`.

#![no_main]

use libfuzzer_sys::fuzz_target;

use hindsight_copilot::parse_session_json;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string - invalid UTF-8 should be handled gracefully
    if let Ok(input) = std::str::from_utf8(data) {
        // parse_session_json should never panic on any input
        // Use a dummy workspace ID for fuzzing
        let _ = parse_session_json(input, "fuzz-workspace-id");
    }
});
