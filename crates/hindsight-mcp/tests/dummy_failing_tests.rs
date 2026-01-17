//! Dummy failing tests for testing hindsight_failing_tests functionality
//!
//! These tests are intentionally designed to fail to populate the failing_tests view.
//! Run with: cargo nextest run --package hindsight-mcp --test dummy_failing_tests
//!
//! After running, delete this file or exclude it from normal test runs.

#[test]
fn test_that_passes() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_assertion_failure() {
    // This test intentionally fails with an assertion error
    assert_eq!(1 + 1, 3, "Math is broken!");
}

#[test]
fn test_panic_failure() {
    // This test intentionally panics
    panic!("This test panics on purpose for testing hindsight_failing_tests");
}

#[test]
fn test_expected_vs_actual_mismatch() {
    // This test shows a clear expected vs actual mismatch
    let expected = "hello world";
    let actual = "goodbye world";
    assert_eq!(actual, expected);
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn test_that_also_passes() {
    // Intentionally trivial assertion for dummy test
    assert!(true);
}

#[test]
#[allow(clippy::unnecessary_literal_unwrap)]
fn test_option_unwrap_failure() {
    // This test intentionally fails on unwrap
    let value: Option<i32> = None;
    let _ = value.expect("Expected a value but got None!");
}
