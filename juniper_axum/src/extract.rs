//! Types and traits for extracting data from requests.

use axum::{
    async_trait,
    body::Body,
    extract::{FromRequest, Query, RequestParts},
    http::{Method, StatusCode},
    Json,
};
use juniper::{
    http::{GraphQLBatchRequest, GraphQLRequest},
    InputValue,
};
use serde::Deserialize;
use serde_json::{Map, Value};

/// The query variables for a GET request
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GetQueryVariables {
    query: String,
    operation_name: Option<String>,
    variables: Option<String>,
}

/// The request body for JSON POST
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum JsonRequestBody {
    Single(SingleRequestBody),
    Batch(Vec<SingleRequestBody>),
}

/// The request body for a single JSON POST request
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SingleRequestBody {
    query: String,
    operation_name: Option<String>,
    variables: Option<Map<String, Value>>,
}

impl JsonRequestBody {
    /// Returns true if the request body is an empty array
    fn is_empty_batch(&self) -> bool {
        match self {
            JsonRequestBody::Batch(r) => r.is_empty(),
            JsonRequestBody::Single(_) => false,
        }
    }
}

/// An extractor for Axum to Extract a JuniperRequest
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
            JsonRequestBody::Single(r) => JuniperRequest::try_from(r),
            JsonRequestBody::Batch(requests) => {
                let mut graphql_requests: Vec<GraphQLRequest> = Vec::new();

                for request in requests {
                    graphql_requests.push(GraphQLRequest::try_from(request)?);
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

impl TryFrom<GetQueryVariables> for JuniperRequest {
    type Error = serde_json::Error;

    fn try_from(value: GetQueryVariables) -> Result<JuniperRequest, Self::Error> {
        let variables: Option<InputValue> = value
            .variables
            .map(|var| serde_json::from_str(&var))
            .transpose()?;

        Ok(JuniperRequest(GraphQLBatchRequest::Single(
            GraphQLRequest::new(value.query, value.operation_name, variables),
        )))
    }
}

/// Helper trait to get some nice clean code
#[async_trait]
trait TryFromRequest {
    type Rejection;

    /// Get `content-type` header from request
    fn try_get_content_type_header(&self) -> Result<Option<&str>, Self::Rejection>;

    /// Try to convert GET request to RequestBody
    async fn try_from_get_request(&mut self) -> Result<JuniperRequest, Self::Rejection>;

    /// Try to convert POST json request to RequestBody
    async fn try_from_json_post_request(&mut self) -> Result<JuniperRequest, Self::Rejection>;

    /// Try to convert POST graphql request to RequestBody
    async fn try_from_graphql_post_request(&mut self) -> Result<JuniperRequest, Self::Rejection>;
}

#[async_trait]
impl TryFromRequest for RequestParts<Body> {
    type Rejection = (StatusCode, &'static str);

    fn try_get_content_type_header(&self) -> Result<Option<&str>, Self::Rejection> {
        self.headers()
            .get("content-Type")
            .map(|header| header.to_str())
            .transpose()
            .map_err(|_e| {
                (
                    StatusCode::BAD_REQUEST,
                    "content-type header not a valid string",
                )
            })
    }

    async fn try_from_get_request(&mut self) -> Result<JuniperRequest, Self::Rejection> {
        let query_vars = Query::<GetQueryVariables>::from_request(self)
            .await
            .map(|result| result.0)
            .map_err(|_err| (StatusCode::BAD_REQUEST, "Request not valid"))?;

        JuniperRequest::try_from(query_vars)
            .map_err(|_err| (StatusCode::BAD_REQUEST, "Could not convert variables"))
    }

    async fn try_from_json_post_request(&mut self) -> Result<JuniperRequest, Self::Rejection> {
        let json_body = Json::<JsonRequestBody>::from_request(self)
            .await
            .map_err(|_err| (StatusCode::BAD_REQUEST, "JSON invalid"))
            .map(|result| result.0)?;

        if json_body.is_empty_batch() {
            return Err((StatusCode::BAD_REQUEST, "Batch request can not be empty"));
        }

        JuniperRequest::try_from(json_body)
            .map_err(|_err| (StatusCode::BAD_REQUEST, "Could not convert variables"))
    }

    async fn try_from_graphql_post_request(&mut self) -> Result<JuniperRequest, Self::Rejection> {
        String::from_request(self)
            .await
            .map(|s| s.into())
            .map_err(|_err| (StatusCode::BAD_REQUEST, "Not valid utf-8"))
    }
}

#[async_trait]
impl FromRequest<Body> for JuniperRequest {
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        let content_type = req.try_get_content_type_header()?;

        // Convert `req` to JuniperRequest based on request method and content-type header
        match (req.method(), content_type) {
            (&Method::GET, _) => req.try_from_get_request().await,
            (&Method::POST, Some("application/json")) => req.try_from_json_post_request().await,
            (&Method::POST, Some("application/graphql")) => {
                req.try_from_graphql_post_request().await
            }
            (&Method::POST, _) => Err((
                StatusCode::BAD_REQUEST,
                "Header content-type is not application/json or application/graphql",
            )),
            _ => Err((StatusCode::METHOD_NOT_ALLOWED, "Method not supported")),
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
