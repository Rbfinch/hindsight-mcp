# Fuzzing hindsight-copilot

This directory contains fuzz targets for the hindsight-copilot crate.

## Prerequisites

```bash
# Install cargo-fuzz (requires nightly)
cargo install cargo-fuzz
```

## Available Targets

| Target | Description |
|--------|-------------|
| `fuzz_session_json` | Fuzzes `parse_session_json()` - VS Code chat session parsing |
| `fuzz_lsp_trace` | Fuzzes `LspMessage` deserialization - LSP JSON-RPC messages |

## Running

```bash
cd crates/hindsight-copilot

# Run a specific target
cargo +nightly fuzz run fuzz_session_json

# Run with a timeout (seconds per input)
cargo +nightly fuzz run fuzz_session_json -- -timeout=5

# Run for a limited time (seconds)
cargo +nightly fuzz run fuzz_session_json -- -max_total_time=60

# List all targets
cargo +nightly fuzz list
```

## Corpus

Seed corpus files can be added to `fuzz/corpus/<target>/` to improve coverage.

Example seed for `fuzz_session_json`:
```json
{
  "version": 1,
  "sessionId": "test-session",
  "requests": [
    {
      "requestId": "req-1",
      "message": {"text": "Hello"},
      "response": [{"value": "Hi there!"}]
    }
  ]
}
```
