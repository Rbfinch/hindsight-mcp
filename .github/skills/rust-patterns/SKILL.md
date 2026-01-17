---
name: rust-patterns
description: Rust coding patterns, conventions, and best practices for the hindsight-mcp workspace. Use when writing or reviewing Rust code, implementing new features, creating error types, adding tests, or when guidance on Rust 2024 idioms is needed. Triggers on Rust implementation tasks, code reviews, and questions about project conventions.
---

# Rust Patterns for hindsight-mcp

Rust 2024 edition patterns and conventions for this workspace.

## Error Handling

Use `thiserror` for error types. Each crate has `src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("Description: {field_name}")]
    VariantName { field_name: String },
    
    #[error("Wrapped error: {0}")]
    Wrapped(#[from] OtherError),
}
```

## Module Organization

```
src/
├── lib.rs          # Module declarations, re-exports
├── error.rs        # Crate-specific error enum
├── prelude.rs      # Common re-exports (optional)
└── feature.rs      # Feature modules
```

## Documentation

```rust
//! Module-level docs at file top

/// Public item docs required
/// 
/// # Examples
/// 
/// ```rust
/// let result = my_function();
/// ```
pub fn my_function() -> Result<(), Error> { ... }
```

## Testing Patterns

```rust
// Unit tests in same file
#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;
    
    #[test]
    fn test_name() {
        // Arrange
        let input = ...;
        
        // Act
        let result = function(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

Integration tests: `tests/integration_tests.rs`
Benchmarks: `benches/<crate>_bench.rs` (Criterion)
Fuzz tests: `fuzz/` directory (cargo-fuzz)
Property tests: Use `proptest` crate

## Logging

Use `tracing` macros:

```rust
use tracing::{info, debug, warn, error, trace};

info!("High-level operation");
debug!(field = value, "Diagnostic info");
error!(?err, "Unrecoverable error");
```

## Serialization

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MyType {
    pub field: String,
}
```

Prefer strongly-typed over `serde_json::Value`.

## Async Patterns

```rust
// MCP server uses rust-mcp-sdk
async fn handler() -> Result<Response, Error> {
    let data = fetch_data().await?;
    Ok(process(data))
}
```

Avoid mixing blocking and async without `spawn_blocking`.

## Best Practices Checklist

- [ ] Use `?` for error propagation
- [ ] No `unwrap()` in library code
- [ ] `impl Trait` in return position
- [ ] `&str` over `String` in parameters
- [ ] `#[must_use]` on important return values
- [ ] Small, focused functions
- [ ] Iterators over explicit loops
