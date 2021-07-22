//! Library fixtures

/// GraphQL schema and data from Star Wars.
pub mod starwars;

#[cfg(feature = "trace")]
/// Fixtures used to test integration with `tracing` crate.
pub mod tracing;
