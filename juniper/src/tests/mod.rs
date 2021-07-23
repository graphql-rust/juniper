//! Library tests and fixtures

pub mod fixtures;
#[cfg(test)]
mod introspection_tests;
#[cfg(test)]
mod query_tests;
#[cfg(test)]
mod schema_introspection;
#[cfg(test)]
mod subscriptions;
#[cfg(all(test, feature = "tracing"))]
mod tracing_tests;
#[cfg(test)]
mod type_info_tests;
