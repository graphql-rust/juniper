#![feature(extern_prelude)]

extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate juniper;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
extern crate reqwest;
extern crate serde_json;
#[cfg(test)]
extern crate tokio;
extern crate url;

use futures::{future, Future};
use futures_cpupool::CpuPool;
use hyper::header::HeaderValue;
use hyper::rt::Stream;
use hyper::{header, Body, Method, Request, Response, StatusCode};
use juniper::http::{
    GraphQLRequest as JuniperGraphQLRequest, GraphQLResponse as JuniperGraphQLResponse,
};
use juniper::{GraphQLType, InputValue, RootNode};
use serde_json::error::Error as SerdeError;
use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;
use std::sync::Arc;
use url::form_urlencoded;

pub fn graphql<CtxT, QueryT, MutationT>(
    pool: CpuPool,
    root_node: Arc<RootNode<'static, QueryT, MutationT>>,
    context: Arc<CtxT>,
    request: Request<Body>,
) -> Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>
where
    CtxT: Send + Sync + 'static,
    QueryT: GraphQLType<Context = CtxT> + Send + Sync + 'static,
    MutationT: GraphQLType<Context = CtxT> + Send + Sync + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT::TypeInfo: Send + Sync,
{
    match request.method() {
        &Method::GET => Box::new(
            future::done(
                request
                    .uri()
                    .query()
                    .map(|q| gql_request_from_get(q).map(GraphQLRequest::Single))
                    .unwrap_or(Err(GraphQLRequestError::Invalid(
                        "'query' parameter is missing".to_string(),
                    ))),
            ).and_then(move |gql_req| execute_request(pool, root_node, context, gql_req))
            .or_else(|err| future::ok(render_error(err))),
        ),
        &Method::POST => Box::new(
            request
                .into_body()
                .concat2()
                .or_else(|err| future::done(Err(GraphQLRequestError::BodyHyper(err))))
                .and_then(move |chunk| {
                    future::done({
                        String::from_utf8(chunk.iter().cloned().collect::<Vec<u8>>())
                            .map_err(GraphQLRequestError::BodyUtf8)
                            .and_then(|input| {
                                serde_json::from_str::<GraphQLRequest>(&input)
                                    .map_err(GraphQLRequestError::BodyJSONError)
                            })
                    })
                }).and_then(move |gql_req| execute_request(pool, root_node, context, gql_req))
                .or_else(|err| future::ok(render_error(err))),
        ),
        _ => return Box::new(future::ok(new_response(StatusCode::METHOD_NOT_ALLOWED))),
    }
}

pub fn graphiql(
    graphql_endpoint: &str,
) -> Box<Future<Item = Response<Body>, Error = hyper::Error> + Send> {
    let mut resp = new_html_response(StatusCode::OK);
    *resp.body_mut() = Body::from(juniper::graphiql::graphiql_source(graphql_endpoint));
    Box::new(future::ok(resp))
}

fn render_error(err: GraphQLRequestError) -> Response<Body> {
    let message = format!("{}", err);
    let mut resp = new_response(StatusCode::BAD_REQUEST);
    *resp.body_mut() = Body::from(message);
    resp
}

fn execute_request<CtxT, QueryT, MutationT, Err>(
    pool: CpuPool,
    root_node: Arc<RootNode<'static, QueryT, MutationT>>,
    context: Arc<CtxT>,
    request: GraphQLRequest,
) -> impl Future<Item = Response<Body>, Error = Err>
where
    CtxT: Send + Sync + 'static,
    QueryT: GraphQLType<Context = CtxT> + Send + Sync + 'static,
    MutationT: GraphQLType<Context = CtxT> + Send + Sync + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    Err: Send + Sync + 'static,
{
    pool.spawn_fn(move || {
        future::lazy(move || {
            let res = request.execute(&root_node, &context);
            let code = if res.is_ok() {
                StatusCode::OK
            } else {
                StatusCode::BAD_REQUEST
            };
            let mut resp = new_response(code);
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            *resp.body_mut() = Body::from(serde_json::to_string_pretty(&res).unwrap());
            future::ok(resp)
        })
    })
}

