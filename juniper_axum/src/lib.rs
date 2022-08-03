#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(warnings)]

pub mod extract;
pub mod response;
pub mod subscriptions;

use axum::response::Html;
use futures::future;

/// Add a GraphQL Playground
///
/// # Arguments
///
/// * `graphql_endpoint_url` - The graphql endpoint you configured
/// * `subscriptions_endpoint_url` - An optional subscription endpoint
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
