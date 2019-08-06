use futures::{Future, future};

use actix_web::{
    web,
    dev,
    Error,
    HttpRequest,
    Responder,
    HttpResponse,
    FromRequest,
};
use actix_web::http::{Method, StatusCode};


use juniper::http::{
    GraphQLRequest as JuniperGraphQLRequest,
    GraphQLResponse as JuniperGraphQLResponse,
    graphiql,
    playground,
};

use juniper::serde::Deserialize;
use juniper::{
    GraphQLType,
    RootNode,
    FieldError,
    ScalarValue,
    ScalarRefValue,
    DefaultScalarValue,
    InputValue
};

use serde::{Deserializer, de};

fn deserialize_non_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let v = Vec::<T>::deserialize(deserializer)?;

    if v.is_empty() {
        Err(de::Error::invalid_length(0, &"a positive integer"))
    } else {
        Ok(v)
    }
}

#[derive(Debug, serde_derive::Deserialize, PartialEq)]
#[serde(untagged)]
#[serde(bound = "InputValue<S>: Deserialize<'de>")]
enum GraphQLBatchRequest<S = DefaultScalarValue>
where
    S: ScalarValue,
{
    Single(JuniperGraphQLRequest<S>),
    #[serde(deserialize_with = "deserialize_non_empty_vec")]
    Batch(Vec<JuniperGraphQLRequest<S>>),
}

#[derive(serde_derive::Serialize)]
#[serde(untagged)]
enum GraphQLBatchResponse<'a, S = DefaultScalarValue>
where
    S: ScalarValue
{
    Single(JuniperGraphQLResponse<'a, S>),
    Batch(Vec<JuniperGraphQLResponse<'a, S>>),
}

impl<S> GraphQLBatchRequest<S>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    pub fn execute<'a, CtxT, QueryT, MutationT>(
        &'a self,
        root_node: &'a RootNode<QueryT, MutationT, S>,
        context: &CtxT,
    ) -> GraphQLBatchResponse<'a, S>
    where
        QueryT: GraphQLType<S, Context = CtxT>,
        MutationT: GraphQLType<S, Context = CtxT>,
    {
        match self {
            &GraphQLBatchRequest::Single(ref request) => GraphQLBatchResponse::Single(
                request.execute(root_node, context)
            ),
            &GraphQLBatchRequest::Batch(ref requests) => GraphQLBatchResponse::Batch(
                requests
                    .iter()
                    .map(|request| request.execute(root_node, context))
                    .collect(),
            )
        }
    }

    pub fn operation_names(&self) -> Vec<Option<&str>> {
        match self {
            GraphQLBatchRequest::Single(request) => vec![request.operation_name()],
            GraphQLBatchRequest::Batch(requests) => requests
                    .iter()
                    .map(|req| req.operation_name())
                    .collect(),
        }
    }
}

impl<'a, S> GraphQLBatchResponse<'a, S>
where
    S: ScalarValue,
{
    fn is_ok(&self) -> bool {
        match self {
            &GraphQLBatchResponse::Single(ref response) => response.is_ok(),
            &GraphQLBatchResponse::Batch(ref responses) => responses
                .iter()
                .fold(true, |ok, response| ok && response.is_ok()),
        }
    }
}

/// Single wrapper around an incoming GraphQL request
/// 
/// See the http module for information. This type can be constructed
/// automatically requests by implementing the FromRequest trait.
#[derive(Debug, PartialEq, serde_derive::Deserialize)]
pub struct GraphQLRequest<S = DefaultScalarValue>(GraphQLBatchRequest<S>)
where
    S: ScalarValue;

/// Simple wrapper around the result of executing a GraphQL query
pub struct GraphQLResponse(pub StatusCode, pub String);

