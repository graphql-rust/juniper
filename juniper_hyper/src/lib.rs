#![doc = include_str!("../README.md")]

use std::{error::Error, fmt, string::FromUtf8Error, sync::Arc};

use http_body_util::BodyExt as _;
use hyper::{
    body,
    header::{self, HeaderValue},
    Method, Request, Response, StatusCode,
};
use juniper::{
    http::{GraphQLBatchRequest, GraphQLRequest as JuniperGraphQLRequest, GraphQLRequest},
    GraphQLSubscriptionType, GraphQLType, GraphQLTypeAsync, InputValue, RootNode, ScalarValue,
};
use serde_json::error::Error as SerdeError;
use url::form_urlencoded;

pub async fn graphql_sync<CtxT, QueryT, MutationT, SubscriptionT, S>(
    root_node: Arc<RootNode<QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    req: Request<body::Incoming>,
) -> Response<String>
where
    QueryT: GraphQLType<S, Context = CtxT>,
    QueryT::TypeInfo: Sync,
    MutationT: GraphQLType<S, Context = CtxT>,
    MutationT::TypeInfo: Sync,
    SubscriptionT: GraphQLType<S, Context = CtxT>,
    SubscriptionT::TypeInfo: Sync,
    CtxT: Sync,
    S: ScalarValue + Send + Sync,
{
    match parse_req(req).await {
        Ok(req) => execute_request_sync(root_node, context, req).await,
        Err(resp) => resp,
    }
}

pub async fn graphql<CtxT, QueryT, MutationT, SubscriptionT, S>(
    root_node: Arc<RootNode<QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    req: Request<body::Incoming>,
) -> Response<String>
where
    QueryT: GraphQLTypeAsync<S, Context = CtxT>,
    QueryT::TypeInfo: Sync,
    MutationT: GraphQLTypeAsync<S, Context = CtxT>,
    MutationT::TypeInfo: Sync,
    SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT>,
    SubscriptionT::TypeInfo: Sync,
    CtxT: Sync,
    S: ScalarValue + Send + Sync,
{
    match parse_req(req).await {
        Ok(req) => execute_request(root_node, context, req).await,
        Err(resp) => resp,
    }
}

async fn parse_req<S: ScalarValue>(
    req: Request<body::Incoming>,
) -> Result<GraphQLBatchRequest<S>, Response<String>> {
    match *req.method() {
        Method::GET => parse_get_req(req),
        Method::POST => {
            let content_type = req
                .headers()
                .get(header::CONTENT_TYPE)
                .map(HeaderValue::to_str);
            match content_type {
                Some(Ok("application/json")) => parse_post_json_req(req.into_body()).await,
                Some(Ok("application/graphql")) => parse_post_graphql_req(req.into_body()).await,
                _ => return Err(new_response(StatusCode::BAD_REQUEST)),
            }
        }
        _ => return Err(new_response(StatusCode::METHOD_NOT_ALLOWED)),
    }
    .map_err(render_error)
}

fn parse_get_req<S: ScalarValue>(
    req: Request<body::Incoming>,
) -> Result<GraphQLBatchRequest<S>, GraphQLRequestError> {
    req.uri()
        .query()
        .map(|q| gql_request_from_get(q).map(GraphQLBatchRequest::Single))
        .unwrap_or_else(|| {
            Err(GraphQLRequestError::Invalid(
                "'query' parameter is missing".into(),
            ))
        })
}

async fn parse_post_json_req<S: ScalarValue>(
    body: body::Incoming,
) -> Result<GraphQLBatchRequest<S>, GraphQLRequestError> {
    let chunk = body
        .collect()
        .await
        .map_err(GraphQLRequestError::BodyHyper)?;

    let input = String::from_utf8(chunk.to_bytes().iter().cloned().collect())
        .map_err(GraphQLRequestError::BodyUtf8)?;

    serde_json::from_str::<GraphQLBatchRequest<S>>(&input)
        .map_err(GraphQLRequestError::BodyJSONError)
}

async fn parse_post_graphql_req<S: ScalarValue>(
    body: body::Incoming,
) -> Result<GraphQLBatchRequest<S>, GraphQLRequestError> {
    let chunk = body
        .collect()
        .await
        .map_err(GraphQLRequestError::BodyHyper)?;

    let query = String::from_utf8(chunk.to_bytes().iter().cloned().collect())
        .map_err(GraphQLRequestError::BodyUtf8)?;

    Ok(GraphQLBatchRequest::Single(GraphQLRequest::new(
        query, None, None,
    )))
}

