//! Library fixtures

/// GraphQL schema and data from Star Wars.
pub mod starwars;

#[cfg(any(feature = "trace", feature = "trace-sync", feature = "trace-async"))]
/// Fixtures used to test integration with `tracing` crate.
pub mod tracing;
