use actix::{
    Actor, ActorContext, ActorFuture, AsyncContext, Handler, Message, Recipient, SpawnHandle,
    StreamHandler, WrapFuture,
};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::{
    ws,
    ws::{handshake_with_protocols, WebsocketContext},
};
use futures::StreamExt;
use juniper::{http::GraphQLRequest, ScalarValue, SubscriptionCoordinator};
use juniper_subscriptions::ws_util::GraphQLOverWebSocketMessage;
pub use juniper_subscriptions::ws_util::{
    GraphQLPayload, SubscriptionState, SubscriptionStateHandler, WsPayload,
};
use juniper_subscriptions::Coordinator;
use serde::Serialize;
use std::ops::Deref;
use std::{
    collections::HashMap,
    error::Error as StdError,
    sync::Arc,
    time::{Duration, Instant},
};

/// Websocket Subscription Handler
///  
/// # Arguments
/// * `coordinator` - The Subscription Coordinator stored in the App State
/// * `context` - The Context that will be used by the Coordinator
/// * `stream` - The Stream used by the request to create the WebSocket
/// * `req` - The Initial Request sent by the Client
/// * `handler` - The SubscriptionStateHandler implementation that will be used in the Subscription.
/// * `ka_interval` - The Duration that will be used to interleave the keep alive messages sent by the server. The default value is 10 seconds.
pub async fn graphql_subscriptions<Query, Mutation, Subscription, Context, S>(
    actor: GraphQLWSSession<Query, Mutation, Subscription, Context, S>,
    stream: web::Payload,
    req: HttpRequest,
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
{
    let mut res = handshake_with_protocols(&req, &["graphql-ws"])?;
    Ok(res.streaming(WebsocketContext::create(actor, stream)))
}

/// Actor for handling each WS Session
pub struct GraphQLWSSession<Query, Mutation, Subscription, Context, S>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    map_req_id_to_spawn_handle: HashMap<String, SpawnHandle>,
    graphql_context: Arc<Context>,
    coordinator: Arc<Coordinator<'static, Query, Mutation, Subscription, Context, S>>,
    handler: Option<Box<dyn SubscriptionStateHandler<Context> + 'static + std::marker::Unpin>>,
    hb: Instant,
}

impl<Query, Mutation, Subscription, Context, S> Actor
    for GraphQLWSSession<Query, Mutation, Subscription, Context, S>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    type Context =
        ws::WebsocketContext<GraphQLWSSession<Query, Mutation, Subscription, Context, S>>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

/// Internal Struct for handling Messages received from the subscriptions
#[derive(Message)]
#[rtype(result = "()")]
struct Msg(pub Option<String>);

impl<Query, Mutation, Subscription, Context, S> Handler<Msg>
    for GraphQLWSSession<Query, Mutation, Subscription, Context, S>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    type Result = ();
    fn handle(&mut self, msg: Msg, ctx: &mut Self::Context) {
        match msg.0 {
            Some(msg) => ctx.text(msg),
            None => ctx.close(None),
        }
    }
}

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[allow(dead_code)]
impl<Query, Mutation, Subscription, Context, S>
    GraphQLWSSession<Query, Mutation, Subscription, Context, S>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    /// Creates a instance for usage in the graphql_subscription endpoint
    pub fn new(
        coord: Arc<Coordinator<'static, Query, Mutation, Subscription, Context, S>>,
        ctx: Context,
    ) -> Self {
        Self {
            coordinator: coord,
            graphql_context: Arc::new(ctx),
            map_req_id_to_spawn_handle: HashMap::new(),
            handler: None,
            hb: Instant::now(),
        }
    }

    /// Inserts a SubscriptionStateHandler in the Session
    pub fn with_handler<H>(self, handler: H) -> Self
    where
        H: SubscriptionStateHandler<Context> + 'static + std::marker::Unpin,
    {
        Self {
            handler: Some(Box::new(handler)),
            ..self
        }
    }

    fn gql_connection_ack() -> String {
        let value = serde_json::json!({ "type": GraphQLOverWebSocketMessage::ConnectionAck });
        serde_json::to_string(&value).unwrap()
    }

    fn gql_connection_error() -> String {
        let value = serde_json::json!({
            "type": GraphQLOverWebSocketMessage::ConnectionError,
        });
        serde_json::to_string(&value).unwrap()
    }
    fn gql_error<T: StdError + Serialize>(request_id: &String, err: T) -> String {
        let value = serde_json::json!({
            "type": GraphQLOverWebSocketMessage::Error,
            "id": request_id,
            "payload": err
        });
        serde_json::to_string(&value).unwrap()
    }

    fn gql_data<T: Serialize>(request_id: &String, payload: T) -> String {
        let value = serde_json::json!({
            "type": GraphQLOverWebSocketMessage::Data,
            "id": request_id,
            "payload": payload
        });
        serde_json::to_string(&value).unwrap()
    }

    fn gql_complete(request_id: &String) -> String {
        let value = serde_json::json!({
            "type": GraphQLOverWebSocketMessage::Complete,
            "id": request_id,
        });
        serde_json::to_string(&value).unwrap()
    }

    fn starting_subscription(
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
            let _ = addr.do_send(Msg(Some(Self::gql_data(&request_id, response))));
        }
        let _ = addr.do_send(Msg(Some(Self::gql_complete(&request_id))));
    }

    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

