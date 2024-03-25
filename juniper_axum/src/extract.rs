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
use serde::Deserialize;

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
/// type Schema = RootNode<Query, EmptyMutation<Context>, EmptySubscription<Context>>;
///
/// let schema = Schema::new(
///    Query,
///    EmptyMutation::<Context>::new(),
///    EmptySubscription::<Context>::new()
/// );
///
/// let app: Router = Router::new()
///     .route("/graphql", post(graphql))
///     .layer(Extension(Arc::new(schema)))
///     .layer(Extension(Context));
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
impl<S, State> FromRequest<State> for JuniperRequest<S>
where
    S: ScalarValue,
    State: Sync,
    Query<GetRequest>: FromRequestParts<State>,
    Json<GraphQLBatchRequest<S>>: FromRequest<State>,
    <Json<GraphQLBatchRequest<S>> as FromRequest<State>>::Rejection: fmt::Display,
    String: FromRequest<State>,
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

        // TODO: Move into `match` expression directly once MSRV is bumped higher than 1.74.
        let method = req.method();
        match (method, content_type) {
            (&Method::GET, _) => req
                .extract_parts::<Query<GetRequest>>()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid request query string: {e}"),
                    )
                        .into_response()
                })
                .and_then(|query| {
                    query
                        .0
                        .try_into()
                        .map(|q| Self(GraphQLBatchRequest::Single(q)))
                        .map_err(|e| {
                            (
                                StatusCode::BAD_REQUEST,
                                format!("Invalid request query `variables`: {e}"),
                            )
                                .into_response()
                        })
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

/// Workaround for a [`GraphQLRequest`] not being [`Deserialize`]d properly from a GET query string,
/// containing `variables` in JSON format.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct GetRequest {
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    variables: Option<String>,
}

impl<S: ScalarValue> TryFrom<GetRequest> for GraphQLRequest<S> {
    type Error = serde_json::Error;
    fn try_from(req: GetRequest) -> Result<Self, Self::Error> {
        let GetRequest {
            query,
            operation_name,
            variables,
        } = req;
        Ok(Self::new(
            query,
            operation_name,
            variables.map(|v| serde_json::from_str(&v)).transpose()?,
        ))
    }
}

#[cfg(test)]
mod juniper_request_tests {
    use axum::{body::Body, extract::FromRequest as _, http::Request};
    use futures::TryStreamExt as _;
    use juniper::{
        graphql_input_value,
        http::{GraphQLBatchRequest, GraphQLRequest},
    };

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
    async fn from_get_request_with_variables() {
        let req = Request::get(&format!(
            "/?query={}&variables={}",
            urlencoding::encode(
                "query($id: String!) { human(id: $id) { id, name, appearsIn, homePlanet } }",
            ),
            urlencoding::encode(r#"{"id": "1000"}"#),
        ))
        .body(Body::empty())
        .unwrap_or_else(|e| panic!("cannot build `Request`: {e}"));

        let expected = JuniperRequest(GraphQLBatchRequest::Single(GraphQLRequest::new(
            "query($id: String!) { human(id: $id) { id, name, appearsIn, homePlanet } }".into(),
            None,
            Some(graphql_input_value!({"id": "1000"})),
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

    /// Converts the provided [`Body`] into a [`String`].
    async fn display_body(body: Body) -> String {
        String::from_utf8(
            body.into_data_stream()
                .map_ok(|bytes| bytes.to_vec())
                .try_concat()
                .await
                .unwrap(),
        )
        .unwrap_or_else(|e| panic!("not UTF-8 body: {e}"))
    }
}
