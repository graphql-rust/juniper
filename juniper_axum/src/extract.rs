//! Types and traits for extracting data from [`Request`]s.

use std::fmt;

use axum::{
    async_trait,
    body::Body,
    extract::{FromRequest, FromRequestParts, Query},
    http::{HeaderValue, Method, Request, StatusCode},
    response::{IntoResponse as _, Response},
    Json, RequestExt as _,
};
use juniper::{
    http::{GraphQLBatchRequest, GraphQLRequest},
    DefaultScalarValue, ScalarValue,
};

/// Extractor for [`axum`] to extract a [`JuniperRequest`].
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
/// use juniper_axum::{extract::JuniperRequest, response::JuniperResponse};
///
/// #[derive(Clone, Copy, Debug)]
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
/// let context = Context;
///
/// let app: Router = Router::new()
///     .route("/graphql", post(graphql))
///     .layer(Extension(Arc::new(schema)))
///     .layer(Extension(context));
///
/// # #[axum::debug_handler]
/// async fn graphql(
///     Extension(schema): Extension<Arc<Schema>>,
///     Extension(context): Extension<Context>,
///     JuniperRequest(req): JuniperRequest, // should be the last argument as consumes `Request`
/// ) -> JuniperResponse {
///     JuniperResponse(req.execute(&*schema, &context).await)
/// }
#[derive(Debug, PartialEq)]
pub struct JuniperRequest<S = DefaultScalarValue>(pub GraphQLBatchRequest<S>)
where
    S: ScalarValue;

#[async_trait]
impl<S, State> FromRequest<State, Body> for JuniperRequest<S>
where
    S: ScalarValue,
    State: Sync,
    Query<GraphQLRequest<S>>: FromRequestParts<State>,
    Json<GraphQLBatchRequest<S>>: FromRequest<State, Body>,
    <Json<GraphQLBatchRequest<S>> as FromRequest<State, Body>>::Rejection: fmt::Display,
    String: FromRequest<State, Body>,
{
    type Rejection = Response;

    async fn from_request(mut req: Request<Body>, state: &State) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get("content-type")
            .map(HeaderValue::to_str)
            .transpose()
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "`Content-Type` header is not a valid HTTP header string",
                )
                    .into_response()
            })?;

        match (req.method(), content_type) {
            (&Method::GET, _) => req
                .extract_parts::<Query<GraphQLRequest<S>>>()
                .await
                .map(|query| Self(GraphQLBatchRequest::Single(query.0)))
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid request query string: {e}"),
                    )
                        .into_response()
                }),
            (&Method::POST, Some("application/json")) => {
                Json::<GraphQLBatchRequest<S>>::from_request(req, state)
                    .await
                    .map(|req| Self(req.0))
                    .map_err(|e| {
                        (StatusCode::BAD_REQUEST, format!("Invalid JSON body: {e}")).into_response()
                    })
            }
            (&Method::POST, Some("application/graphql")) => String::from_request(req, state)
                .await
                .map(|body| {
                    Self(GraphQLBatchRequest::Single(GraphQLRequest::new(
                        body, None, None,
                    )))
                })
                .map_err(|_| (StatusCode::BAD_REQUEST, "Not valid UTF-8 body").into_response()),
            (&Method::POST, _) => Err((
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "`Content-Type` header is expected to be either `application/json` or \
                 `application/graphql`",
            )
                .into_response()),
            _ => Err((
                StatusCode::METHOD_NOT_ALLOWED,
                "HTTP method is expected to be either GET or POST",
            )
                .into_response()),
        }
    }
}

#[cfg(test)]
mod juniper_request_tests {
    use std::fmt;

    use axum::{
        body::{Body, Bytes, HttpBody},
        extract::FromRequest as _,
        http::Request,
    };
    use juniper::http::{GraphQLBatchRequest, GraphQLRequest};

    use super::JuniperRequest;

    #[tokio::test]
    async fn from_get_request() {
        let req = Request::get(&format!(
            "/?query={}",
            urlencoding::encode("{ add(a: 2, b: 3) }")
        ))
        .body(Body::empty())
        .unwrap_or_else(|e| panic!("cannot build `Request`: {e}"));

        let expected = JuniperRequest(GraphQLBatchRequest::Single(GraphQLRequest::new(
            "{ add(a: 2, b: 3) }".into(),
            None,
            None,
        )));

        assert_eq!(do_from_request(req).await, expected);
    }

    #[tokio::test]
    async fn from_json_post_request() {
        let req = Request::post("/")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"query": "{ add(a: 2, b: 3) }"}"#))
            .unwrap_or_else(|e| panic!("cannot build `Request`: {e}"));

        let expected = JuniperRequest(GraphQLBatchRequest::Single(GraphQLRequest::new(
            "{ add(a: 2, b: 3) }".to_string(),
            None,
            None,
        )));

        assert_eq!(do_from_request(req).await, expected);
    }

    #[tokio::test]
    async fn from_graphql_post_request() {
        let req = Request::post("/")
            .header("content-type", "application/graphql")
            .body(Body::from(r#"{ add(a: 2, b: 3) }"#))
            .unwrap_or_else(|e| panic!("cannot build `Request`: {e}"));

        let expected = JuniperRequest(GraphQLBatchRequest::Single(GraphQLRequest::new(
            "{ add(a: 2, b: 3) }".to_string(),
            None,
            None,
        )));

        assert_eq!(do_from_request(req).await, expected);
    }

    /// Performs [`JuniperRequest::from_request()`].
    async fn do_from_request(req: Request<Body>) -> JuniperRequest {
        match JuniperRequest::from_request(req, &()).await {
            Ok(resp) => resp,
            Err(resp) => {
                panic!(
                    "`JuniperRequest::from_request()` failed with `{}` status and body:\n{}",
                    resp.status(),
                    display_body(resp.into_body()).await,
                )
            }
        }
    }

    /// Converts the provided [`HttpBody`] into a [`String`].
    async fn display_body<B>(body: B) -> String
    where
        B: HttpBody<Data = Bytes>,
        B::Error: fmt::Display,
    {
        let bytes = hyper::body::to_bytes(body)
            .await
            .unwrap_or_else(|e| panic!("failed to represent `Body` as `Bytes`: {e}"));
        String::from_utf8(bytes.into()).unwrap_or_else(|e| panic!("not UTF-8 body: {e}"))
    }
}
