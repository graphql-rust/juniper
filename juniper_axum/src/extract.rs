//! Types and traits for extracting data from requests.

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
    InputValue,
};
use serde::Deserialize;
use serde_json::{Map, Value};

/// Request body of a JSON POST request.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonRequestBody {
    Single(SingleRequestBody),
    Batch(Vec<SingleRequestBody>),
}

/// Request body of a single JSON POST request.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SingleRequestBody {
    query: String,
    operation_name: Option<String>,
    variables: Option<Map<String, Value>>,
}

impl JsonRequestBody {
    /// Indicates whether the request body represents an empty array.
    fn is_empty_batch(&self) -> bool {
        match self {
            JsonRequestBody::Batch(r) => r.is_empty(),
            JsonRequestBody::Single(_) => false,
        }
    }
}

/// Extractor for [`axum`] to extract a [`JuniperRequest`].
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
///
/// use axum::{
///     body::Body,
///     Json,
///     routing::post,
///     Router,
///     Extension,
/// };
/// use juniper::{
///     http::GraphQLBatchResponse,
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
/// let app: Router<Body> = Router::new()
///     .route("/graphql", post(graphql))
///     .layer(Extension(schema))
///     .layer(Extension(context));
///
/// async fn graphql(
///     JuniperRequest(request): JuniperRequest,
///     Extension(schema): Extension<Schema>,
///     Extension(context): Extension<Context>
/// ) -> JuniperResponse {
///     JuniperResponse(request.execute(&schema, &context).await)
/// }
#[derive(Debug, PartialEq)]
pub struct JuniperRequest(pub GraphQLBatchRequest);

impl TryFrom<SingleRequestBody> for JuniperRequest {
    type Error = serde_json::Error;

    fn try_from(value: SingleRequestBody) -> Result<JuniperRequest, Self::Error> {
        Ok(JuniperRequest(GraphQLBatchRequest::Single(
            GraphQLRequest::try_from(value)?,
        )))
    }
}

impl TryFrom<SingleRequestBody> for GraphQLRequest {
    type Error = serde_json::Error;

    fn try_from(value: SingleRequestBody) -> Result<GraphQLRequest, Self::Error> {
        // Convert Map<String, Value> to InputValue with the help of serde_json
        let variables: Option<InputValue> = value
            .variables
            .map(|vars| serde_json::to_string(&vars))
            .transpose()?
            .map(|s| serde_json::from_str(&s))
            .transpose()?;

        Ok(GraphQLRequest::new(
            value.query,
            value.operation_name,
            variables,
        ))
    }
}

impl TryFrom<JsonRequestBody> for JuniperRequest {
    type Error = serde_json::Error;

    fn try_from(value: JsonRequestBody) -> Result<JuniperRequest, Self::Error> {
        match value {
            JsonRequestBody::Single(req) => req.try_into(),
            JsonRequestBody::Batch(requests) => {
                let mut graphql_requests: Vec<GraphQLRequest> = Vec::new();

                for req in requests {
                    graphql_requests.push(GraphQLRequest::try_from(req)?);
                }

                Ok(JuniperRequest(GraphQLBatchRequest::Batch(graphql_requests)))
            }
        }
    }
}

impl From<String> for JuniperRequest {
    fn from(query: String) -> Self {
        JuniperRequest(GraphQLBatchRequest::Single(GraphQLRequest::new(
            query, None, None,
        )))
    }
}