fn gql_request_from_get(input: &str) -> Result<JuniperGraphQLRequest, GraphQLRequestError> {
    let mut query = None;
    let operation_name = None;
    let mut variables = None;
    for (key, value) in form_urlencoded::parse(input.as_bytes()).into_owned() {
        match key.as_ref() {
            "query" => {
                if query.is_some() {
                    return Err(invalid_err("query"));
                }
                query = Some(value)
            }
            "operationName" => {
                if operation_name.is_some() {
                    return Err(invalid_err("operationName"));
                }
            }
            "variables" => {
                if variables.is_some() {
                    return Err(invalid_err("variables"));
                }
                match serde_json::from_str::<InputValue>(&value)
                    .map_err(GraphQLRequestError::Variables)
                {
                    Ok(parsed_variables) => variables = Some(parsed_variables),
                    Err(e) => return Err(e),
                }
            }
            _ => continue,
        }
    }
    match query {
        Some(query) => Ok(JuniperGraphQLRequest::new(query, operation_name, variables)),
        None => Err(GraphQLRequestError::Invalid(
            "'query' parameter is missing".to_string(),
        )),
    }
}

fn invalid_err(parameter_name: &str) -> GraphQLRequestError {
    GraphQLRequestError::Invalid(format!(
        "'{}' parameter is specified multiple times",
        parameter_name
    ))
}

fn new_response(code: StatusCode) -> Response<Body> {
    let mut r = Response::new(Body::empty());
    *r.status_mut() = code;
    r
}

fn new_html_response(code: StatusCode) -> Response<Body> {
    let mut resp = new_response(code);
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    resp
}

#[derive(Deserialize)]
#[serde(untagged)]
enum GraphQLRequest {
    Single(JuniperGraphQLRequest),
    Batch(Vec<JuniperGraphQLRequest>),
}

