#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(warnings)]

use actix_web::{
    error::JsonPayloadError, http::Method, web, Error, FromRequest, HttpMessage, HttpRequest,
    HttpResponse,
};
use juniper::{
    http::{
        graphiql::graphiql_source, playground::playground_source, GraphQLBatchRequest,
        GraphQLRequest,
    },
    ScalarValue,
};
use serde::Deserialize;

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
struct GetGraphQLRequest {
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    variables: Option<String>,
}

impl<S> From<GetGraphQLRequest> for GraphQLRequest<S>
where
    S: ScalarValue,
{
    fn from(get_req: GetGraphQLRequest) -> Self {
        let GetGraphQLRequest {
            query,
            operation_name,
            variables,
        } = get_req;
        let variables = variables.map(|s| serde_json::from_str(&s).unwrap());
        Self::new(query, operation_name, variables)
    }
}

/// Actix Web GraphQL Handler for GET and POST requests
pub async fn graphql_handler<Query, Mutation, Subscription, CtxT, S>(
    schema: &juniper::RootNode<Query, Mutation, Subscription, S>,
    context: &CtxT,
    req: HttpRequest,
    payload: actix_web::web::Payload,
) -> Result<HttpResponse, Error>
where
    Query: juniper::GraphQLTypeAsync<S, Context = CtxT>,
    Query::TypeInfo: Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = CtxT>,
    Mutation::TypeInfo: Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = CtxT>,
    Subscription::TypeInfo: Sync,
    CtxT: Sync,
    S: ScalarValue + Send + Sync,
{
    match *req.method() {
        Method::POST => post_graphql_handler(schema, context, req, payload).await,
        Method::GET => get_graphql_handler(schema, context, req).await,
        _ => Err(actix_web::error::UrlGenerationError::ResourceNotFound.into()),
    }
}
/// Actix GraphQL Handler for GET requests
pub async fn get_graphql_handler<Query, Mutation, Subscription, CtxT, S>(
    schema: &juniper::RootNode<Query, Mutation, Subscription, S>,
    context: &CtxT,
    req: HttpRequest,
) -> Result<HttpResponse, Error>
where
    Query: juniper::GraphQLTypeAsync<S, Context = CtxT>,
    Query::TypeInfo: Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = CtxT>,
    Mutation::TypeInfo: Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = CtxT>,
    Subscription::TypeInfo: Sync,
    CtxT: Sync,
    S: ScalarValue + Send + Sync,
{
    let get_req = web::Query::<GetGraphQLRequest>::from_query(req.query_string())?;
    let req = GraphQLRequest::from(get_req.into_inner());
    let gql_response = req.execute(schema, context).await;
    let body_response = serde_json::to_string(&gql_response)?;
    let mut response = match gql_response.is_ok() {
        true => HttpResponse::Ok(),
        false => HttpResponse::BadRequest(),
    };
    Ok(response
        .content_type("application/json")
        .body(body_response))
}

/// Actix GraphQL Handler for POST requests
pub async fn post_graphql_handler<Query, Mutation, Subscription, CtxT, S>(
    schema: &juniper::RootNode<Query, Mutation, Subscription, S>,
    context: &CtxT,
    req: HttpRequest,
    payload: actix_web::web::Payload,
) -> Result<HttpResponse, Error>
where
    Query: juniper::GraphQLTypeAsync<S, Context = CtxT>,
    Query::TypeInfo: Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = CtxT>,
    Mutation::TypeInfo: Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = CtxT>,
    Subscription::TypeInfo: Sync,
    CtxT: Sync,
    S: ScalarValue + Send + Sync,
{
    let req = match req.content_type() {
        "application/json" => {
            let body = String::from_request(&req, &mut payload.into_inner()).await?;
            serde_json::from_str::<GraphQLBatchRequest<S>>(&body)
                .map_err(JsonPayloadError::Deserialize)
        }
        "application/graphql" => {
            let body = String::from_request(&req, &mut payload.into_inner()).await?;
            Ok(GraphQLBatchRequest::Single(GraphQLRequest::new(
                body, None, None,
            )))
        }
        _ => Err(JsonPayloadError::ContentType),
    }?;
    let gql_batch_response = req.execute(schema, context).await;
    let gql_response = serde_json::to_string(&gql_batch_response)?;
    let mut response = match gql_batch_response.is_ok() {
        true => HttpResponse::Ok(),
        false => HttpResponse::BadRequest(),
    };
    Ok(response.content_type("application/json").body(gql_response))
}

