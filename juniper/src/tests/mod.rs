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
#[cfg(test)]
mod type_info_tests;

#[cfg(test)]
#[cfg(feature = "trace")]
mod trace_tests;

#[cfg(test)]
#[cfg(feature = "trace-sync")]
mod trace_sync_tests;

#[cfg(test)]
#[cfg(feature = "trace-async")]
mod trace_async_tests;
