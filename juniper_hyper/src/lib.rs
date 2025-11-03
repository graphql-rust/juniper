#![cfg_attr(any(doc, test), doc = include_str!("../README.md"))]
#![cfg_attr(not(any(doc, test)), doc = env!("CARGO_PKG_NAME"))]
#![cfg_attr(test, expect(unused_crate_dependencies, reason = "examples"))]

use std::{string::FromUtf8Error, sync::Arc};

use derive_more::with_trait::{Debug, Display, Error};
use http_body_util::BodyExt as _;
use hyper::{
    Method, Request, Response, StatusCode,
    body::Body,
    header::{self, HeaderValue},
};
use juniper::{
    GraphQLSubscriptionType, GraphQLType, GraphQLTypeAsync, InputValue, RootNode, ScalarValue,
    http::{GraphQLBatchRequest, GraphQLRequest as JuniperGraphQLRequest, GraphQLRequest},
};
use serde_json::error::Error as SerdeError;
use url::form_urlencoded;

/// Executes synchronously  the provided GraphQL [`Request`] against the provided `schema` in the
/// provided `context`, returning the encoded [`Response`].
pub async fn graphql_sync<CtxT, QueryT, MutationT, SubscriptionT, S, B>(
    schema: Arc<RootNode<QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    req: Request<B>,
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
    B: Body<Error: Display>,
{
    match parse_req(req).await {
        Ok(req) => execute_request_sync(schema, context, req).await,
        Err(resp) => resp,
    }
}

/// Executes the provided GraphQL [`Request`] against the provided `schema` in the provided
/// `context`, returning the encoded [`Response`].
pub async fn graphql<CtxT, QueryT, MutationT, SubscriptionT, S, B>(
    schema: Arc<RootNode<QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    req: Request<B>,
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
    B: Body<Error: Display>,
{
    match parse_req(req).await {
        Ok(req) => execute_request(schema, context, req).await,
        Err(resp) => resp,
    }
}

async fn parse_req<S, B>(req: Request<B>) -> Result<GraphQLBatchRequest<S>, Response<String>>
where
    S: ScalarValue,
    B: Body<Error: Display>,
{
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

fn parse_get_req<S, B>(req: Request<B>) -> Result<GraphQLBatchRequest<S>, GraphQLRequestError<B>>
where
    S: ScalarValue,
    B: Body,
{
    req.uri()
        .query()
        .map(|q| gql_request_from_get(q).map(GraphQLBatchRequest::Single))
        .unwrap_or_else(|| {
            Err(GraphQLRequestError::Invalid(
                "'query' parameter is missing".into(),
            ))
        })
}

async fn parse_post_json_req<S, B>(
    body: B,
) -> Result<GraphQLBatchRequest<S>, GraphQLRequestError<B>>
where
    S: ScalarValue,
    B: Body,
{
    let chunk = body
        .collect()
        .await
        .map_err(GraphQLRequestError::BodyHyper)?;

    let input =
        String::from_utf8(chunk.to_bytes().into()).map_err(GraphQLRequestError::BodyUtf8)?;

    serde_json::from_str::<GraphQLBatchRequest<S>>(&input)
        .map_err(GraphQLRequestError::BodyJSONError)
}

async fn parse_post_graphql_req<S, B>(
    body: B,
) -> Result<GraphQLBatchRequest<S>, GraphQLRequestError<B>>
where
    S: ScalarValue,
    B: Body,
{
    let chunk = body
        .collect()
        .await
        .map_err(GraphQLRequestError::BodyHyper)?;

    let query =
        String::from_utf8(chunk.to_bytes().into()).map_err(GraphQLRequestError::BodyUtf8)?;

    Ok(GraphQLBatchRequest::Single(GraphQLRequest {
        query,
        operation_name: None,
        variables: None,
        extensions: None,
    }))
}

/// Generates a [`Response`] page containing [GraphiQL].
///
/// This does not handle routing, so you can mount it on any endpoint.
///
/// [GraphiQL]: https://github.com/graphql/graphiql
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

/// Generates a [`Response`] page containing [GraphQL Playground].
///
/// This does not handle routing, so you can mount it on any endpoint.
///
/// [GraphQL Playground]: https://github.com/prisma/graphql-playground
pub async fn playground(
    graphql_endpoint: &str,
    subscriptions_endpoint: Option<&str>,
) -> Response<String> {
    let mut resp = new_html_response(StatusCode::OK);
    *resp.body_mut() =
        juniper::http::playground::playground_source(graphql_endpoint, subscriptions_endpoint);
    resp
}

fn render_error<B>(err: GraphQLRequestError<B>) -> Response<String>
where
    B: Body<Error: Display>,
{
    let mut resp = new_response(StatusCode::BAD_REQUEST);
    *resp.body_mut() = err.to_string();
    resp
}

async fn execute_request_sync<CtxT, QueryT, MutationT, SubscriptionT, S>(
    schema: Arc<RootNode<QueryT, MutationT, SubscriptionT, S>>,
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
    let res = request.execute_sync(&*schema, &context);
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
    schema: Arc<RootNode<QueryT, MutationT, SubscriptionT, S>>,
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
    let res = request.execute(&*schema, &context).await;
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

fn gql_request_from_get<S, B>(
    input: &str,
) -> Result<JuniperGraphQLRequest<S>, GraphQLRequestError<B>>
where
    S: ScalarValue,
    B: Body,
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
        Some(query) => Ok(JuniperGraphQLRequest {
            query,
            operation_name,
            variables,
            extensions: None,
        }),
        None => Err(GraphQLRequestError::Invalid(
            "'query' parameter is missing".into(),
        )),
    }
}

fn invalid_err<B: Body>(parameter_name: &str) -> GraphQLRequestError<B> {
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

// TODO: Use `#[debug(forward)]` once `derive_more::Debug` is capable of it.
#[derive(Debug, Display, Error)]
enum GraphQLRequestError<B: Body> {
    #[debug("{_0:?}")]
    BodyHyper(B::Error),
    #[debug("{_0:?}")]
    BodyUtf8(FromUtf8Error),
    #[debug("{_0:?}")]
    BodyJSONError(SerdeError),
    #[debug("{_0:?}")]
    Variables(SerdeError),
    #[debug("{_0:?}")]
    Invalid(#[error(not(source))] String),
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible, error::Error, net::SocketAddr, panic, sync::Arc, time::Duration,
    };

    use http_body_util::BodyExt as _;
    use hyper::{
        Method, Request, Response, StatusCode, body::Incoming, server::conn::http1,
        service::service_fn,
    };
    use hyper_util::rt::TokioIo;
    use juniper::{
        EmptyMutation, EmptySubscription, RootNode,
        http::tests as http_tests,
        tests::fixtures::starwars::schema::{Database, Query},
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

    async fn run_hyper_integration(port: u16, is_sync: bool, is_custom_type: bool) {
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
                                service_fn(move |req: Request<Incoming>| {
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
                                            if is_custom_type {
                                                let (parts, mut body) = req.into_parts();
                                                let body = {
                                                    let mut buf = String::new();
                                                    if let Some(Ok(frame)) = body.frame().await {
                                                        if let Ok(bytes) = frame.into_data() {
                                                            buf = String::from_utf8_lossy(&bytes)
                                                                .to_string();
                                                        }
                                                    }
                                                    buf
                                                };
                                                let req = Request::from_parts(parts, body);
                                                if is_sync {
                                                    super::graphql_sync(root_node, db, req).await
                                                } else {
                                                    super::graphql(root_node, db, req).await
                                                }
                                            } else {
                                                if is_sync {
                                                    super::graphql_sync(root_node, db, req).await
                                                } else {
                                                    super::graphql(root_node, db, req).await
                                                }
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
        run_hyper_integration(3000, false, false).await
    }

    #[tokio::test]
    async fn test_sync_hyper_integration() {
        run_hyper_integration(3001, true, false).await
    }

    #[tokio::test]
    async fn test_custom_request_hyper_integration() {
        run_hyper_integration(3002, false, false).await
    }

    #[tokio::test]
    async fn test_custom_request_sync_hyper_integration() {
        run_hyper_integration(3003, true, true).await
    }
}
