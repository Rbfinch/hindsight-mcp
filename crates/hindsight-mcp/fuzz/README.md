# Fuzz testing for hindsight-mcp

This directory contains fuzz tests for the hindsight-mcp crate.

## Setup

Install cargo-fuzz:
```bash
cargo install cargo-fuzz
```

## Running fuzz tests

```bash
cargo +nightly fuzz run fuzz_target_1
```