impl GraphQLRequest {
    pub fn execute<'a, CtxT, QueryT, MutationT>(
        &'a self,
        root_node: &RootNode<QueryT, MutationT>,
        context: &CtxT,
    ) -> GraphQLResponse<'a>
    where
        QueryT: GraphQLType<Context = CtxT>,
        MutationT: GraphQLType<Context = CtxT>,
    {
        match self {
            &GraphQLRequest::Single(ref request) => {
                GraphQLResponse::Single(request.execute(root_node, context))
            }
            &GraphQLRequest::Batch(ref requests) => GraphQLResponse::Batch(
                requests
                    .iter()
                    .map(|request| request.execute(root_node, context))
                    .collect(),
            ),
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum GraphQLResponse<'a> {
    Single(JuniperGraphQLResponse<'a>),
    Batch(Vec<JuniperGraphQLResponse<'a>>),
}

impl<'a> GraphQLResponse<'a> {
    fn is_ok(&self) -> bool {
        match self {
            &GraphQLResponse::Single(ref response) => response.is_ok(),
            &GraphQLResponse::Batch(ref responses) => responses
                .iter()
                .fold(true, |ok, response| ok && response.is_ok()),
        }
    }
}

#[derive(Debug)]
enum GraphQLRequestError {
    BodyHyper(hyper::Error),
    BodyUtf8(FromUtf8Error),
    BodyJSONError(SerdeError),
    Variables(SerdeError),
    Invalid(String),
}

impl fmt::Display for GraphQLRequestError {
    fn fmt(&self, mut f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GraphQLRequestError::BodyHyper(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLRequestError::BodyUtf8(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLRequestError::BodyJSONError(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLRequestError::Variables(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLRequestError::Invalid(ref err) => fmt::Display::fmt(err, &mut f),
        }
    }
}

impl Error for GraphQLRequestError {
    fn description(&self) -> &str {
        match *self {
            GraphQLRequestError::BodyHyper(ref err) => err.description(),
            GraphQLRequestError::BodyUtf8(ref err) => err.description(),
            GraphQLRequestError::BodyJSONError(ref err) => err.description(),
            GraphQLRequestError::Variables(ref err) => err.description(),
            GraphQLRequestError::Invalid(ref err) => err,
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            GraphQLRequestError::BodyHyper(ref err) => Some(err),
            GraphQLRequestError::BodyUtf8(ref err) => Some(err),
            GraphQLRequestError::BodyJSONError(ref err) => Some(err),
            GraphQLRequestError::Variables(ref err) => Some(err),
            GraphQLRequestError::Invalid(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::{future, Future};
    use futures_cpupool::Builder;
    use hyper::service::service_fn;
    use hyper::Method;
    use hyper::{header, Body, Response, Server, StatusCode};
    use juniper::http::tests as http_tests;
    use juniper::tests::model::Database;
    use juniper::EmptyMutation;
    use juniper::RootNode;
    use reqwest;
    use reqwest::Response as ReqwestResponse;
    use std::sync::Arc;
    use std::thread;
    use std::time;
    use tokio::runtime::Runtime;

    struct TestHyperIntegration;

    impl http_tests::HTTPIntegration for TestHyperIntegration {
        fn get(&self, url: &str) -> http_tests::TestResponse {
            let url = format!("http://127.0.0.1:3001/graphql{}", url);
            make_test_response(reqwest::get(&url).expect(&format!("failed GET {}", url)))
        }

        fn post(&self, url: &str, body: &str) -> http_tests::TestResponse {
            let url = format!("http://127.0.0.1:3001/graphql{}", url);
            let client = reqwest::Client::new();
            let res = client
                .post(&url)
                .body(body.to_string())
                .send()
                .expect(&format!("failed POST {}", url));
            make_test_response(res)
        }
    }

    fn make_test_response(mut response: ReqwestResponse) -> http_tests::TestResponse {
        let status_code = response.status().as_u16() as i32;
        let content_type = String::from_utf8(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .map(|h| h.clone().as_ref().to_vec())
                .unwrap_or(vec![]),
        ).expect("Content-type header invalid UTF-8");

        let body = response.text().unwrap();

        http_tests::TestResponse {
            status_code,
            body: Some(body),
            content_type,
        }
    }

    #[test]
    fn test_hyper_integration() {
        let addr = ([127, 0, 0, 1], 3001).into();

        let pool = Builder::new().create();
        let db = Arc::new(Database::new());
        let root_node = Arc::new(RootNode::new(db.clone(), EmptyMutation::<Database>::new()));

        let new_service = move || {
            let pool = pool.clone();
            let root_node = root_node.clone();
            let ctx = db.clone();
            service_fn(move |req| {
                let pool = pool.clone();
                let root_node = root_node.clone();
                let ctx = ctx.clone();
                let matches = {
                    let path = req.uri().path();
                    match req.method() {
                        &Method::POST | &Method::GET => path == "/graphql" || path == "/graphql/",
                        _ => false,
                    }
                };
                if matches {
                    super::graphql(pool, root_node, ctx, req)
                } else {
                    let mut response = Response::new(Body::empty());
                    *response.status_mut() = StatusCode::NOT_FOUND;
                    Box::new(future::ok(response))
                }
            })
        };
        let server = Server::bind(&addr)
            .serve(new_service)
            .map_err(|e| eprintln!("server error: {}", e));

        let mut runtime = Runtime::new().unwrap();
        runtime.spawn(server);
        thread::sleep(time::Duration::from_millis(10)); // wait 10ms for server to bind

        let integration = TestHyperIntegration;
        http_tests::run_http_test_suite(&integration);

        runtime.shutdown_now().wait().unwrap();
    }
}
