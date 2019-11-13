use futures::{future, Future};

use actix_web::http::{Method, StatusCode};
use actix_web::{dev, web, Error, FromRequest, HttpRequest, HttpResponse, Responder};

use juniper::http::{
    graphiql, playground, GraphQLRequest as JuniperGraphQLRequest,
    GraphQLResponse as JuniperGraphQLResponse,
};

use juniper::serde::Deserialize;
use juniper::{
    DefaultScalarValue, FieldError, GraphQLType, InputValue, RootNode, ScalarRefValue, ScalarValue,
};

use serde::{de, Deserializer};

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
    S: ScalarValue,
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
            &GraphQLBatchRequest::Single(ref request) => {
                GraphQLBatchResponse::Single(request.execute(root_node, context))
            }
            &GraphQLBatchRequest::Batch(ref requests) => GraphQLBatchResponse::Batch(
                requests
                    .iter()
                    .map(|request| request.execute(root_node, context))
                    .collect(),
            ),
        }
    }

    pub fn operation_names(&self) -> Vec<Option<&str>> {
        match self {
            GraphQLBatchRequest::Single(request) => vec![request.operation_name()],
            GraphQLBatchRequest::Batch(requests) => {
                requests.iter().map(|req| req.operation_name()).collect()
            }
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

use serde::de::DeserializeOwned;

fn deserialize_from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    // let s: &'de str = Deserialize::deserialize(deserializer)?;
    let data: String = Deserialize::deserialize(deserializer)?;
    serde_json::from_str(&data).map_err(de::Error::custom)
}

#[serde(deny_unknown_fields)]
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct StrictGraphQLRequest<S = DefaultScalarValue>
where
    S: ScalarValue,
{
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    #[serde(bound(deserialize = "InputValue<S>: DeserializeOwned"))]
    #[serde(default = "Option::default")]
    #[serde(deserialize_with = "deserialize_from_str")]
    variables: Option<InputValue<S>>,
}

impl<S> Into<JuniperGraphQLRequest<S>> for StrictGraphQLRequest<S>
where
    S: ScalarValue,
{
    fn into(self) -> JuniperGraphQLRequest<S> {
        JuniperGraphQLRequest::<S>::new(self.query, self.operation_name, self.variables)
    }
}

impl<S> FromRequest for GraphQLRequest<S>
where
    S: ScalarValue + 'static,
{
    type Error = actix_web::Error;
    type Future = Box<dyn Future<Item = Self, Error = Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, payload: &mut dev::Payload) -> Self::Future {
        match req.method() {
            &Method::GET => Box::new(future::result(
                web::Query::<StrictGraphQLRequest<S>>::from_query(req.query_string())
                    .map_err(Error::from)
                    .map(|gql_request| {
                        GraphQLRequest(GraphQLBatchRequest::Single(gql_request.into_inner().into()))
                    }),
            )),
            &Method::POST => Box::new(
                web::Json::<GraphQLBatchRequest<S>>::from_request(req, payload)
                    .map_err(Error::from)
                    .map(|gql_request| GraphQLRequest(gql_request.into_inner())),
            ),
            _ => Box::new(future::result(Err(
                actix_http::error::ErrorMethodNotAllowed(
                    "GraphQL requests can only be sent with GET or POST",
                ),
            ))),
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
    use actix_web::http::header;
    use actix_web::test::TestRequest;

    fn req_is_single(req: &GraphQLRequest) -> bool {
        if let GraphQLRequest(GraphQLBatchRequest::Single(_)) = req {
            true
        } else {
            false
        }
    }

    // Get tests
    const URI_PREFIX: &'static str = "http://test.com";

    #[test]
    fn test_empty_get() {
        let (req, mut payload) = TestRequest::get()
            .uri(&format!("{}", URI_PREFIX))
            .to_http_parts();
        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Query deserialize error: missing field `query`"
        );
    }

    #[test]
    fn test_no_query() {
        let (req, mut payload) = TestRequest::get()
            .uri("http://example.com?operationName=foo&variables={}")
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Query deserialize error: missing field `query`"
        );
    }

    #[test]
    fn test_normal_get() {
        let query = "{foo{bar,baz}}";
        let (req, mut payload) = TestRequest::get()
            .uri(&format!(
                "{}?query={}&operationName=rust",
                URI_PREFIX, query
            ))
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_ok());
        assert!(req_is_single(&result.unwrap()));
    }

    #[test]
    fn test_get_all_fields() {
        use url::form_urlencoded;
        let query = "query a($qux: Qux) { foo(qux: $qux) { bar } } query b { foo { baz } }";
        let operation_name = "b";
        let variables = r#"{
            "qux": "quux"
        }"#;

        let encoded: String = form_urlencoded::Serializer::new(String::new())
            .append_pair("query", query)
            .append_pair("operationName", operation_name)
            .append_pair("variables", variables)
            .finish();

        let (req, mut payload) = TestRequest::get()
            .uri(&format!("{}?{}", URI_PREFIX, encoded))
            .to_http_parts();
        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_ok());
        assert!(req_is_single(&result.unwrap()));
    }

    #[test]
    fn test_get_extra_fields() {
        let query = "{foo{bar,baz}}";
        let (req, mut payload) = TestRequest::get()
            .uri(&format!(
                "{}?query={}&operationName=rust&foo=bar",
                URI_PREFIX, query
            ))
            .to_http_parts();
        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .starts_with("Query deserialize error: unknown field `foo`"))
    }

    #[test]
    fn test_get_duplicate_query() {
        let query = "{foo{bar,baz}}";
        let (req, mut payload) = TestRequest::get()
            .uri(&format!(
                "{}?query={}&operationName=rust&query=bar",
                URI_PREFIX, query
            ))
            .to_http_parts();
        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Query deserialize error: duplicate field `query`"
        );
    }

    #[test]
    fn test_get_duplicate_operation_name() {
        let query = "{foo{bar,baz}}";
        let (req, mut payload) = TestRequest::get()
            .uri(&format!(
                "{}?query={}&operationName=rust&operationName=bar",
                URI_PREFIX, query
            ))
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Query deserialize error: duplicate field `operationName`"
        );
    }

    // Post tests
    #[test]
    fn test_empty_post_single() {
        let (req, mut payload) = TestRequest::post()
            .set_payload("{}")
            .header(header::CONTENT_TYPE, "application/json")
            .to_http_parts();

        let result: Result<GraphQLRequest, _> =
            GraphQLRequest::from_request(&req, &mut payload).wait();
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
            .set_payload(
                r#"{
                    "query": "{foo { bar }}"
                }"#,
            )
            .header(header::CONTENT_TYPE, "content/json")
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_ok());
        assert!(req_is_single(&result.unwrap()));
    }

    #[test]
    fn test_post_batch() {
        let (req, mut payload) = TestRequest::post()
            .set_payload(
                r#"[
                    { "query": "{ foo { bar } }" },
                    { "query": "{ baz { qux } }" }
                ]"#,
            )
            .header(header::CONTENT_TYPE, "content/json")
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_ok());
        assert!(!req_is_single(&result.unwrap()));
    }

    #[test]
    fn test_post_duplicate_field() {
        let (req, mut payload) = TestRequest::post()
            .set_payload(
                r#"{
                    "query": "foo",
                    "query": "bar"
                }"#,
            )
            .header(header::CONTENT_TYPE, "content/json")
            .to_http_parts();

        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        assert!(result.is_err());
    }

    #[test]
    fn test_post_variables() {
        let (req, mut payload) = TestRequest::post()
            .set_payload(
                r#"{
                "query": "meep",
                "variables": {"foo": "bar"}
            }"#,
            )
            .header(header::CONTENT_TYPE, "content/json")
            .to_http_parts();
        let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
        // println!("ERR: {:?}", &result.unwrap_err().to_string());
        assert!(result.is_ok());
        assert!(req_is_single(&result.unwrap()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_rt;
    use actix_web::{guard, web, App, HttpServer};
    use futures::lazy;
    use juniper::{
        http::tests::{run_http_test_suite, HTTPIntegration, TestResponse},
        tests::{model::Database, schema::Query},
        EmptyMutation, RootNode,
    };
    use std::sync::Arc;

    type Schema = RootNode<'static, Query, EmptyMutation<Database>>;

    struct Data {
        schema: Schema,
        context: Database,
    }

    struct TestActixWebIntegration;

    impl TestActixWebIntegration {
        const fn server_url() -> &'static str {
            "127.0.0.1:8088"
        }
    }

    impl HTTPIntegration for TestActixWebIntegration {
        fn get(&self, url: &str) -> TestResponse {
            let url = format!("http://{}{}", TestActixWebIntegration::server_url(), url);
            actix_rt::System::new("get_request")
                .block_on(lazy(|| {
                    awc::Client::default()
                        .get(&url)
                        .send()
                        .map(make_test_response)
                }))
                .expect(&format!("failed GET {}", url))
        }

        fn post(&self, url: &str, body: &str) -> TestResponse {
            let url = format!("http://{}{}", TestActixWebIntegration::server_url(), url);
            actix_rt::System::new("post_request")
                .block_on(lazy(|| {
                    awc::Client::default()
                        .post(&url)
                        .header(awc::http::header::CONTENT_TYPE, "application/json")
                        .send_body(body.to_string())
                        .map(make_test_response)
                }))
                .expect(&format!("failed POST {}", url))
        }
    }

    type Resp = awc::ClientResponse<
        actix_http::encoding::Decoder<actix_http::Payload<actix_http::PayloadStream>>,
    >;

    fn make_test_response(mut response: Resp) -> TestResponse {
        let status_code = response.status().as_u16() as i32;
        let body = response
            .body()
            .wait()
            .ok()
            .and_then(|body| String::from_utf8(body.to_vec()).ok());
        let content_type_header = response
            .headers()
            .get(actix_web::http::header::CONTENT_TYPE);
        let content_type = content_type_header
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or_default()
            .to_string();

        TestResponse {
            status_code,
            body,
            content_type,
        }
    }

    #[test]
    fn test_actix_web_integration() {
        let schema = Schema::new(Query, EmptyMutation::<Database>::new());
        let context = Database::new();
        let data = Arc::new(Data { schema, context });

        let (tx, rx) = std::sync::mpsc::channel();
        let join = std::thread::spawn(move || {
            let sys = actix_rt::System::new("test-integration-server");
            let app = HttpServer::new(move || {
                App::new().data(data.clone()).service(
                    web::resource("/")
                        .guard(guard::Any(guard::Get()).or(guard::Post()))
                        .to(|st: web::Data<Arc<Data>>, data: GraphQLRequest| {
                            data.execute(&st.schema, &st.context)
                        }),
                )
            })
            .shutdown_timeout(0)
            .bind(TestActixWebIntegration::server_url())
            .unwrap();

            tx.send(app.system_exit().start()).unwrap();
            sys.run().unwrap();
        });

        let server = rx.recv().unwrap();
        server.resume();

        run_http_test_suite(&TestActixWebIntegration);

        server.stop(true);
        join.join().unwrap();
    }
}
