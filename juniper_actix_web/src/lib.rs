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
            &Method::POST => {
                let content_type_header = req
                    .headers()
                    .get(actix_web::http::header::CONTENT_TYPE)
                    .and_then(|hv| hv.to_str().ok());
                match content_type_header {
                    Some("application/json") => Box::new(
                        web::Json::<GraphQLBatchRequest<S>>::from_request(req, payload)
                            .map_err(Error::from)
                            .map(|gql_request| GraphQLRequest(gql_request.into_inner())),
                    ),
                    Some("application/graphql") => Box::new(
                        String::from_request(req, payload)
                            .map_err(|_| actix_http::error::ContentTypeError::ParseError.into())
                            .map(|query| {
                                GraphQLRequest(GraphQLBatchRequest::Single(
                                    JuniperGraphQLRequest::new(query, None, None),
                                ))
                            }),
                    ),
                    _ => Box::new(future::err(
                        actix_http::error::ContentTypeError::UnknownEncoding.into(),
                    )),
                }
            }
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
mod fromrequest_tests;

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