/// Create a handler that replies with an HTML page containing GraphiQL. This does not handle routing, so you can mount it on any endpoint
///
/// For example:
///
/// ```
/// # use juniper_actix::graphiql_handler;
/// # use actix_web::{web, App};
///
/// let app = App::new()
///          .route("/", web::get().to(|| graphiql_handler("/graphql", Some("/graphql/subscriptions"))));
/// ```
pub async fn graphiql_handler(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: Option<&'static str>,
) -> Result<HttpResponse, Error> {
    let html = graphiql_source(graphql_endpoint_url, subscriptions_endpoint_url);
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// Create a handler that replies with an HTML page containing GraphQL Playground. This does not handle routing, so you cant mount it on any endpoint.
pub async fn playground_handler(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: Option<&'static str>,
) -> Result<HttpResponse, Error> {
    let html = playground_source(graphql_endpoint_url, subscriptions_endpoint_url);
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

#[cfg(feature = "subscriptions")]
/// `juniper_actix` subscriptions handler implementation.
pub mod subscriptions {
    use std::{fmt, pin::pin, sync::Arc};

    use actix_web::{
        http::header::{HeaderName, HeaderValue},
        web, HttpRequest, HttpResponse,
    };
    use futures::{future, SinkExt as _, StreamExt as _};
    use juniper::{GraphQLSubscriptionType, GraphQLTypeAsync, RootNode, ScalarValue};
    use juniper_graphql_ws::{graphql_transport_ws, graphql_ws, ArcSchema, Init};

    /// Serves by auto-selecting between the
    /// [legacy `graphql-ws` GraphQL over WebSocket Protocol][old] and the
    /// [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], based on the
    /// `Sec-Websocket-Protocol` HTTP header value.
    ///
    /// The `schema` argument is your [`juniper`] schema.
    ///
    /// The `init` argument is used to provide the custom [`juniper::Context`] and additional
    /// configuration for connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the
    /// context and configuration are already known, or it can be a closure that gets executed
    /// asynchronously whenever a client sends the subscription initialization message. Using a
    /// closure allows to perform an authentication based on the parameters provided by a client.
    ///
    /// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
    /// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
    pub async fn ws_handler<Query, Mutation, Subscription, CtxT, S, I>(
        req: HttpRequest,
        stream: web::Payload,
        schema: Arc<RootNode<Query, Mutation, Subscription, S>>,
        init: I,
    ) -> Result<HttpResponse, actix_web::Error>
    where
        Query: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
        Query::TypeInfo: Send + Sync,
        Mutation: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
        Mutation::TypeInfo: Send + Sync,
        Subscription: GraphQLSubscriptionType<S, Context = CtxT> + Send + 'static,
        Subscription::TypeInfo: Send + Sync,
        CtxT: Unpin + Send + Sync + 'static,
        S: ScalarValue + Send + Sync + 'static,
        I: Init<S, CtxT> + Send,
    {
        if req
            .headers()
            .get("sec-websocket-protocol")
            .map(AsRef::as_ref)
            == Some("graphql-ws".as_bytes())
        {
            graphql_ws_handler(req, stream, schema, init).await
        } else {
            graphql_transport_ws_handler(req, stream, schema, init).await
        }
    }

    /// Serves the [legacy `graphql-ws` GraphQL over WebSocket Protocol][old].
    ///
    /// The `init` argument is used to provide the context and additional configuration for
    /// connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the context and
    /// configuration are already known, or it can be a closure that gets executed asynchronously
    /// when the client sends the `GQL_CONNECTION_INIT` message. Using a closure allows to perform
    /// an authentication based on the parameters provided by a client.
    ///
    /// > __WARNING__: This protocol has been deprecated in favor of the
    ///                [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], which is
    ///                provided by the [`graphql_transport_ws_handler()`] function.
    ///
    /// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
    /// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
    pub async fn graphql_ws_handler<Query, Mutation, Subscription, CtxT, S, I>(
        req: HttpRequest,
        stream: web::Payload,
        schema: Arc<RootNode<Query, Mutation, Subscription, S>>,
        init: I,
    ) -> Result<HttpResponse, actix_web::Error>
    where
        Query: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
        Query::TypeInfo: Send + Sync,
        Mutation: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
        Mutation::TypeInfo: Send + Sync,
        Subscription: GraphQLSubscriptionType<S, Context = CtxT> + Send + 'static,
        Subscription::TypeInfo: Send + Sync,
        CtxT: Unpin + Send + Sync + 'static,
        S: ScalarValue + Send + Sync + 'static,
        I: Init<S, CtxT> + Send,
    {
        let (mut resp, mut ws_tx, ws_rx) = actix_ws::handle(&req, stream)?;
        let (s_tx, mut s_rx) = graphql_ws::Connection::new(ArcSchema(schema), init).split();

        actix_web::rt::spawn(async move {
            let input = ws_rx
                .map(|r| r.map(Message))
                .forward(s_tx.sink_map_err(|e| match e {}));
            let output = pin!(async move {
                while let Some(msg) = s_rx.next().await {
                    match serde_json::to_string(&msg) {
                        Ok(m) => {
                            if ws_tx.text(m).await.is_err() {
                                return;
                            }
                        }
                        Err(e) => {
                            _ = ws_tx
                                .close(Some(actix_ws::CloseReason {
                                    code: actix_ws::CloseCode::Error,
                                    description: Some(format!("error serializing response: {e}")),
                                }))
                                .await;
                            return;
                        }
                    }
                }
                _ = ws_tx
                    .close(Some((actix_ws::CloseCode::Normal, "Normal Closure").into()))
                    .await;
            });

            // No errors can be returned here, so ignoring is OK.
            _ = future::select(input, output).await;
        });

        resp.headers_mut().insert(
            HeaderName::from_static("sec-websocket-protocol"),
            HeaderValue::from_static("graphql-ws"),
        );
        Ok(resp)
    }

    /// Serves the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new].
    ///
    /// The `init` argument is used to provide the context and additional configuration for
    /// connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the context and
    /// configuration are already known, or it can be a closure that gets executed asynchronously
    /// when the client sends the `ConnectionInit` message. Using a closure allows to perform an
    /// authentication based on the parameters provided by a client.
    ///
    /// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
    pub async fn graphql_transport_ws_handler<Query, Mutation, Subscription, CtxT, S, I>(
        req: HttpRequest,
        stream: web::Payload,
        schema: Arc<RootNode<Query, Mutation, Subscription, S>>,
        init: I,
    ) -> Result<HttpResponse, actix_web::Error>
    where
        Query: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
        Query::TypeInfo: Send + Sync,
        Mutation: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
        Mutation::TypeInfo: Send + Sync,
        Subscription: GraphQLSubscriptionType<S, Context = CtxT> + Send + 'static,
        Subscription::TypeInfo: Send + Sync,
        CtxT: Unpin + Send + Sync + 'static,
        S: ScalarValue + Send + Sync + 'static,
        I: Init<S, CtxT> + Send,
    {
        let (mut resp, mut ws_tx, ws_rx) = actix_ws::handle(&req, stream)?;
        let (s_tx, mut s_rx) =
            graphql_transport_ws::Connection::new(ArcSchema(schema), init).split();

        actix_web::rt::spawn(async move {
            let input = ws_rx
                .map(|r| r.map(Message))
                .forward(s_tx.sink_map_err(|e| match e {}));
            let output = pin!(async move {
                while let Some(output) = s_rx.next().await {
                    match output {
                        graphql_transport_ws::Output::Message(msg) => {
                            match serde_json::to_string(&msg) {
                                Ok(m) => {
                                    if ws_tx.text(m).await.is_err() {
                                        return;
                                    }
                                }
                                Err(e) => {
                                    _ = ws_tx
                                        .close(Some(actix_ws::CloseReason {
                                            code: actix_ws::CloseCode::Error,
                                            description: Some(format!(
                                                "error serializing response: {e}",
                                            )),
                                        }))
                                        .await;
                                    return;
                                }
                            }
                        }
                        graphql_transport_ws::Output::Close { code, message } => {
                            _ = ws_tx
                                .close(Some(actix_ws::CloseReason {
                                    code: code.into(),
                                    description: Some(message),
                                }))
                                .await;
                            return;
                        }
                    }
                }
                _ = ws_tx
                    .close(Some((actix_ws::CloseCode::Normal, "Normal Closure").into()))
                    .await;
            });

            // No errors can be returned here, so ignoring is OK.
            _ = future::select(input, output).await;
        });

        resp.headers_mut().insert(
            HeaderName::from_static("sec-websocket-protocol"),
            HeaderValue::from_static("graphql-transport-ws"),
        );
        Ok(resp)
    }

    #[derive(Debug)]
    struct Message(actix_ws::Message);

    impl<S: ScalarValue> TryFrom<Message> for graphql_transport_ws::Input<S> {
        type Error = Error;

        fn try_from(msg: Message) -> Result<Self, Self::Error> {
            match msg.0 {
                actix_ws::Message::Text(text) => serde_json::from_slice(text.as_bytes())
                    .map(Self::Message)
                    .map_err(Error::Serde),
                actix_ws::Message::Binary(bytes) => serde_json::from_slice(bytes.as_ref())
                    .map(Self::Message)
                    .map_err(Error::Serde),
                actix_ws::Message::Close(_) => Ok(Self::Close),
                other => Err(Error::UnexpectedClientMessage(other)),
            }
        }
    }

    impl<S: ScalarValue> TryFrom<Message> for graphql_ws::ClientMessage<S> {
        type Error = Error;

        fn try_from(msg: Message) -> Result<Self, Self::Error> {
            match msg.0 {
                actix_ws::Message::Text(text) => {
                    serde_json::from_slice(text.as_bytes()).map_err(Error::Serde)
                }
                actix_ws::Message::Binary(bytes) => {
                    serde_json::from_slice(bytes.as_ref()).map_err(Error::Serde)
                }
                actix_ws::Message::Close(_) => Ok(Self::ConnectionTerminate),
                other => Err(Error::UnexpectedClientMessage(other)),
            }
        }
    }

    /// Possible errors of serving an [`actix_ws`] connection.
    #[derive(Debug)]
    enum Error {
        /// Deserializing of a client [`actix_ws::Message`] failed.
        Serde(serde_json::Error),

        /// Unexpected client [`actix_ws::Message`].
        UnexpectedClientMessage(actix_ws::Message),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Serde(e) => write!(f, "`serde` error: {e}"),
                Self::UnexpectedClientMessage(m) => {
                    write!(f, "unexpected message received from client: {m:?}")
                }
            }
        }
    }

    impl std::error::Error for Error {}
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;

    use actix_http::body::MessageBody;
    use actix_web::{
        dev::ServiceResponse,
        http,
        http::header::{ACCEPT, CONTENT_TYPE},
        test::{self, TestRequest},
        web::Data,
        App,
    };
    use futures::future;
    use juniper::{
        http::tests::{run_http_test_suite, HttpIntegration, TestResponse},
        tests::fixtures::starwars::schema::{Database, Query},
        EmptyMutation, EmptySubscription, RootNode,
    };

    use super::*;

    type Schema = juniper::RootNode<Query, EmptyMutation<Database>, EmptySubscription<Database>>;

    async fn take_response_body_string(resp: ServiceResponse) -> String {
        let mut body = resp.into_body();
        String::from_utf8(
            future::poll_fn(|cx| Pin::new(&mut body).poll_next(cx))
                .await
                .unwrap()
                .unwrap()
                .to_vec(),
        )
        .unwrap()
    }

    async fn index(
        req: HttpRequest,
        payload: actix_web::web::Payload,
        schema: web::Data<Schema>,
    ) -> Result<HttpResponse, Error> {
        let context = Database::new();
        graphql_handler(&schema, &context, req, payload).await
    }

    #[actix_web::rt::test]
    async fn graphiql_response_does_not_panic() {
        let result = graphiql_handler("/abcd", None).await;
        assert!(result.is_ok())
    }

    #[actix_web::rt::test]
    async fn graphiql_endpoint_matches() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            graphiql_handler("/abcd", None).await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = TestRequest::get()
            .uri("/")
            .append_header((ACCEPT, "text/html"))
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::rt::test]
    async fn graphiql_endpoint_returns_graphiql_source() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            graphiql_handler("/dogs-api/graphql", Some("/dogs-api/subscriptions")).await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = TestRequest::get()
            .uri("/")
            .append_header((ACCEPT, "text/html"))
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            "text/html; charset=utf-8"
        );
        let body = take_response_body_string(resp).await;
        assert!(body.contains("var JUNIPER_URL = '/dogs-api/graphql';"));
        assert!(body.contains("var JUNIPER_SUBSCRIPTIONS_URL = '/dogs-api/subscriptions';"))
    }

    #[actix_web::rt::test]
    async fn playground_endpoint_matches() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            playground_handler("/abcd", None).await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = TestRequest::get()
            .uri("/")
            .append_header((ACCEPT, "text/html"))
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::rt::test]
    async fn playground_endpoint_returns_playground_source() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            playground_handler("/dogs-api/graphql", Some("/dogs-api/subscriptions")).await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = TestRequest::get()
            .uri("/")
            .append_header((ACCEPT, "text/html"))
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            "text/html; charset=utf-8"
        );
        let body = take_response_body_string(resp).await;
        assert!(body.contains(
            "endpoint: '/dogs-api/graphql', subscriptionEndpoint: '/dogs-api/subscriptions'",
        ));
    }

    #[actix_web::rt::test]
    async fn graphql_post_works_json_post() {
        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let req = TestRequest::post()
            .append_header(("content-type", "application/json; charset=utf-8"))
            .set_payload(
                r#"{ "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" }"#,
            )
            .uri("/")
            .to_request();

        let mut app = test::init_service(
            App::new()
                .app_data(Data::new(schema))
                .route("/", web::post().to(index)),
        )
        .await;

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json",
        );
        assert_eq!(
            take_response_body_string(resp).await,
            r#"{"data":{"hero":{"name":"R2-D2"}}}"#
        );
    }

    #[actix_web::rt::test]
    async fn graphql_get_works() {
        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let req = TestRequest::get()
            .append_header(("content-type", "application/json"))
            .uri("/?query=%7B%20hero%28episode%3A%20NEW_HOPE%29%20%7B%20name%20%7D%20%7D&variables=null")
            .to_request();

        let mut app = test::init_service(
            App::new()
                .app_data(Data::new(schema))
                .route("/", web::get().to(index)),
        )
        .await;

        let resp = test::call_service(&mut app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json",
        );
        assert_eq!(
            take_response_body_string(resp).await,
            r#"{"data":{"hero":{"name":"R2-D2"}}}"#
        );
    }

    #[actix_web::rt::test]
    async fn batch_request_works() {
        use juniper::{
            tests::fixtures::starwars::schema::{Database, Query},
            EmptyMutation, EmptySubscription, RootNode,
        };

        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let req = TestRequest::post()
            .append_header(("content-type", "application/json"))
            .set_payload(
                r#"[
                     { "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" },
                     { "variables": null, "query": "{ hero(episode: EMPIRE) { id name } }" }
                 ]"#,
            )
            .uri("/")
            .to_request();

        let mut app = test::init_service(
            App::new()
                .app_data(Data::new(schema))
                .route("/", web::post().to(index)),
        )
        .await;

        let resp = test::call_service(&mut app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json",
        );
        assert_eq!(
            take_response_body_string(resp).await,
            r#"[{"data":{"hero":{"name":"R2-D2"}}},{"data":{"hero":{"id":"1000","name":"Luke Skywalker"}}}]"#
        );
    }

    #[test]
    fn batch_request_deserialization_can_fail() {
        let json = r#"blah"#;
        let result: Result<GraphQLBatchRequest, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }

    pub struct TestActixWebIntegration;

    impl TestActixWebIntegration {
        fn make_request(&self, req: TestRequest) -> TestResponse {
            actix_web::rt::System::new().block_on(async move {
                let schema = Schema::new(
                    Query,
                    EmptyMutation::<Database>::new(),
                    EmptySubscription::<Database>::new(),
                );

                let mut app = test::init_service(
                    App::new()
                        .app_data(Data::new(schema))
                        .route("/", web::to(index)),
                )
                .await;

                let resp = test::call_service(&mut app, req.to_request()).await;
                make_test_response(resp).await
            })
        }
    }

    impl HttpIntegration for TestActixWebIntegration {
        fn get(&self, url: &str) -> TestResponse {
            self.make_request(TestRequest::get().uri(url))
        }

        fn post_json(&self, url: &str, body: &str) -> TestResponse {
            self.make_request(
                TestRequest::post()
                    .append_header(("content-type", "application/json"))
                    .set_payload(body.to_owned())
                    .uri(url),
            )
        }

        fn post_graphql(&self, url: &str, body: &str) -> TestResponse {
            self.make_request(
                TestRequest::post()
                    .append_header(("content-type", "application/graphql"))
                    .set_payload(body.to_owned())
                    .uri(url),
            )
        }
    }

    async fn make_test_response(resp: ServiceResponse) -> TestResponse {
        let status_code = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .into();
        let body = take_response_body_string(resp).await;
        TestResponse {
            status_code: status_code as i32,
            body: Some(body),
            content_type,
        }
    }

    #[test]
    fn test_actix_web_integration() {
        run_http_test_suite(&TestActixWebIntegration);
    }
}