impl<Query, Mutation, Subscription, Context, S>
    StreamHandler<Result<ws::Message, ws::ProtocolError>>
    for GraphQLWSSession<Query, Mutation, Subscription, Context, S>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static + std::marker::Unpin,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
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
                            if let Err(_) = on_connect_result {
                                ctx.text(Self::gql_connection_error());
                                ctx.stop();
                                return;
                            }
                        }
                        ctx.text(Self::gql_connection_ack());
                    }
                    GraphQLOverWebSocketMessage::Start => {
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
                            handler.as_ref().handle(state).unwrap();
                        }
                        let context = self.graphql_context.clone();
                        {
                            use std::collections::hash_map::Entry;
                            let req_id = request_id.clone();
                            let future =
                                async move { (graphql_request, req_id, context, coordinator) }
                                    .into_actor(self)
                                    .then(Self::starting_subscription);
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
                    GraphQLOverWebSocketMessage::Stop => {
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
            ws::Message::Binary(_) | ws::Message::Close(_) | ws::Message::Continuation(_) => {
                if let Some(handler) = &self.handler {
                    let context = self.graphql_context.deref();
                    let state = SubscriptionState::OnDisconnect(context);
                    handler.handle(state).unwrap();
                }
                ctx.stop();
            }
            ws::Message::Nop => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::HttpRequest;
    use actix_web::{test, App};
    use actix_web_actors::ws::{Frame, Message};
    use futures::StreamExt;
    use futures::{SinkExt, Stream};
    use juniper::{
        tests::model::Database, tests::schema::Query, DefaultScalarValue, EmptyMutation,
        FieldError, RootNode,
    };
    use juniper_subscriptions::Coordinator;
    use std::{pin::Pin, time::Duration};
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

    async fn gql_subscriptions(
        coordinator: web::Data<MyCoordinator>,
        stream: web::Payload,
        req: HttpRequest,
    ) -> Result<HttpResponse, Error> {
        let context = Database::new();
        let actor = GraphQLWSSession::new(coordinator.into_inner(), context);
        graphql_subscriptions(actor, stream, req).await
    }

    fn test_server() -> test::TestServer {
        let schema: Schema =
            RootNode::new(Query, EmptyMutation::<Database>::new(), Subscription {});

        let coord = web::Data::new(juniper_subscriptions::Coordinator::new(schema));
        test::start(move || {
            App::new()
                .app_data(coord.clone())
                .service(web::resource("/subscriptions").to(gql_subscriptions))
        })
    }

    fn received_msg(msg: &'static str) -> Option<bytes::Bytes> {
        Some(bytes::Bytes::from(msg))
    }

    async fn test_subscription(
        msgs_to_send: Vec<&str>,
        msgs_to_receive: Vec<Vec<Option<bytes::Bytes>>>,
    ) {
        let mut app = test_server();
        let mut ws = app.ws_at("/subscriptions").await.unwrap();
        for (index, msg_to_be_sent) in msgs_to_send.into_iter().enumerate() {
            let expected_msgs = msgs_to_receive.get(index).unwrap();
            ws.send(Message::Text(msg_to_be_sent.to_string()))
                .await
                .unwrap();
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

    #[actix_rt::test]
    async fn basic_connection() {
        let msgs_to_send = vec![
            r#"{"type":"connection_init","payload":{}}"#,
            r#"{"type":"connection_terminate"}"#,
        ];
        let msgs_to_receive = vec![
            vec![received_msg(r#"{"type":"connection_ack"}"#)],
            vec![None],
        ];
        test_subscription(msgs_to_send, msgs_to_receive).await;
    }

    #[actix_rt::test]
    async fn basic_subscription() {
        let msgs_to_send = vec![
            r#"{"type":"connection_init","payload":{}}"#,
            r#"{"id":"1","type":"start","payload":{"variables":{},"extensions":{},"operationName":"hello","query":"subscription hello {  helloWorld}"}}"#,
            r#"{"type":"connection_terminate"}"#,
        ];
        let msgs_to_receive = vec![
            vec![received_msg(r#"{"type":"connection_ack"}"#)],
            vec![received_msg(
                r#"{"type":"data","id":"1","payload":{"data":{"helloWorld":"Hello"}}}"#,
            )],
            vec![None],
        ];
        test_subscription(msgs_to_send, msgs_to_receive).await;
    }

    #[actix_rt::test]
    async fn conn_with_two_subscriptions() {
        let msgs_to_send = vec![
            r#"{"type":"connection_init","payload":{}}"#,
            r#"{"id":"1","type":"start","payload":{"variables":{},"extensions":{},"operationName":"hello","query":"subscription hello {  helloWorld}"}}"#,
            r#"{"id":"2","type":"start","payload":{"variables":{},"extensions":{},"operationName":"hello","query":"subscription hello {  helloWorld}"}}"#,
            r#"{"id":"1","type":"stop"}"#,
            r#"{"type":"connection_terminate"}"#,
        ];
        let msgs_to_receive = vec![
            vec![received_msg(r#"{"type":"connection_ack"}"#)],
            vec![received_msg(
                r#"{"type":"data","id":"1","payload":{"data":{"helloWorld":"Hello"}}}"#,
            )],
            vec![received_msg(
                r#"{"type":"data","id":"2","payload":{"data":{"helloWorld":"Hello"}}}"#,
            )],
            vec![received_msg(r#"{"type":"complete","id":"1"}"#)],
            vec![None],
        ];
        test_subscription(msgs_to_send, msgs_to_receive).await;
    }
}
