/*!

# juniper_actix

This repository contains the [actix][actix] web server integration for
[Juniper][Juniper], a [GraphQL][GraphQL] implementation for Rust.

## Documentation

For documentation, including guides and examples, check out [Juniper][Juniper].

A basic usage example can also be found in the [API documentation][documentation].

## Examples

Check [examples/actix_server][example] for example code of a working actix
server with GraphQL handlers.

## Links

* [Juniper][Juniper]
* [API Reference][documentation]
* [actix][actix]

## License

This project is under the BSD-2 license.

Check the LICENSE file for details.

[actix]: https://github.com/actix/actix-web
[Juniper]: https://github.com/graphql-rust/juniper
[GraphQL]: http://graphql.org
[documentation]: https://docs.rs/juniper_actix
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_actix/examples/actix_server.rs

*/

#![deny(missing_docs)]
#![deny(warnings)]
#![doc(html_root_url = "https://docs.rs/juniper_actix/0.1.0")]

// use futures::{FutureExt as _};
use actix_web::{web, Error, HttpResponse};
use juniper::graphiql::graphiql_source;
use juniper::http::playground::playground_source;
use juniper::{DefaultScalarValue, InputValue, ScalarValue};
use serde::Deserialize;

/// Enum for handling batch requests
#[derive(Debug, serde_derive::Deserialize, PartialEq)]
#[serde(untagged)]
#[serde(bound = "InputValue<S>: Deserialize<'de>")]
pub enum GraphQLBatchRequest<S = DefaultScalarValue>
where
    S: ScalarValue,
{
    /// Single Request
    Single(juniper::http::GraphQLRequest<S>),
    /// Batch Request
    Batch(Vec<juniper::http::GraphQLRequest<S>>),
}

#[allow(dead_code)]
impl<S> GraphQLBatchRequest<S>
where
    S: ScalarValue,
{
    /// Execute synchronous
    pub fn execute_sync<'a, CtxT, QueryT, MutationT, SubscriptionT>(
        &'a self,
        root_node: &'a juniper::RootNode<QueryT, MutationT, SubscriptionT, S>,
        context: &CtxT,
    ) -> GraphQLBatchResponse<'a, S>
    where
        QueryT: juniper::GraphQLType<S, Context = CtxT>,
        MutationT: juniper::GraphQLType<S, Context = CtxT>,
        SubscriptionT: juniper::GraphQLType<S, Context = CtxT>,
        SubscriptionT::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
    {
        match *self {
            GraphQLBatchRequest::Single(ref request) => {
                GraphQLBatchResponse::Single(request.execute_sync(root_node, context))
            }
            GraphQLBatchRequest::Batch(ref requests) => GraphQLBatchResponse::Batch(
                requests
                    .iter()
                    .map(|request| request.execute_sync(root_node, context))
                    .collect(),
            ),
        }
    }

    /// Execute asynchronous
    pub async fn execute<'a, CtxT, QueryT, MutationT, SubscriptionT>(
        &'a self,
        root_node: &'a juniper::RootNode<'a, QueryT, MutationT, SubscriptionT, S>,
        context: &'a CtxT,
    ) -> GraphQLBatchResponse<'a, S>
    where
        QueryT: juniper::GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
        QueryT::TypeInfo: Send + Sync,
        MutationT: juniper::GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
        MutationT::TypeInfo: Send + Sync,
        SubscriptionT: juniper::GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync,
        SubscriptionT::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
        S: Send + Sync,
    {
        match *self {
            GraphQLBatchRequest::Single(ref request) => {
                let res = request.execute(root_node, context).await;
                GraphQLBatchResponse::Single(res)
            }
            GraphQLBatchRequest::Batch(ref requests) => {
                let futures = requests
                    .iter()
                    .map(|request| request.execute(root_node, context))
                    .collect::<Vec<_>>();
                let responses = futures::future::join_all(futures).await;

                GraphQLBatchResponse::Batch(responses)
            }
        }
    }
}

