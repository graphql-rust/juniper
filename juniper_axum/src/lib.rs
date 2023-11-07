#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(warnings)]

pub mod extract;
pub mod response;
#[cfg(feature = "subscriptions")]
pub mod subscriptions;

use std::future;

use axum::response::Html;

/// Creates a handler that replies with an HTML page containing [GraphiQL].
///
/// This does not handle routing, so you can mount it on any endpoint.
///
/// # Example
///
/// ```rust
/// use axum::{
///     routing::get,
///     Router
/// };
/// use axum::body::Body;
/// use juniper_axum::graphiql;
///
/// let app: Router<Body> = Router::new().route("/", get(graphiql("/graphql", "/subscriptions")));
/// ```
///
/// [GraphiQL]: https://github.com/graphql/graphiql
pub fn graphiql<'a>(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: impl Into<Option<&'a str>>,
) -> impl FnOnce() -> future::Ready<Html<String>> + Clone + Send {
    let html = Html(juniper::http::graphiql::graphiql_source(
        graphql_endpoint_url,
        subscriptions_endpoint_url.into(),
    ));

    || future::ready(html)
}

/// Creates a handler that replies with an HTML page containing [GraphQL Playground].
///
/// This does not handle routing, so you can mount it on any endpoint.
///
/// # Example
///
/// ```rust
/// use axum::{
///     routing::get,
///     Router
/// };
/// use axum::body::Body;
/// use juniper_axum::playground;
///
/// let app: Router<Body> = Router::new().route("/", get(playground("/graphql", "/subscriptions")));
/// ```
///
/// [GraphQL Playground]: https://github.com/prisma/graphql-playground
pub fn playground<'a>(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: impl Into<Option<&'a str>>,
) -> impl FnOnce() -> future::Ready<Html<String>> + Clone + Send {
    let html = Html(juniper::http::playground::playground_source(
        graphql_endpoint_url,
        subscriptions_endpoint_url.into(),
    ));

    || future::ready(html)
}