pub async fn graphiql(
    graphql_endpoint: &str,
    subscriptions_endpoint: Option<&str>,
) -> Response<String> {
    let mut resp = new_html_response(StatusCode::OK);
    // XXX: is the call to graphiql_source blocking?
    *resp.body_mut() =
        juniper::http::graphiql::graphiql_source(graphql_endpoint, subscriptions_endpoint);
    resp
}

pub async fn playground(
    graphql_endpoint: &str,
    subscriptions_endpoint: Option<&str>,
) -> Response<String> {
    let mut resp = new_html_response(StatusCode::OK);
    *resp.body_mut() =
        juniper::http::playground::playground_source(graphql_endpoint, subscriptions_endpoint);
    resp
}

fn render_error(err: GraphQLRequestError) -> Response<String> {
    let mut resp = new_response(StatusCode::BAD_REQUEST);
    *resp.body_mut() = err.to_string();
    resp
}

async fn execute_request_sync<CtxT, QueryT, MutationT, SubscriptionT, S>(
    root_node: Arc<RootNode<QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    request: GraphQLBatchRequest<S>,
) -> Response<String>
where
    QueryT: GraphQLType<S, Context = CtxT>,
    QueryT::TypeInfo: Sync,
    MutationT: GraphQLType<S, Context = CtxT>,
    MutationT::TypeInfo: Sync,
    SubscriptionT: GraphQLType<S, Context = CtxT>,
    SubscriptionT::TypeInfo: Sync,
    CtxT: Sync,
    S: ScalarValue + Send + Sync,
{
    let res = request.execute_sync(&*root_node, &context);
    let body = serde_json::to_string_pretty(&res).unwrap();
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
    *resp.body_mut() = body;
    resp
}

async fn execute_request<CtxT, QueryT, MutationT, SubscriptionT, S>(
    root_node: Arc<RootNode<QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    request: GraphQLBatchRequest<S>,
) -> Response<String>
where
    QueryT: GraphQLTypeAsync<S, Context = CtxT>,
    QueryT::TypeInfo: Sync,
    MutationT: GraphQLTypeAsync<S, Context = CtxT>,
    MutationT::TypeInfo: Sync,
    SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT>,
    SubscriptionT::TypeInfo: Sync,
    CtxT: Sync,
    S: ScalarValue + Send + Sync,
{
    let res = request.execute(&*root_node, &context).await;
    let body = serde_json::to_string_pretty(&res).unwrap();
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
    *resp.body_mut() = body;
    resp
}

fn gql_request_from_get<S>(input: &str) -> Result<JuniperGraphQLRequest<S>, GraphQLRequestError>
where
    S: ScalarValue,
{
    let mut query = None;
    let mut operation_name = None;
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
                operation_name = Some(value)
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
            "'query' parameter is missing".into(),
        )),
    }
}

fn invalid_err(parameter_name: &str) -> GraphQLRequestError {
    GraphQLRequestError::Invalid(format!(
        "`{parameter_name}` parameter is specified multiple times",
    ))
}

fn new_response(code: StatusCode) -> Response<String> {
    let mut r = Response::new(String::new());
    *r.status_mut() = code;
    r
}

fn new_html_response(code: StatusCode) -> Response<String> {
    let mut resp = new_response(code);
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    resp
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GraphQLRequestError::BodyHyper(err) => fmt::Display::fmt(err, f),
            GraphQLRequestError::BodyUtf8(err) => fmt::Display::fmt(err, f),
            GraphQLRequestError::BodyJSONError(err) => fmt::Display::fmt(err, f),
            GraphQLRequestError::Variables(err) => fmt::Display::fmt(err, f),
            GraphQLRequestError::Invalid(err) => fmt::Display::fmt(err, f),
        }
    }
}

