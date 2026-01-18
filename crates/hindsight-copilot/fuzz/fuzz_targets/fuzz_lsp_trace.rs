// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

//! Fuzz target for LSP message deserialization
//!
//! This fuzzes the `LspMessage` type which deserializes Copilot's LSP
//! messages in JSON-RPC format.

#![no_main]

use libfuzzer_sys::fuzz_target;

use hindsight_copilot::lsp::LspMessage;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string - invalid UTF-8 should be handled gracefully
    if let Ok(input) = std::str::from_utf8(data) {
        // Deserializing should never panic on any input
        let _: Result<LspMessage, _> = serde_json::from_str(input);
    }
});
