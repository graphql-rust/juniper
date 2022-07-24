use axum::response::Html;

pub mod extract;
pub mod response;
pub mod subscriptions;

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
/// let app: Router<Body> = Router::new().route("/", get(|| playground("/graphql", Some("/subscriptions"))));
/// ```
pub async fn playground(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: Option<&str>,
) -> Html<String> {
    Html(juniper::http::playground::playground_source(
        graphql_endpoint_url,
        subscriptions_endpoint_url,
    ))
}