/// Enum for the batch response
#[derive(serde_derive::Serialize)]
#[serde(untagged)]
pub enum GraphQLBatchResponse<'a, S = DefaultScalarValue>
where
    S: ScalarValue,
{
    /// When is a single response
    Single(juniper::http::GraphQLResponse<'a, S>),
    /// When is a batch response
    Batch(Vec<juniper::http::GraphQLResponse<'a, S>>),
}

#[allow(dead_code)]
impl<'a, S> GraphQLBatchResponse<'a, S>
where
    S: ScalarValue,
{
    fn is_ok(&self) -> bool {
        match self {
            GraphQLBatchResponse::Single(res) => res.is_ok(),
            GraphQLBatchResponse::Batch(reses) => reses.iter().all(|res| res.is_ok()),
        }
    }
}

/// Actix GraphQL Handler for GET requests
pub async fn get_graphql_handler<Query, Mutation, Subscription, Context, S>(
    schema: &juniper::RootNode<'static, Query, Mutation, Subscription, S>,
    context: &Context,
    req: web::Query<GraphQLBatchRequest<S>>,
) -> Result<HttpResponse, Error>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    let gql_batch_response = req.execute(schema, context).await;

    let gql_response = serde_json::to_string(&gql_batch_response)?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(gql_response))
}
/// Actix GraphQL Handler for POST requests
pub async fn post_graphql_handler<Query, Mutation, Subscription, Context, S>(
    schema: &juniper::RootNode<'static, Query, Mutation, Subscription, S>,
    context: &Context,
    req: web::Json<GraphQLBatchRequest<S>>,
) -> Result<HttpResponse, Error>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    let gql_batch_response = req.execute(schema, context).await;
    let gql_response = serde_json::to_string(&gql_batch_response)?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(gql_response))
}

