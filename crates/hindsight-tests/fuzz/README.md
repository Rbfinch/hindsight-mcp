# Fuzz testing for hindsight-tests

This directory contains fuzz tests for the hindsight-tests crate.

## Setup

Install cargo-fuzz:
```bash
cargo install cargo-fuzz
```

## Running fuzz tests

```bash
cargo +nightly fuzz run fuzz_target_1
```
