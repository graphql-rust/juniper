//! Library fixtures

/// GraphQL schema and data from Star Wars.
pub mod starwars;

#[cfg(all(test, feature = "tracing"))]
pub mod tracing;
