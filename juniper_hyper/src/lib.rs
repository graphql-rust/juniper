#![doc(html_root_url = "https://docs.rs/juniper_hyper/0.2.0")]

#[cfg(test)]
extern crate reqwest;

use hyper::{
    header::{self, HeaderValue},
    Body, Method, Request, Response, StatusCode,
};
use juniper::{
    http::{GraphQLBatchRequest, GraphQLRequest as JuniperGraphQLRequest},
    GraphQLSubscriptionType, GraphQLType, GraphQLTypeAsync, InputValue, RootNode, ScalarValue,
};
use serde_json::error::Error as SerdeError;
use std::{error::Error, fmt, string::FromUtf8Error, sync::Arc};
use url::form_urlencoded;

pub async fn graphql<CtxT, QueryT, MutationT, SubscriptionT, S>(
    root_node: Arc<RootNode<'static, QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    request: Request<Body>,
) -> Result<Response<Body>, hyper::Error>
where
    S: ScalarValue + Send + Sync + 'static,
    CtxT: Send + Sync + 'static,
    QueryT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    MutationT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    SubscriptionT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT::TypeInfo: Send + Sync,
{
    match *request.method() {
        Method::GET => {
            let gql_req = parse_get_req(request);

            match gql_req {
                Ok(gql_req) => Ok(execute_request(root_node, context, gql_req).await),
                Err(err) => Ok(render_error(err)),
            }
        }
        Method::POST => {
            let gql_req = parse_post_req(request.into_body()).await;

            match gql_req {
                Ok(gql_req) => Ok(execute_request(root_node, context, gql_req).await),
                Err(err) => Ok(render_error(err)),
            }
        }
        _ => Ok(new_response(StatusCode::METHOD_NOT_ALLOWED)),
    }
}

pub async fn graphql_async<CtxT, QueryT, MutationT, SubscriptionT, S>(
    root_node: Arc<RootNode<'static, QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    request: Request<Body>,
) -> Result<Response<Body>, hyper::Error>
where
    S: ScalarValue + Send + Sync + 'static,
    CtxT: Send + Sync + 'static,
    QueryT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync + 'static,
    MutationT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync + 'static,
    SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync,
    QueryT::TypeInfo: Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT::TypeInfo: Send + Sync,
{
    match *request.method() {
        Method::GET => {
            let gql_req = parse_get_req(request);

            match gql_req {
                Ok(gql_req) => Ok(execute_request_async(root_node, context, gql_req).await),
                Err(err) => Ok(render_error(err)),
            }
        }
        Method::POST => {
            let gql_req = parse_post_req(request.into_body()).await;

            match gql_req {
                Ok(gql_req) => Ok(execute_request_async(root_node, context, gql_req).await),
                Err(err) => Ok(render_error(err)),
            }
        }
        _ => Ok(new_response(StatusCode::METHOD_NOT_ALLOWED)),
    }
}

fn parse_get_req<S: ScalarValue>(
    req: Request<Body>,
) -> Result<GraphQLBatchRequest<S>, GraphQLRequestError> {
    req.uri()
        .query()
        .map(|q| gql_request_from_get(q).map(GraphQLBatchRequest::Single))
        .unwrap_or_else(|| {
            Err(GraphQLRequestError::Invalid(
                "'query' parameter is missing".to_string(),
            ))
        })
}

async fn parse_post_req<S: ScalarValue>(
    body: Body,
) -> Result<GraphQLBatchRequest<S>, GraphQLRequestError> {
    let chunk = hyper::body::to_bytes(body)
        .await
        .map_err(GraphQLRequestError::BodyHyper)?;

    let input = String::from_utf8(chunk.iter().cloned().collect())
        .map_err(GraphQLRequestError::BodyUtf8)?;

    serde_json::from_str::<GraphQLBatchRequest<S>>(&input)
        .map_err(GraphQLRequestError::BodyJSONError)
}