impl Error for GraphQLRequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            GraphQLRequestError::BodyHyper(err) => Some(err),
            GraphQLRequestError::BodyUtf8(err) => Some(err),
            GraphQLRequestError::BodyJSONError(err) => Some(err),
            GraphQLRequestError::Variables(err) => Some(err),
            GraphQLRequestError::Invalid(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible, error::Error, net::SocketAddr, panic, sync::Arc, time::Duration,
    };

    use hyper::{server::conn::http1, service::service_fn, Method, Response, StatusCode};
    use hyper_util::rt::TokioIo;
    use juniper::{
        http::tests as http_tests,
        tests::fixtures::starwars::schema::{Database, Query},
        EmptyMutation, EmptySubscription, RootNode,
    };
    use reqwest::blocking::Response as ReqwestResponse;
    use tokio::{net::TcpListener, task, time::sleep};

    struct TestHyperIntegration {
        port: u16,
    }

    impl http_tests::HttpIntegration for TestHyperIntegration {
        fn get(&self, url: &str) -> http_tests::TestResponse {
            let url = format!("http://127.0.0.1:{}/graphql{url}", self.port);
            make_test_response(
                reqwest::blocking::get(&url).unwrap_or_else(|_| panic!("failed GET {url}")),
            )
        }

        fn post_json(&self, url: &str, body: &str) -> http_tests::TestResponse {
            let url = format!("http://127.0.0.1:{}/graphql{url}", self.port);
            let client = reqwest::blocking::Client::new();
            let res = client
                .post(&url)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body.to_owned())
                .send()
                .unwrap_or_else(|_| panic!("failed POST {url}"));
            make_test_response(res)
        }

        fn post_graphql(&self, url: &str, body: &str) -> http_tests::TestResponse {
            let url = format!("http://127.0.0.1:{}/graphql{url}", self.port);
            let client = reqwest::blocking::Client::new();
            let res = client
                .post(&url)
                .header(reqwest::header::CONTENT_TYPE, "application/graphql")
                .body(body.to_owned())
                .send()
                .unwrap_or_else(|_| panic!("failed POST {url}"));
            make_test_response(res)
        }
    }

    fn make_test_response(response: ReqwestResponse) -> http_tests::TestResponse {
        let status_code = response.status().as_u16() as i32;
        let content_type_header = response.headers().get(reqwest::header::CONTENT_TYPE);
        let content_type = content_type_header
            .map(|ct| ct.to_str().unwrap().into())
            .unwrap_or_default();
        let body = response.text().unwrap();

        http_tests::TestResponse {
            status_code,
            body: Some(body),
            content_type,
        }
    }

    async fn run_hyper_integration(is_sync: bool) {
        let port = if is_sync { 3002 } else { 3001 };
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let db = Arc::new(Database::new());
        let root_node = Arc::new(RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        ));

        let server: task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> =
            task::spawn(async move {
                let listener = TcpListener::bind(addr).await?;

                loop {
                    let (stream, _) = listener.accept().await?;
                    let io = TokioIo::new(stream);

                    let root_node = root_node.clone();
                    let db = db.clone();

                    _ = task::spawn(async move {
                        let root_node = root_node.clone();
                        let db = db.clone();

                        if let Err(e) = http1::Builder::new()
                            .serve_connection(
                                io,
                                service_fn(move |req| {
                                    let root_node = root_node.clone();
                                    let db = db.clone();
                                    let matches = {
                                        let path = req.uri().path();
                                        match req.method() {
                                            &Method::POST | &Method::GET => {
                                                path == "/graphql" || path == "/graphql/"
                                            }
                                            _ => false,
                                        }
                                    };
                                    async move {
                                        Ok::<_, Infallible>(if matches {
                                            if is_sync {
                                                super::graphql_sync(root_node, db, req).await
                                            } else {
                                                super::graphql(root_node, db, req).await
                                            }
                                        } else {
                                            let mut resp = Response::new(String::new());
                                            *resp.status_mut() = StatusCode::NOT_FOUND;
                                            resp
                                        })
                                    }
                                }),
                            )
                            .await
                        {
                            eprintln!("server error: {e}");
                        }
                    });
                }
            });

        sleep(Duration::from_secs(10)).await; // wait 10ms for `server` to bind

        match task::spawn_blocking(move || {
            let integration = TestHyperIntegration { port };
            http_tests::run_http_test_suite(&integration);
        })
        .await
        {
            Err(f) if f.is_panic() => panic::resume_unwind(f.into_panic()),
            Ok(()) | Err(_) => {}
        }

        server.abort();
        if let Ok(Err(e)) = server.await {
            panic!("server failed: {e}");
        }
    }

    #[tokio::test]
    async fn test_hyper_integration() {
        run_hyper_integration(false).await
    }

    #[tokio::test]
    async fn test_sync_hyper_integration() {
        run_hyper_integration(true).await
    }
}
