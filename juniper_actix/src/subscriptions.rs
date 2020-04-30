use actix::{
    Actor, ActorContext, ActorFuture, AsyncContext, Handler, Message, Recipient, SpawnHandle,
    StreamHandler, WrapFuture,
};
use actix_web::{error::PayloadError, web, web::Bytes, Error, HttpRequest, HttpResponse};
use actix_web_actors::{
    ws,
    ws::{handshake_with_protocols, WebsocketContext},
};
use futures::{Stream, StreamExt};
use juniper::{http::GraphQLRequest, ScalarValue, SubscriptionCoordinator};
use juniper_subscriptions::ws_util::GraphQLOverWebSocketMessage;
pub use juniper_subscriptions::ws_util::{
    EmptySubscriptionHandler, GraphQLPayload, SubscriptionState, SubscriptionStateHandler,
    WsPayload,
};
use juniper_subscriptions::Coordinator;
use serde::Serialize;
use std::ops::Deref;
use std::{
    collections::HashMap,
    error::Error as StdError,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::time::Duration;

/// Websocket Subscription Handler
///  
/// # Arguments
/// * `coordinator` - The Subscription Coordinator stored in the App State
/// * `context` - The Context that will be used by the Coordinator
/// * `stream` - The Stream used by the request to create the WebSocket
/// * `req` - The Initial Request sent by the Client
/// * `handler` - The SubscriptionStateHandler implementation that will be used in the Subscription.
/// * `ka_interval` - The Duration that will be used to interleave the keep alive messages sent by the server. The default value is 10 seconds.
pub async fn graphql_subscriptions<Query, Mutation, Subscription, Context, S, SubHandler, E>(
    coordinator: web::Data<Coordinator<'static, Query, Mutation, Subscription, Context, S>>,
    context: Context,
    stream: web::Payload,
    req: HttpRequest,
    handler: Option<SubHandler>,
    ka_interval: Option<Duration>,
) -> Result<HttpResponse, Error>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
    SubHandler: SubscriptionStateHandler<Context, E> + 'static + std::marker::Unpin,
    E: 'static + std::error::Error + std::marker::Unpin,
{
    start(
        GraphQLWSSession {
            coordinator: coordinator.into_inner(),
            graphql_context: Arc::new(context),
            map_req_id_to_spawn_handle: HashMap::new(),
            has_started: Arc::new(AtomicBool::new(false)),
            handler,
            error_handler: std::marker::PhantomData,
            ka_interval: ka_interval.unwrap_or_else(|| Duration::from_secs(10)),
        },
        &req,
        stream,
    )
}

fn start<Query, Mutation, Subscription, Context, S, SubHandler, T, E>(
    actor: GraphQLWSSession<Query, Mutation, Subscription, Context, S, SubHandler, E>,
    req: &HttpRequest,
    stream: T,
) -> Result<HttpResponse, Error>
where
    T: Stream<Item = Result<Bytes, PayloadError>> + 'static,
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
    SubHandler: SubscriptionStateHandler<Context, E> + 'static + std::marker::Unpin,
    E: 'static + std::error::Error + std::marker::Unpin,
{
    let mut res = handshake_with_protocols(req, &["graphql-ws"])?;
    Ok(res.streaming(WebsocketContext::create(actor, stream)))
}

struct GraphQLWSSession<Query, Mutation, Subscription, Context, S, SubHandler, E>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
    SubHandler: SubscriptionStateHandler<Context, E> + 'static + std::marker::Unpin,
    E: 'static + std::error::Error + std::marker::Unpin,
{
    pub map_req_id_to_spawn_handle: HashMap<String, SpawnHandle>,
    pub has_started: Arc<AtomicBool>,
    pub graphql_context: Arc<Context>,
    pub coordinator: Arc<Coordinator<'static, Query, Mutation, Subscription, Context, S>>,
    pub handler: Option<SubHandler>,
    pub ka_interval: Duration,
    error_handler: std::marker::PhantomData<E>,
}

impl<Query, Mutation, Subscription, Context, S, SubHandler, E> Actor
    for GraphQLWSSession<Query, Mutation, Subscription, Context, S, SubHandler, E>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
    SubHandler: SubscriptionStateHandler<Context, E> + 'static + std::marker::Unpin,
    E: 'static + std::error::Error + std::marker::Unpin,
{
    type Context = ws::WebsocketContext<
        GraphQLWSSession<Query, Mutation, Subscription, Context, S, SubHandler, E>,
    >;
}

/// Internal Struct for handling Messages received from the subscriptions
#[derive(Message)]
#[rtype(result = "()")]
struct Msg(pub Option<String>);

impl<Query, Mutation, Subscription, Context, S, SubHandler, E> Handler<Msg>
    for GraphQLWSSession<Query, Mutation, Subscription, Context, S, SubHandler, E>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
    SubHandler: SubscriptionStateHandler<Context, E> + 'static + std::marker::Unpin,
    E: 'static + std::error::Error + std::marker::Unpin,
{
    type Result = ();
    fn handle(&mut self, msg: Msg, ctx: &mut Self::Context) {
        match msg.0 {
            Some(msg) => ctx.text(msg),
            None => ctx.close(None),
        }
    }
}

#[allow(dead_code)]
impl<Query, Mutation, Subscription, Context, S, SubHandler, E>
    GraphQLWSSession<Query, Mutation, Subscription, Context, S, SubHandler, E>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
    SubHandler: SubscriptionStateHandler<Context, E> + 'static + std::marker::Unpin,
    E: 'static + std::error::Error + std::marker::Unpin,
{
    fn gql_connection_ack() -> String {
        let type_value =
            serde_json::to_string(&GraphQLOverWebSocketMessage::ConnectionAck).unwrap();
        format!(r#"{{"type":{}, "payload": null }}"#, type_value)
    }

    fn gql_connection_ka() -> String {
        let type_value =
            serde_json::to_string(&GraphQLOverWebSocketMessage::ConnectionKeepAlive).unwrap();
        format!(r#"{{"type":{}, "payload": null }}"#, type_value)
    }

    fn gql_connection_error() -> String {
        let type_value =
            serde_json::to_string(&GraphQLOverWebSocketMessage::ConnectionError).unwrap();
        format!(r#"{{"type":{}, "payload": null }}"#, type_value)
    }
    fn gql_error<T: StdError + Serialize>(request_id: &String, err: T) -> String {
        let type_value = serde_json::to_string(&GraphQLOverWebSocketMessage::Error).unwrap();
        format!(
            r#"{{"type":{},"id":"{}","payload":{}}}"#,
            type_value,
            request_id,
            serde_json::ser::to_string(&err)
                .unwrap_or("Error deserializing GraphQLError".to_owned())
        )
    }

    fn gql_data(request_id: &String, response_text: String) -> String {
        let type_value = serde_json::to_string(&GraphQLOverWebSocketMessage::Data).unwrap();
        format!(
            r#"{{"type":{},"id":"{}","payload":{} }}"#,
            type_value, request_id, response_text
        )
    }

    fn gql_complete(request_id: &String) -> String {
        let type_value = serde_json::to_string(&GraphQLOverWebSocketMessage::Complete).unwrap();
        format!(
            r#"{{"type":{},"id":"{}","payload":null}}"#,
            type_value, request_id
        )
    }

    fn starting_handle(
        result: (
            GraphQLRequest<S>,
            String,
            Arc<Context>,
            Arc<Coordinator<'static, Query, Mutation, Subscription, Context, S>>,
        ),
        actor: &mut Self,
        ctx: &mut ws::WebsocketContext<Self>,
    ) -> actix::fut::FutureWrap<impl futures::Future<Output = ()>, Self> {
        let (req, req_id, gql_context, coord) = result;
        let addr = ctx.address();
        Self::handle_subscription(req, gql_context, req_id, coord, addr.recipient())
            .into_actor(actor)
    }

    async fn handle_subscription(
        req: GraphQLRequest<S>,
        graphql_context: Arc<Context>,
        request_id: String,
        coord: Arc<Coordinator<'static, Query, Mutation, Subscription, Context, S>>,
        addr: Recipient<Msg>,
    ) {
        let mut values_stream = {
            let subscribe_result = coord.subscribe(&req, &graphql_context).await;
            match subscribe_result {
                Ok(s) => s,
                Err(err) => {
                    let _ = addr.do_send(Msg(Some(Self::gql_error(&request_id, err))));
                    let _ = addr.do_send(Msg(Some(Self::gql_complete(&request_id))));
                    let _ = addr.do_send(Msg(None));
                    return;
                }
            }
        };

        while let Some(response) = values_stream.next().await {
            let request_id = request_id.clone();
            let response_text = serde_json::to_string(&response)
                .unwrap_or("Error deserializing respone".to_owned());
            let _ = addr.do_send(Msg(Some(Self::gql_data(&request_id, response_text))));
        }
        let _ = addr.do_send(Msg(Some(Self::gql_complete(&request_id))));
    }
}

impl<Query, Mutation, Subscription, Context, S, SubHandler, E>
    StreamHandler<Result<ws::Message, ws::ProtocolError>>
    for GraphQLWSSession<Query, Mutation, Subscription, Context, S, SubHandler, E>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
    SubHandler: SubscriptionStateHandler<Context, E> + 'static + std::marker::Unpin,
    E: 'static + std::error::Error + std::marker::Unpin,
{
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };
        let has_started = self.has_started.clone();
        let has_started_value = has_started.load(Ordering::Relaxed);
        match msg {
            ws::Message::Text(text) => {
                let m = text.trim();
                let request: WsPayload = match serde_json::from_str(m) {
                    Ok(payload) => payload,
                    Err(_) => {
                        return;
                    }
                };
                match request.type_name {
                    GraphQLOverWebSocketMessage::ConnectionInit => {
                        if let Some(handler) = &self.handler {
                            let state = SubscriptionState::OnConnection(
                                request.payload,
                                Arc::get_mut(&mut self.graphql_context).unwrap(),
                            );
                            let on_connect_result = handler.handle(state);
                            if let Err(_err) = on_connect_result {
                                ctx.text(Self::gql_connection_error());
                                ctx.stop();
                                return;
                            }
                        }
                        ctx.text(Self::gql_connection_ack());
                        ctx.text(Self::gql_connection_ka());
                        has_started.store(true, Ordering::Relaxed);
                        ctx.run_interval(self.ka_interval, |actor, ctx| {
                            let no_request = actor.map_req_id_to_spawn_handle.len() == 0;
                            if no_request {
                                ctx.stop();
                            } else {
                                ctx.text(Self::gql_connection_ka());
                            }
                        });
                    }
                    GraphQLOverWebSocketMessage::Start if has_started_value => {
                        let coordinator = self.coordinator.clone();

                        let payload = request
                            .graphql_payload::<S>()
                            .expect("Could not deserialize payload");
                        let request_id = request.id.unwrap_or("1".to_owned());
                        let graphql_request = GraphQLRequest::<_>::new(
                            payload.query.expect("Could not deserialize query"),
                            None,
                            payload.variables,
                        );
                        if let Some(handler) = &self.handler {
                            let state =
                                SubscriptionState::OnOperation(self.graphql_context.deref());
                            handler.handle(state).unwrap();
                        }
                        let context = self.graphql_context.clone();
                        {
                            use std::collections::hash_map::Entry;
                            let req_id = request_id.clone();
                            let future =
                                async move { (graphql_request, req_id, context, coordinator) }
                                    .into_actor(self)
                                    .then(Self::starting_handle);
                            match self.map_req_id_to_spawn_handle.entry(request_id) {
                                // Since there is another request being handle
                                // this just ignores the start of another request with this same
                                // request_id
                                Entry::Occupied(_o) => (),
                                Entry::Vacant(v) => {
                                    v.insert(ctx.spawn(future));
                                }
                            };
                        }
                    }
                    GraphQLOverWebSocketMessage::Stop if has_started_value => {
                        let request_id = request.id.unwrap_or("1".to_owned());
                        if let Some(handler) = &self.handler {
                            let context = self.graphql_context.deref();
                            let state = SubscriptionState::OnOperationComplete(context);
                            handler.handle(state).unwrap();
                        }
                        match self.map_req_id_to_spawn_handle.remove(&request_id) {
                            Some(spawn_handle) => {
                                ctx.cancel_future(spawn_handle);
                                ctx.text(Self::gql_complete(&request_id));
                            }
                            None => {
                                // No request with this id was found in progress.
                                // since the Subscription Protocol Spec does not specify
                                // what occurs in this case im just considering the possibility
                                // of send a error.
                            }
                        }
                    }
                    GraphQLOverWebSocketMessage::ConnectionTerminate => {
                        if let Some(handler) = &self.handler {
                            let context = self.graphql_context.deref();
                            let state = SubscriptionState::OnDisconnect(context);
                            handler.handle(state).unwrap();
                        }
                        ctx.stop();
                    }
                    _ => {}
                }
            }
            ws::Message::Close(_) => {
                if let Some(handler) = &self.handler {
                    let context = self.graphql_context.deref();
                    let state = SubscriptionState::OnDisconnect(context);
                    handler.handle(state).unwrap();
                }
                ctx.stop();
            }
            _ => {
                // Non Text messages are not allowed
                ctx.stop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use futures::StreamExt;

    #[actix_rt::test]
    async fn expected_communication() {
        use actix_web::HttpRequest;
        use actix_web_actors::ws::{Frame, Message};
        use futures::{SinkExt, Stream};
        use juniper::{DefaultScalarValue, EmptyMutation, FieldError, RootNode};
        use juniper_subscriptions::Coordinator;
        use std::{pin::Pin, time::Duration};

        pub struct Query;

        #[juniper::graphql_object(Context = Database)]
        impl Query {
            fn hello_world() -> &str {
                "Hello World!"
            }
        }
        type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;
        type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;
        type MyCoordinator = Coordinator<
            'static,
            Query,
            EmptyMutation<Database>,
            Subscription,
            Database,
            DefaultScalarValue,
        >;
        struct Subscription;

        #[derive(Clone)]
        pub struct Database;

        impl juniper::Context for Database {}

        impl Database {
            fn new() -> Self {
                Self {}
            }
        }

        #[juniper::graphql_subscription(Context = Database)]
        impl Subscription {
            async fn hello_world() -> StringStream {
                let mut counter = 0;
                let stream = tokio::time::interval(Duration::from_secs(2)).map(move |_| {
                    counter += 1;
                    if counter % 2 == 0 {
                        Ok(String::from("World!"))
                    } else {
                        Ok(String::from("Hello"))
                    }
                });
                Box::pin(stream)
            }
        }

        let schema: Schema =
            RootNode::new(Query, EmptyMutation::<Database>::new(), Subscription {});

        async fn gql_subscriptions(
            coordinator: web::Data<MyCoordinator>,
            stream: web::Payload,
            req: HttpRequest,
        ) -> Result<HttpResponse, Error> {
            let context = Database::new();
            graphql_subscriptions(
                coordinator,
                context,
                stream,
                req,
                Some(EmptySubscriptionHandler::default()),
                None,
            )
            .await
        }
        let coord = web::Data::new(juniper_subscriptions::Coordinator::new(schema));
        let mut app = test::start(move || {
            App::new()
                .app_data(coord.clone())
                .service(web::resource("/subscriptions").to(gql_subscriptions))
        });
        let mut ws = app.ws_at("/subscriptions").await.unwrap();
        let messages_to_be_sent = vec![
            String::from(r#"{"type":"connection_init","payload":{}}"#),
            String::from(
                r#"{"id":"1","type":"start","payload":{"variables":{},"extensions":{},"operationName":"hello","query":"subscription hello {  helloWorld}"}}"#,
            ),
            String::from(
                r#"{"id":"2","type":"start","payload":{"variables":{},"extensions":{},"operationName":"hello","query":"subscription hello {  helloWorld}"}}"#,
            ),
            String::from(r#"{"id":"1","type":"stop"}"#),
            String::from(r#"{"type":"connection_terminate"}"#),
        ];
        let messages_to_be_received = vec![
            vec![
                Some(bytes::Bytes::from(
                    r#"{"type":"connection_ack", "payload": null }"#,
                )),
                Some(bytes::Bytes::from(r#"{"type":"ka", "payload": null }"#)),
            ],
            vec![Some(bytes::Bytes::from(
                r#"{"type":"data","id":"1","payload":{"data":{"helloWorld":"Hello"}} }"#,
            ))],
            vec![Some(bytes::Bytes::from(
                r#"{"type":"data","id":"2","payload":{"data":{"helloWorld":"Hello"}} }"#,
            ))],
            vec![Some(bytes::Bytes::from(
                r#"{"type":"complete","id":"1","payload":null}"#,
            ))],
            vec![None],
        ];

        for (index, msg_to_be_sent) in messages_to_be_sent.into_iter().enumerate() {
            let expected_msgs = messages_to_be_received.get(index).unwrap();
            ws.send(Message::Text(msg_to_be_sent)).await.unwrap();
            for expected_msg in expected_msgs {
                let (item, ws_stream) = ws.into_future().await;
                ws = ws_stream;
                match expected_msg {
                    Some(expected_msg) => {
                        if let Some(Ok(Frame::Text(msg))) = item {
                            assert_eq!(msg, expected_msg);
                        } else {
                            assert!(false);
                        }
                    }
                    None => assert_eq!(item.is_none(), expected_msg.is_none()),
                }
            }
        }
    }
}
