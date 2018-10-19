#[macro_use]
extern crate futures;
extern crate hyper;
extern crate juniper;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
extern crate reqwest;
extern crate serde_json;
extern crate tokio;
extern crate tokio_threadpool;
extern crate url;

use futures::future::Either;
use hyper::header::HeaderValue;
use hyper::rt::Stream;
use hyper::{header, Body, Method, Request, Response, StatusCode};
use juniper::http::GraphQLRequest as JuniperGraphQLRequest;
use juniper::serde::Deserialize;
use juniper::{DefaultScalarValue, GraphQLType, InputValue, RootNode, ScalarRefValue, ScalarValue};
use serde_json::error::Error as SerdeError;
use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;
use std::sync::Arc;
use tokio::prelude::*;
use url::form_urlencoded;

pub fn graphql<CtxT, QueryT, MutationT, S>(
    root_node: Arc<RootNode<'static, QueryT, MutationT, S>>,
    context: Arc<CtxT>,
    request: Request<Body>,
) -> impl Future<Item = Response<Body>, Error = hyper::Error>
where
    S: ScalarValue + Send + Sync + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
    CtxT: Send + Sync + 'static,
    QueryT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    MutationT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT::TypeInfo: Send + Sync,
{
    match request.method() {
        &Method::GET => Either::A(Either::A(
            future::done(
                request
                    .uri()
                    .query()
                    .map(|q| gql_request_from_get(q).map(GraphQLRequest::Single))
                    .unwrap_or(Err(GraphQLRequestError::Invalid(
                        "'query' parameter is missing".to_string(),
                    ))),
            ).and_then(move |gql_req| {
                execute_request(root_node, context, gql_req).map_err(|_| {
                    unreachable!("thread pool has shut down?!");
                })
            }).or_else(|err| future::ok(render_error(err))),
        )),
        &Method::POST => Either::A(Either::B(
            request
                .into_body()
                .concat2()
                .or_else(|err| future::done(Err(GraphQLRequestError::BodyHyper(err))))
                .and_then(move |chunk| {
                    future::done({
                        String::from_utf8(chunk.iter().cloned().collect::<Vec<u8>>())
                            .map_err(GraphQLRequestError::BodyUtf8)
                            .and_then(|input| {
                                serde_json::from_str::<GraphQLRequest<S>>(&input)
                                    .map_err(GraphQLRequestError::BodyJSONError)
                            })
                    })
                }).and_then(move |gql_req| {
                    execute_request(root_node, context, gql_req).map_err(|_| {
                        unreachable!("thread pool has shut down?!");
                    })
                }).or_else(|err| future::ok(render_error(err))),
        )),
        _ => return Either::B(future::ok(new_response(StatusCode::METHOD_NOT_ALLOWED))),
    }
}

pub fn graphiql(
    graphql_endpoint: &str,
) -> impl Future<Item = Response<Body>, Error = hyper::Error> {
    let mut resp = new_html_response(StatusCode::OK);
    // XXX: is the call to graphiql_source blocking?
    *resp.body_mut() = Body::from(juniper::graphiql::graphiql_source(graphql_endpoint));
    future::ok(resp)
}

fn render_error(err: GraphQLRequestError) -> Response<Body> {
    let message = format!("{}", err);
    let mut resp = new_response(StatusCode::BAD_REQUEST);
    *resp.body_mut() = Body::from(message);
    resp
}

fn execute_request<CtxT, QueryT, MutationT, S>(
    root_node: Arc<RootNode<'static, QueryT, MutationT, S>>,
    context: Arc<CtxT>,
    request: GraphQLRequest<S>,
) -> impl Future<Item = Response<Body>, Error = tokio_threadpool::BlockingError>
where
    S: ScalarValue + Send + Sync + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
    CtxT: Send + Sync + 'static,
    QueryT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    MutationT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT::TypeInfo: Send + Sync,
{
    request.execute(root_node, context).map(|(is_ok, body)| {
        let code = if is_ok {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        let mut resp = new_response(code);
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        *resp.body_mut() = body;
        resp
    })
}

fn gql_request_from_get<S>(input: &str) -> Result<JuniperGraphQLRequest<S>, GraphQLRequestError>
where
    S: ScalarValue,
{
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
                match serde_json::from_str::<InputValue<S>>(&value)
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
#[serde(bound = "InputValue<S>: Deserialize<'de>")]
enum GraphQLRequest<S = DefaultScalarValue>
where
    S: ScalarValue,
{
    Single(JuniperGraphQLRequest<S>),
    Batch(Vec<JuniperGraphQLRequest<S>>),
}

impl<S> GraphQLRequest<S>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    fn execute<'a, CtxT: 'a, QueryT, MutationT>(
        self,
        root_node: Arc<RootNode<'a, QueryT, MutationT, S>>,
        context: Arc<CtxT>,
    ) -> impl Future<Item = (bool, hyper::Body), Error = tokio_threadpool::BlockingError> + 'a
    where
        S: 'a,
        QueryT: GraphQLType<S, Context = CtxT> + 'a,
        MutationT: GraphQLType<S, Context = CtxT> + 'a,
    {
        match self {
            GraphQLRequest::Single(request) => Either::A(future::poll_fn(move || {
                let res = try_ready!(tokio_threadpool::blocking(
                    || request.execute(&root_node, &context)
                ));
                let is_ok = res.is_ok();
                let body = Body::from(serde_json::to_string_pretty(&res).unwrap());
                Ok(Async::Ready((is_ok, body)))
            })),
            GraphQLRequest::Batch(requests) => {
                Either::B(
                    future::join_all(requests.into_iter().map(move |request| {
                        // TODO: these clones are sad
                        let root_node = root_node.clone();
                        let context = context.clone();
                        future::poll_fn(move || {
                            let res = try_ready!(tokio_threadpool::blocking(
                                || request.execute(&root_node, &context)
                            ));
                            let is_ok = res.is_ok();
                            let body = serde_json::to_string_pretty(&res).unwrap();
                            Ok(Async::Ready((is_ok, body)))
                        })
                    })).map(|results| {
                        let is_ok = results.iter().all(|&(is_ok, _)| is_ok);
                        // concatenate json bodies as array
                        // TODO: maybe use Body chunks instead?
                        let bodies: Vec<_> = results.into_iter().map(|(_, body)| body).collect();
                        let body = hyper::Body::from(format!("[{}]", bodies.join(",")));
                        (is_ok, body)
                    }),
                )
            }
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
    use futures::{future, future::Either, Future};
    use hyper::service::service_fn;
    use hyper::Method;
    use hyper::{Body, Response, Server, StatusCode};
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
        let body = response.text().unwrap();
        let content_type_header = response.headers().get(reqwest::header::CONTENT_TYPE);
        let content_type = if let Some(ct) = content_type_header {
            format!("{}", ct.to_str().unwrap())
        } else {
            String::default()
        };

        http_tests::TestResponse {
            status_code,
            body: Some(body),
            content_type,
        }
    }

    #[test]
    fn test_hyper_integration() {
        let addr = ([127, 0, 0, 1], 3001).into();

        let db = Arc::new(Database::new());
        let root_node = Arc::new(RootNode::new(db.clone(), EmptyMutation::<Database>::new()));

        let new_service = move || {
            let root_node = root_node.clone();
            let ctx = db.clone();
            service_fn(move |req| {
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
                    Either::A(super::graphql(root_node, ctx, req))
                } else {
                    let mut response = Response::new(Body::empty());
                    *response.status_mut() = StatusCode::NOT_FOUND;
                    Either::B(future::ok(response))
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