#[async_trait]
impl<S> FromRequest<S, Body> for JuniperRequest
where
    S: Sync,
    Query<GraphQLRequest>: FromRequestParts<S>,
    Json<JsonRequestBody>: FromRequest<S, Body>,
    <Json<JsonRequestBody> as FromRequest<S, Body>>::Rejection: fmt::Display,
    String: FromRequest<S, Body>,
{
    type Rejection = Response;

    async fn from_request(mut req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get("content-type")
            .map(HeaderValue::to_str)
            .transpose()
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "`Content-Type` header is not a valid header string",
                )
                    .into_response()
            })?;

        match (req.method(), content_type) {
            (&Method::GET, _) => {
                let query = req
                    .extract_parts::<Query<GraphQLRequest>>()
                    .await
                    .map(|q| q.0)
                    .map_err(|e| {
                        (StatusCode::BAD_REQUEST, format!("Invalid request: {e}")).into_response()
                    })?;

                Ok(Self(GraphQLBatchRequest::Single(query)))
            }
            (&Method::POST, Some("application/json")) => {
                let json_body = Json::<JsonRequestBody>::from_request(req, state)
                    .await
                    .map(|result| result.0)
                    .map_err(|e| {
                        (StatusCode::BAD_REQUEST, format!("Invalid JSON: {e}")).into_response()
                    })?;

                if json_body.is_empty_batch() {
                    return Err(
                        (StatusCode::BAD_REQUEST, "Batch request cannot be empty").into_response()
                    );
                }

                Self::try_from(json_body).map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Could not convert variables: {e}"),
                    )
                        .into_response()
                })
            }
            (&Method::POST, Some("application/graphql")) => String::from_request(req, state)
                .await
                .map(Into::into)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Not valid UTF-8").into_response()),
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
mod tests {
    use axum::http::Request;
    use juniper::http::GraphQLRequest;

    use super::*;

    #[test]
    fn convert_simple_request_body_to_juniper_request() {
        let request_body = SingleRequestBody {
            query: "{ add(a: 2, b: 3) }".to_string(),
            operation_name: None,
            variables: None,
        };

        let expected = JuniperRequest(GraphQLBatchRequest::Single(GraphQLRequest::new(
            "{ add(a: 2, b: 3) }".to_string(),
            None,
            None,
        )));

        assert_eq!(JuniperRequest::try_from(request_body).unwrap(), expected);
    }

    #[tokio::test]
    async fn convert_get_request_to_juniper_request() {
        // /?query={ add(a: 2, b: 3) }
        let request = Request::get("/?query=%7B%20add%28a%3A%202%2C%20b%3A%203%29%20%7D")
            .body(Body::empty())
            .unwrap();
        let mut parts = RequestParts::new(request);

        let expected = JuniperRequest(GraphQLBatchRequest::Single(GraphQLRequest::new(
            "{ add(a: 2, b: 3) }".to_string(),
            None,
            None,
        )));

        let result = JuniperRequest::from_request(&mut parts).await.unwrap();
        assert_eq!(result, expected)
    }

    #[tokio::test]
    async fn convert_simple_post_request_to_juniper_request() {
        let json = String::from(r#"{ "query": "{ add(a: 2, b: 3) }"}"#);
        let request = Request::post("/")
            .header("content-type", "application/json")
            .body(Body::from(json))
            .unwrap();
        let mut parts = RequestParts::new(request);

        let expected = JuniperRequest(GraphQLBatchRequest::Single(GraphQLRequest::new(
            "{ add(a: 2, b: 3) }".to_string(),
            None,
            None,
        )));

        let result = JuniperRequest::from_request(&mut parts).await.unwrap();
        assert_eq!(result, expected)
    }

    #[tokio::test]
    async fn convert_simple_post_request_to_juniper_request_2() {
        let body = String::from(r#"{ add(a: 2, b: 3) }"#);
        let request = Request::post("/")
            .header("content-type", "application/graphql")
            .body(Body::from(body))
            .unwrap();
        let mut parts = RequestParts::new(request);

        let expected = JuniperRequest(GraphQLBatchRequest::Single(GraphQLRequest::new(
            "{ add(a: 2, b: 3) }".to_string(),
            None,
            None,
        )));

        let result = JuniperRequest::from_request(&mut parts).await.unwrap();
        assert_eq!(result, expected)
    }
}
