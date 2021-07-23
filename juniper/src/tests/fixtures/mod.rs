//! Library fixtures

/// GraphQL schema and data from Star Wars.
pub mod starwars;

/// Fixtures used to test integration with `tracing` crate.
#[cfg(feature = "tracing")]
pub mod tracing;