/// Generate a HTML page containing GraphiQL
pub fn graphiql_source(graphql_endpoint_url: &str) -> HttpResponse {
    let html = graphiql::graphiql_source(graphql_endpoint_url);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

/// Generate a HTML page containing GraphQL Playground
pub fn playground_source(graphql_endpoint_url: &str) -> HttpResponse {
    let html = playground::playground_source(graphql_endpoint_url);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

impl<S> GraphQLRequest<S>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    /// Execute an incoming GraphQL query
    pub fn execute<CtxT, QueryT, MutationT>(
        &self,
        root_node: &RootNode<QueryT, MutationT, S>,
        context: &CtxT,
    ) -> GraphQLResponse
    where
        QueryT: GraphQLType<S, Context = CtxT>,
        MutationT: GraphQLType<S, Context = CtxT>,
    {
        let response = self.0.execute(root_node, context);
        let status = if response.is_ok() {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        let json = serde_json::to_string(&response).unwrap();

        GraphQLResponse(status, json)
    }

    /// Returns the operation names associated with this request.
    /// 
    /// For batch requests there will be multiple names.
    pub fn operation_names(&self) -> Vec<Option<&str>> {
        self.0.operation_names()
    }
}

impl GraphQLResponse {
    pub fn error(error: FieldError) -> Self {
        let response = JuniperGraphQLResponse::error(error);
        let json = serde_json::to_string(&response).unwrap();
        GraphQLResponse(StatusCode::BAD_REQUEST, json)
    }

    pub fn custom(status: StatusCode, response: serde_json::Value) -> Self {
        let json = serde_json::to_string(&response).unwrap();
        GraphQLResponse(status, json)
    }
}

impl<S> FromRequest for GraphQLRequest<S>
where
    S: ScalarValue + 'static
{
    type Error = actix_web::Error;
    type Future = Box<dyn Future<Item = Self, Error = Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, payload: &mut dev::Payload) -> Self::Future {
        match req.method() {
            &Method::GET => Box::new(future::result(
                web::Query::<JuniperGraphQLRequest<S>>::from_query(req.query_string())
                    .map_err(Error::from)
                    .map(|gql_request| GraphQLRequest(
                        GraphQLBatchRequest::Single(gql_request.into_inner())
                    ))
            )),
            &Method::POST => Box::new(
                web::Json::<GraphQLBatchRequest<S>>::from_request(req, payload)
                    .map_err(Error::from)
                    .map(|gql_request| GraphQLRequest(gql_request.into_inner()))
            ),
            _ => Box::new(future::result(Err(actix_http::error::ErrorMethodNotAllowed("GraphQL requests can only be sent with GET or POST")))),
        }
    }
}

impl Responder for GraphQLResponse {
    type Error = actix_web::Error;
    type Future = Result<HttpResponse, Error>;
    fn respond_to(self, _: &HttpRequest) -> Self::Future {
        let GraphQLResponse(status, body) = self;

        Ok(HttpResponse::Ok()
            .status(status)
            .content_type("application/json")
            .body(body))
    }
}

#[cfg(test)]
mod fromrequest_tests {
    use super::*;
    use juniper::InputValue;
    use actix_web::test::TestRequest;
    use actix_web::http::header;

    fn req_is_single(req: &GraphQLRequest) -> bool {
        if let GraphQLRequest(GraphQLBatchRequest::Single(_)) = req {
            true
        } else {
            false
        }
    }

    // Get tests
    const URI_PREFIX: &'static str =  "http://example.com";

    #[test]
    fn test_empty_get() {
        let (req, mut payload) = TestRequest::get()
            .uri(&format!("{}", URI_PREFIX))
            .to_http_parts();
        
        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Query deserialize error: missing field `query`");
    }

    #[test]
    fn test_no_query() {
        let (req, mut payload) = TestRequest::get()
            .uri("http://example.com?operation_name=foo&variables={}")
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Query deserialize error: missing field `query`");
    }

    #[test]
    fn test_get() {
        let query = "{foo{bar,baz}}";
        let (req, mut payload) = TestRequest::get()
            .uri(&format!("{}?query={}&operationName=cheese&foo=bar", URI_PREFIX, query))
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_ok());
        assert!(req_is_single(&result.unwrap()));
    }

    // Post tests
    #[test]
    fn test_empty_post_single() {
        let (req, mut payload) = TestRequest::post()
            .set_payload("{}")
            .header(header::CONTENT_TYPE, "application/json")
            .to_http_parts();

        let result: Result<GraphQLRequest, _> = GraphQLRequest::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_post_batch() {
        let (req, mut payload) = TestRequest::post()
            .set_payload("[]")
            .header(header::CONTENT_TYPE, "application/json")
            .to_http_parts();
        
        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
    }

    #[test]
    fn test_post_single() {
        let (req, mut payload) = TestRequest::post()
            .set_payload(r#"{
                "query": "{foo { bar }}"
            }"#)
            .header(header::CONTENT_TYPE, "content/json")
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_ok());
        assert!(req_is_single(&result.unwrap()));
    }

    #[test]
    fn test_post_batch() {
        let (req, mut payload) = TestRequest::post()
            .set_payload(r#"[
                {
                    "query": "{foo { bar }}"
                }
            ]"#)
            .header(header::CONTENT_TYPE, "content/json")
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_ok());
        assert!(!req_is_single(&result.unwrap()));
    }

}

#[cfg(test)]
mod tests {

}