/// Create a handler that replies with an HTML page containing GraphiQL. This does not handle routing, so you can mount it on any endpoint
///
/// For example:
///
/// ```
/// # extern crate actix;
/// # extern crate juniper_actix;
/// #
/// # use juniper_actix::graphiql_handler;
/// # use actix_web::{web, App};
///
/// let app = App::new()
///          .route("/", web::get().to(|| graphiql_handler("/graphql")));
/// ```
#[allow(dead_code)]
pub async fn graphiql_handler(graphql_endpoint_url: &str) -> Result<HttpResponse, Error> {
    let html = graphiql_source(graphql_endpoint_url);
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
pub mod subscriptions {
    use actix::{Actor, ActorContext, AsyncContext, StreamHandler, WrapFuture};
    use actix_web::error::PayloadError;
    use actix_web::web::Bytes;
    use actix_web::{web, Error, HttpRequest, HttpResponse};
    use actix_web_actors::ws;
    use actix_web_actors::ws::{handshake_with_protocols, WebsocketContext};
    use futures::{Stream, StreamExt};
    use juniper::{http::GraphQLRequest, InputValue, ScalarValue, SubscriptionCoordinator};
    use juniper_subscriptions::Coordinator;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    pub const GQL_CONNECTION_INIT: &str = "connection_init";
    pub const GQL_CONNECTION_ACK: &str = "connection_ack";
    pub const GQL_CONNECTION_ERROR: &str = "connection_error";
    pub const GQL_CONNECTION_KEEP_ALIVE: &str = "ka";
    pub const GQL_CONNECTION_TERMINATE: &str = "connection_terminate";
    pub const GQL_START: &str = "start";
    pub const GQL_DATA: &str = "data";
    pub const GQL_ERROR: &str = "error";
    pub const GQL_COMPLETE: &str = "complete";
    pub const GQL_STOP: &str = "stop";

    fn start<Query, Mutation, Subscription, Context, S, FunStart, T>(
        actor: GraphQLWebSocketActor<Query, Mutation, Subscription, Context, S, FunStart>,
        req: &HttpRequest,
        stream: T,
    ) -> Result<HttpResponse, Error>
    where
        T: Stream<Item = Result<Bytes, PayloadError>> + 'static,
        S: ScalarValue + Send + Sync + 'static,
        Context: Clone + Send + Sync + 'static + std::marker::Unpin,
        Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Query::TypeInfo: Send + Sync,
        Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Mutation::TypeInfo: Send + Sync,
        Subscription:
            juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
        Subscription::TypeInfo: Send + Sync,
        FunStart: std::marker::Unpin + FnMut(&mut Context, String) -> Result<(), String> + 'static,
    {
        let mut res = handshake_with_protocols(req, &["graphql-ws"])?;
        Ok(res.streaming(WebsocketContext::create(actor, stream)))
    }
    /// Since this implementation makes usage of the unsafe keyword i will consider this as unsafe for now.
    pub async unsafe fn graphql_subscriptions<Query, Mutation, Subscription, Context, S, FunStart>(
        coordinator: web::Data<Coordinator<'static, Query, Mutation, Subscription, Context, S>>,
        context: Context,
        stream: web::Payload,
        req: HttpRequest,
        on_start: FunStart,
    ) -> Result<HttpResponse, Error>
    where
        S: ScalarValue + Send + Sync + 'static,
        Context: Clone + Send + Sync + 'static + std::marker::Unpin,
        Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Query::TypeInfo: Send + Sync,
        Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Mutation::TypeInfo: Send + Sync,
        Subscription:
            juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
        Subscription::TypeInfo: Send + Sync,
        FunStart: std::marker::Unpin + FnMut(&mut Context, String) -> Result<(), String> + 'static,
    {
        start(
            GraphQLWebSocketActor {
                coordinator: coordinator.into_inner(),
                graphql_context: context,
                is_closed: Arc::new(AtomicBool::new(false)),
                on_start,
            },
            &req,
            stream,
        )
    }

    struct GraphQLWebSocketActor<Query, Mutation, Subscription, Context, S, FunStart>
    where
        S: ScalarValue + Send + Sync + 'static,
        Context: Clone + Send + Sync + 'static + std::marker::Unpin,
        Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Query::TypeInfo: Send + Sync,
        Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Mutation::TypeInfo: Send + Sync,
        Subscription:
            juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
        Subscription::TypeInfo: Send + Sync,
        FunStart: std::marker::Unpin + FnMut(&mut Context, String) -> Result<(), String> + 'static,
    {
        pub is_closed: Arc<AtomicBool>,
        pub graphql_context: Context,
        pub coordinator: Arc<Coordinator<'static, Query, Mutation, Subscription, Context, S>>,
        pub on_start: FunStart,
    }

    impl<Query, Mutation, Subscription, Context, S, FunStart> Actor
        for GraphQLWebSocketActor<Query, Mutation, Subscription, Context, S, FunStart>
    where
        S: ScalarValue + Send + Sync + 'static,
        Context: Clone + Send + Sync + 'static + std::marker::Unpin,
        Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Query::TypeInfo: Send + Sync,
        Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Mutation::TypeInfo: Send + Sync,
        Subscription:
            juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
        Subscription::TypeInfo: Send + Sync,
        FunStart: std::marker::Unpin + FnMut(&mut Context, String) -> Result<(), String> + 'static,
    {
        type Context = ws::WebsocketContext<
            GraphQLWebSocketActor<Query, Mutation, Subscription, Context, S, FunStart>,
        >;
    }

    impl<Query, Mutation, Subscription, Context, S, FunStart>
        StreamHandler<Result<ws::Message, ws::ProtocolError>>
        for GraphQLWebSocketActor<Query, Mutation, Subscription, Context, S, FunStart>
    where
        S: ScalarValue + Send + Sync + 'static,
        Context: Clone + Send + Sync + 'static + std::marker::Unpin,
        Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Query::TypeInfo: Send + Sync,
        Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Mutation::TypeInfo: Send + Sync,
        Subscription:
            juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
        Subscription::TypeInfo: Send + Sync,
        FunStart: std::marker::Unpin + FnMut(&mut Context, String) -> Result<(), String> + 'static,
    {
        fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
            let msg = match msg {
                Err(_) => {
                    ctx.stop();
                    return;
                }
                Ok(msg) => msg,
            };
            let coordinator = self.coordinator.clone();
            let context = self.graphql_context.clone();
            let got_close_signal = self.is_closed.clone();
            match msg {
                ws::Message::Text(text) => {
                    let m = text.trim();
                    let request: WsPayload<S> = serde_json::from_str(m).expect("Invalid WsPayload");
                    match request.type_name.as_str() {
                        GQL_CONNECTION_INIT => {
                            match (self.on_start)(&mut self.graphql_context, String::from(m)) {
                                Ok(_) => {
                                    ctx.text(format!(
                                        r#"{{"type":"{}", "payload": null }}"#,
                                        GQL_CONNECTION_ACK
                                    ));
                                    ctx.text(format!(
                                        r#"{{"type":"{}", "payload": null }}"#,
                                        GQL_CONNECTION_KEEP_ALIVE
                                    ));
                                }
                                Err(_err) => ctx.text(format!(
                                    r#"{{"type":"{}", "payload": null }}"#,
                                    GQL_CONNECTION_ERROR
                                )),
                            }
                        }
                        GQL_START => {
                            let payload = request.payload.expect("Could not deserialize payload");
                            let request_id = request.id.unwrap_or("1".to_owned());

                            let graphql_request = GraphQLRequest::<_>::new(
                                payload.query.expect("Could not deserialize query"),
                                None,
                                payload.variables,
                            );
                            {
                                let ctx_ref: *mut Self::Context = ctx;
                                let actor_future = async move {
                                    // I didnt found another way to handle the insertion of ctx into this block
                                    // So for now i will consider this as unsafe
                                    let ctx_ref = unsafe { ctx_ref.as_mut().unwrap() };
                                    let values_stream = {
                                        let subscribe_result =
                                            coordinator.subscribe(&graphql_request, &context).await;
                                        match subscribe_result {
                                            Ok(s) => s,
                                            Err(err) => {
                                                ctx_ref.text(format!(
                                                    r#"{{"type":"{}","id":"{}","payload":{}}}"#,
                                                    GQL_ERROR,
                                                    request_id,
                                                    serde_json::ser::to_string(&err).unwrap_or(
                                                        "Error deserializing GraphQLError"
                                                            .to_owned()
                                                    )
                                                ));
                                                let close_message = format!(
                                                    r#"{{"type":"{}","id":"{}","payload":null}}"#,
                                                    GQL_COMPLETE, request_id
                                                );
                                                ctx_ref.text(close_message);
                                                ctx_ref.stop();
                                                return;
                                            }
                                        }
                                    };
                                    let mut futures_stream = values_stream.into_future();
                                    while let (Some(response), stream) = futures_stream.await {
                                        futures_stream = stream.into_future();
                                        let request_id = request_id.clone();
                                        let closed = got_close_signal.load(Ordering::Relaxed);
                                        if !closed {
                                            let mut response_text = serde_json::to_string(
                                                &response,
                                            )
                                            .unwrap_or("Error deserializing respone".to_owned());
                                            response_text = format!(
                                                r#"{{"type":"{}","id":"{}","payload":{} }}"#,
                                                GQL_DATA, request_id, response_text
                                            );
                                            ctx_ref.text(response_text);
                                        } else {
                                            ctx_ref.stop();
                                            break;
                                        }
                                    }
                                }
                                .into_actor(self);
                                ctx.spawn(actor_future);
                            }
                        }
                        GQL_STOP => {
                            let request_id = request.id.unwrap_or("1".to_owned());
                            let close_message = format!(
                                r#"{{"type":"{}","id":"{}","payload":null}}"#,
                                GQL_COMPLETE, request_id
                            );
                            ctx.text(close_message);
                            got_close_signal.store(true, Ordering::Relaxed);
                            ctx.stop();
                        }
                        _ => {}
                    }
                }
                ws::Message::Binary(_) => println!("Unexpected binary"),
                ws::Message::Close(_) => {
                    got_close_signal.store(true, Ordering::Relaxed);
                    ctx.stop();
                }
                ws::Message::Continuation(_) => {
                    got_close_signal.store(true, Ordering::Relaxed);
                    ctx.stop();
                }
                _ => (),
            }
        }
    }

    #[derive(Deserialize)]
    #[serde(bound = "GraphQLPayload<S>: Deserialize<'de>")]
    struct WsPayload<S>
    where
        S: ScalarValue + Send + Sync + 'static,
    {
        id: Option<String>,
        #[serde(rename(deserialize = "type"))]
        type_name: String,
        payload: Option<GraphQLPayload<S>>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(bound = "InputValue<S>: Deserialize<'de>")]
    struct GraphQLPayload<S>
    where
        S: ScalarValue + Send + Sync + 'static,
    {
        variables: Option<InputValue<S>>,
        extensions: Option<HashMap<String, String>>,
        #[serde(rename(deserialize = "operationName"))]
        operaton_name: Option<String>,
        query: Option<String>,
    }

    #[derive(Serialize)]
    struct Output {
        data: String,
        variables: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::dev::ServiceResponse;
    use actix_web::{http, test, App};
    use futures::StreamExt;
    use juniper::{
        tests::{model::Database, schema::Query},
        EmptyMutation, EmptySubscription, RootNode,
    };

    type Schema =
        juniper::RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;

    async fn take_response_body_string(resp: &mut ServiceResponse) -> String {
        let (response_body, ..) = resp
            .take_body()
            .map(|body_out| body_out.unwrap().to_vec())
            .into_future()
            .await;
        let response_body = response_body.unwrap();
        String::from_utf8(response_body).unwrap()
    }

    async fn index(
        req: web::Json<GraphQLBatchRequest<DefaultScalarValue>>,
        schema: web::Data<Schema>,
    ) -> Result<HttpResponse, Error> {
        let context = Database::new();
        post_graphql_handler(&schema, &context, req).await
    }

    async fn index_get(
        req: web::Query<GraphQLBatchRequest<DefaultScalarValue>>,
        schema: web::Data<Schema>,
    ) -> Result<HttpResponse, Error> {
        let context = Database::new();
        get_graphql_handler(&schema, &context, req).await
    }

    #[actix_rt::test]
    async fn graphiql_response_does_not_panic() {
        let result = graphiql_handler("/abcd").await;
        assert!(result.is_ok())
    }

    #[actix_rt::test]
    async fn graphiql_endpoint_matches() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            graphiql_handler("/abcd").await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = test::TestRequest::get()
            .uri("/")
            .header("accept", "text/html")
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_rt::test]
    async fn playground_endpoint_matches() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            playground_handler("/abcd", None).await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = test::TestRequest::get()
            .uri("/")
            .header("accept", "text/html")
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_rt::test]
    async fn graphql_post_works_json_post() {
        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let req = test::TestRequest::post()
            .header("content-type", "application/json")
            .set_payload(
                r##"{ "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" }"##,
            )
            .uri("/")
            .to_request();

        let mut app =
            test::init_service(App::new().data(schema).route("/", web::post().to(index))).await;

        let mut resp = test::call_service(&mut app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            take_response_body_string(&mut resp).await,
            r#"{"data":{"hero":{"name":"R2-D2"}}}"#
        );
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json",
        );
    }

    #[actix_rt::test]
    async fn graphql_get_works() {
        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let req = test::TestRequest::get()
            .header("content-type", "application/json")
            .uri("/?query=%7B%20hero%28episode%3A%20NEW_HOPE%29%20%7B%20name%20%7D%20%7D&variables=null")
            .to_request();

        let mut app =
            test::init_service(App::new().data(schema).route("/", web::get().to(index_get))).await;

        let mut resp = test::call_service(&mut app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            take_response_body_string(&mut resp).await,
            r#"{"data":{"hero":{"name":"R2-D2"}}}"#
        );
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json",
        );
    }

    #[actix_rt::test]
    async fn batch_request_works() {
        use juniper::{
            tests::{model::Database, schema::Query},
            EmptyMutation, EmptySubscription, RootNode,
        };

        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let req = test::TestRequest::post()
            .header("content-type", "application/json")
            .set_payload(
                r##"[
                     { "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" },
                     { "variables": null, "query": "{ hero(episode: EMPIRE) { id name } }" }
                 ]"##,
            )
            .uri("/")
            .to_request();

        let mut app =
            test::init_service(App::new().data(schema).route("/", web::post().to(index))).await;

        let mut resp = test::call_service(&mut app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            take_response_body_string(&mut resp).await,
            r#"[{"data":{"hero":{"name":"R2-D2"}}},{"data":{"hero":{"id":"1000","name":"Luke Skywalker"}}}]"#
        );
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json",
        );
    }

    #[test]
    fn batch_request_deserialization_can_fail() {
        let json = r#"blah"#;
        let result: Result<GraphQLBatchRequest, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }
}
