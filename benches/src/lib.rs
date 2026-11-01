//! Benchmarks of [`juniper`] against [`async_graphql`] and [`apollo_compiler`].
//!
//! Every [`Engine`] serves the same schema and the same data. The `comparison` benchmark runs
//! every [`Operation`] through every engine. One run starts with a JSON request body and ends with
//! a JSON response body. A GraphQL server does the same work for each request, without HTTP.
//!
//! Run the benchmark with `cargo bench -p juniper_benchmarks`.
//!
//! [`Engine`]: engine::Engine
//! [`Operation`]: operation::Operation

#![cfg_attr(test, allow(unused_crate_dependencies, reason = "benches"))]

pub mod data;
pub mod engine;
pub mod operation;
