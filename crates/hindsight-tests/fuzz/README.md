# Fuzzing hindsight-tests

This directory contains fuzz targets for the hindsight-tests crate.

## Prerequisites

```bash
# Install cargo-fuzz (requires nightly)
cargo install cargo-fuzz
```

## Available Targets

| Target | Description |
|--------|-------------|
| `fuzz_nextest_run` | Fuzzes `parse_run_output()` - libtest JSON parsing |
| `fuzz_nextest_list` | Fuzzes `parse_list_output()` - test list JSON parsing |
| `fuzz_streaming_parser` | Fuzzes `StreamingParser` - incremental line processing |

## Running

```bash
cd crates/hindsight-tests

# Run a specific target
cargo +nightly fuzz run fuzz_nextest_run

# Run with a timeout (seconds per input)
cargo +nightly fuzz run fuzz_nextest_run -- -timeout=5

# Run for a limited time (seconds)
cargo +nightly fuzz run fuzz_nextest_run -- -max_total_time=60

# List all targets
cargo +nightly fuzz list
```

## Corpus

Seed corpus files can be added to `fuzz/corpus/<target>/` to improve coverage.

Example seed for `fuzz_nextest_run`:
```json
{"type":"suite","event":"started","test_count":1}
{"type":"test","event":"started","name":"test_example"}
{"type":"test","event":"ok","name":"test_example","exec_time":0.001}
{"type":"suite","event":"ok","passed":1,"failed":0}
```
