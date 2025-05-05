#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(any(doc, test), doc = include_str!("../README.md"))]
#![cfg_attr(not(any(doc, test)), doc = env!("CARGO_PKG_NAME"))]

// TODO: Try remove on upgrade of `axum` crate.
mod for_minimal_versions_check_only {
    use bytes as _;
}

pub mod extract;
pub mod response;
#[cfg(feature = "subscriptions")]
pub mod subscriptions;

use std::future;

use axum::{extract::Extension, response::Html};
use juniper_graphql_ws::Schema;

use self::{extract::JuniperRequest, response::JuniperResponse};

#[cfg(feature = "subscriptions")]
#[doc(inline)]
pub use self::subscriptions::{graphql_transport_ws, graphql_ws, ws};

/// [`Handler`], which handles a [`JuniperRequest`] with the specified [`Schema`], by [`extract`]ing
/// it from [`Extension`]s and initializing its fresh [`Schema::Context`] as a [`Default`] one.
///
/// > __NOTE__: This is a ready-to-go default [`Handler`] for serving GraphQL requests. If you need
/// >           to customize it (for example, extract [`Schema::Context`] from [`Extension`]s
/// >           instead initializing a [`Default`] one), create your own [`Handler`] accepting a
/// >           [`JuniperRequest`] (see its documentation for examples).
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
///
/// use axum::{routing::post, Extension, Json, Router};
/// use juniper::{
///     RootNode, EmptySubscription, EmptyMutation, graphql_object,
/// };
/// use juniper_axum::graphql;
///
/// #[derive(Clone, Copy, Debug, Default)]
/// pub struct Context;
///
/// impl juniper::Context for Context {}
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Query;
///
/// #[graphql_object(context = Context)]
/// impl Query {
///     fn add(a: i32, b: i32) -> i32 {
///         a + b
///     }
/// }
///
/// type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;
///
/// let schema = Schema::new(
///    Query,
///    EmptyMutation::<Context>::new(),
///    EmptySubscription::<Context>::new()
/// );
///
/// let app: Router = Router::new()
///     .route("/graphql", post(graphql::<Arc<Schema>>))
///     .layer(Extension(Arc::new(schema)));
/// ```
///
/// [`extract`]: axum::extract
/// [`Handler`]: axum::handler::Handler
pub async fn graphql<S>(
    Extension(schema): Extension<S>,
    JuniperRequest(req): JuniperRequest<S::ScalarValue>,
) -> JuniperResponse<S::ScalarValue>
where
    S: Schema, // TODO: Refactor in the way we don't depend on `juniper_graphql_ws::Schema` here.
    S::Context: Default,
{
    JuniperResponse(
        req.execute(schema.root_node(), &S::Context::default())
            .await,
    )
}

/// Creates a [`Handler`] that replies with an HTML page containing [GraphiQL].
///
/// This does not handle routing, so you can mount it on any endpoint.
///
/// # Example
///
/// ```rust
/// use axum::{routing::get, Router};
/// use juniper_axum::graphiql;
///
/// let app: Router = Router::new()
///     .route("/", get(graphiql("/graphql", "/subscriptions")));
/// ```
///
/// [`Handler`]: axum::handler::Handler
/// [GraphiQL]: https://github.com/graphql/graphiql
pub fn graphiql<'a, T: Into<Option<&'a str>>>(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: T,
) -> impl FnOnce() -> future::Ready<Html<String>> + Clone + Send + use<T> {
    let html = Html(juniper::http::graphiql::graphiql_source(
        graphql_endpoint_url,
        subscriptions_endpoint_url.into(),
    ));

    || future::ready(html)
}

/// Creates a [`Handler`] that replies with an HTML page containing [GraphQL Playground].
///
/// This does not handle routing, so you can mount it on any endpoint.
///
/// # Example
///
/// ```rust
/// use axum::{routing::get, Router};
/// use juniper_axum::playground;
///
/// let app: Router = Router::new()
///     .route("/", get(playground("/graphql", "/subscriptions")));
/// ```
///
/// [`Handler`]: axum::handler::Handler
/// [GraphQL Playground]: https://github.com/prisma/graphql-playground
pub fn playground<'a, T: Into<Option<&'a str>>>(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: T,
) -> impl FnOnce() -> future::Ready<Html<String>> + Clone + Send + use<T> {
    let html = Html(juniper::http::playground::playground_source(
        graphql_endpoint_url,
        subscriptions_endpoint_url.into(),
    ));

    || future::ready(html)
}