#[cfg(feature = "subscriptions")]
#[cfg(test)]
mod subscription_tests {
    use actix_http::ws;
    use actix_test::start;
    use actix_web::{web, App, Error, HttpRequest, HttpResponse};
    use juniper::{
        futures::{SinkExt, StreamExt},
        http::tests::{graphql_transport_ws, graphql_ws, WsIntegration, WsIntegrationMessage},
        tests::fixtures::starwars::schema::{Database, Query, Subscription},
        EmptyMutation, LocalBoxFuture,
    };
    use juniper_graphql_ws::ConnectionConfig;
    use tokio::time::timeout;

    use super::subscriptions;

    struct TestWsIntegration(&'static str);

    impl TestWsIntegration {
        async fn run_async(
            &self,
            messages: Vec<WsIntegrationMessage>,
        ) -> Result<(), anyhow::Error> {
            let proto = self.0;

            let mut server = start(|| {
                App::new()
                    .app_data(web::Data::new(Schema::new(
                        Query,
                        EmptyMutation::<Database>::new(),
                        Subscription,
                    )))
                    .service(web::resource("/subscriptions").to(subscription(proto)))
            });
            let mut framed = server.ws_at("/subscriptions").await.unwrap();

            for message in &messages {
                match message {
                    WsIntegrationMessage::Send(body) => {
                        framed
                            .send(ws::Message::Text(body.to_string().into()))
                            .await
                            .map_err(|e| anyhow::anyhow!("WS error: {e:?}"))?;
                    }
                    WsIntegrationMessage::Expect(body, message_timeout) => {
                        let frame = timeout(*message_timeout, framed.next())
                            .await
                            .map_err(|_| anyhow::anyhow!("Timed-out waiting for message"))?
                            .ok_or_else(|| anyhow::anyhow!("Empty message received"))?
                            .map_err(|e| anyhow::anyhow!("WS error: {e:?}"))?;

                        match frame {
                            ws::Frame::Text(ref bytes) => {
                                let value: serde_json::Value = serde_json::from_slice(bytes)
                                    .map_err(|e| anyhow::anyhow!("Serde error: {e:?}"))?;

                                if value != *body {
                                    return Err(anyhow::anyhow!(
                                        "Expected message: {body}. \
                                         Received message: {value}",
                                    ));
                                }
                            }
                            ws::Frame::Close(Some(reason)) => {
                                let actual = serde_json::json!({
                                    "code": u16::from(reason.code),
                                    "description": reason.description,
                                });
                                if actual != *body {
                                    return Err(anyhow::anyhow!(
                                        "Expected message: {body}. \
                                         Received message: {actual}",
                                    ));
                                }
                            }
                            f => return Err(anyhow::anyhow!("Received non-text frame: {f:?}")),
                        }
                    }
                }
            }

            Ok(())
        }
    }

    impl WsIntegration for TestWsIntegration {
        fn run(
            &self,
            messages: Vec<WsIntegrationMessage>,
        ) -> LocalBoxFuture<Result<(), anyhow::Error>> {
            Box::pin(self.run_async(messages))
        }
    }

    type Schema = juniper::RootNode<Query, EmptyMutation<Database>, Subscription>;

    fn subscription(
        proto: &'static str,
    ) -> impl actix_web::Handler<
        (HttpRequest, web::Payload, web::Data<Schema>),
        Output = Result<HttpResponse, Error>,
    > {
        move |req: HttpRequest, stream: web::Payload, schema: web::Data<Schema>| async move {
            let context = Database::new();
            let schema = schema.into_inner();
            let config = ConnectionConfig::new(context);

            if proto == "graphql-ws" {
                subscriptions::graphql_ws_handler(req, stream, schema, config).await
            } else {
                subscriptions::graphql_transport_ws_handler(req, stream, schema, config).await
            }
        }
    }

    #[actix_web::rt::test]
    async fn test_graphql_ws_integration() {
        graphql_ws::run_test_suite(&mut TestWsIntegration("graphql-ws")).await;
    }

    #[actix_web::rt::test]
    async fn test_graphql_transport_ws_integration() {
        graphql_transport_ws::run_test_suite(&mut TestWsIntegration("graphql-transport-ws")).await;
    }
}