pub async fn graphiql(
    graphql_endpoint: &str,
    subscriptions_endpoint: Option<&str>,
) -> Result<Response<Body>, hyper::Error> {
    let mut resp = new_html_response(StatusCode::OK);
    // XXX: is the call to graphiql_source blocking?
    *resp.body_mut() = Body::from(juniper::http::graphiql::graphiql_source(
        graphql_endpoint,
        subscriptions_endpoint,
    ));
    Ok(resp)
}

pub async fn playground(
    graphql_endpoint: &str,
    subscriptions_endpoint: Option<&str>,
) -> Result<Response<Body>, hyper::Error> {
    let mut resp = new_html_response(StatusCode::OK);
    *resp.body_mut() = Body::from(juniper::http::playground::playground_source(
        graphql_endpoint,
        subscriptions_endpoint,
    ));
    Ok(resp)
}

fn render_error(err: GraphQLRequestError) -> Response<Body> {
    let message = format!("{}", err);
    let mut resp = new_response(StatusCode::BAD_REQUEST);
    *resp.body_mut() = Body::from(message);
    resp
}

async fn execute_request<CtxT, QueryT, MutationT, SubscriptionT, S>(
    root_node: Arc<RootNode<'static, QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    request: GraphQLBatchRequest<S>,
) -> Response<Body>
where
    S: ScalarValue + Send + Sync + 'static,
    CtxT: Send + Sync + 'static,
    QueryT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    MutationT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    SubscriptionT: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT::TypeInfo: Send + Sync,
{
    let res = request.execute_sync(&*root_node, &context);
    let body = Body::from(serde_json::to_string_pretty(&res).unwrap());
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

async fn execute_request_async<CtxT, QueryT, MutationT, SubscriptionT, S>(
    root_node: Arc<RootNode<'static, QueryT, MutationT, SubscriptionT, S>>,
    context: Arc<CtxT>,
    request: GraphQLBatchRequest<S>,
) -> Response<Body>
where
    S: ScalarValue + Send + Sync + 'static,
    CtxT: Send + Sync + 'static,
    QueryT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync + 'static,
    MutationT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync + 'static,
    SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync,
    QueryT::TypeInfo: Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT::TypeInfo: Send + Sync,
{
    let res = request.execute(&*root_node, &context).await;
    let body = Body::from(serde_json::to_string_pretty(&res).unwrap());
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
    fn source(&self) -> Option<&(dyn Error + 'static)> {
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
    use hyper::{
        service::{make_service_fn, service_fn},
        Body, Method, Response, Server, StatusCode,
    };
    use juniper::{
        http::tests as http_tests,
        tests::{model::Database, schema::Query},
        EmptyMutation, EmptySubscription, RootNode,
    };
    use reqwest::{self, Response as ReqwestResponse};
    use std::{net::SocketAddr, sync::Arc, thread, time::Duration};

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

    #[tokio::test]
    async fn test_hyper_integration() {
        let addr: SocketAddr = ([127, 0, 0, 1], 3001).into();

        let db = Arc::new(Database::new());
        let root_node = Arc::new(RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        ));

        let new_service = make_service_fn(move |_| {
            let root_node = root_node.clone();
            let ctx = db.clone();

            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let root_node = root_node.clone();
                    let ctx = ctx.clone();
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
                        if matches {
                            super::graphql(root_node, ctx, req).await
                        } else {
                            let mut response = Response::new(Body::empty());
                            *response.status_mut() = StatusCode::NOT_FOUND;
                            Ok(response)
                        }
                    }
                }))
            }
        });

        let (shutdown_fut, shutdown) = futures::future::abortable(async {
            tokio::time::delay_for(Duration::from_secs(60)).await;
        });

        let server = Server::bind(&addr)
            .serve(new_service)
            .with_graceful_shutdown(async {
                shutdown_fut.await.unwrap_err();
            });

        tokio::task::spawn_blocking(move || {
            thread::sleep(Duration::from_millis(10)); // wait 10ms for server to bind
            let integration = TestHyperIntegration;
            http_tests::run_http_test_suite(&integration);
            shutdown.abort();
        });

        if let Err(e) = server.await {
            eprintln!("server error: {}", e);
        }
    }
}
