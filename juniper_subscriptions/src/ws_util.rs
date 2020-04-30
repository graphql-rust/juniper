use juniper::{InputValue, ScalarValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Enum of Subscription Protocol Message Types over WS
/// to know more access [Subscriptions Transport over WS][SubscriptionsTransportWS]
///
/// [SubscriptionsTransportWS]: https://github.com/apollographql/subscriptions-transport-ws/blob/master/PROTOCOL.md
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GraphQLOverWebSocketMessage {
    /// Client -> Server
    /// Client sends this message after plain websocket connection to start the communication
    /// with the server
    #[serde(rename = "connection_init")]
    ConnectionInit,
    /// Server -> Client
    /// The server may responses with this message to the GQL_CONNECTION_INIT from client,
    /// indicates the server accepted the connection.
    #[serde(rename = "connection_ack")]
    ConnectionAck,
    /// Server -> Client
    /// The server may responses with this message to the GQL_CONNECTION_INIT from client,
    /// indicates the server rejected the connection.
    #[serde(rename = "connection_error")]
    ConnectionError,
    /// Server -> Client
    /// Server message that should be sent right after each GQL_CONNECTION_ACK processed
    /// and then periodically to keep the client connection alive.
    #[serde(rename = "ka")]
    ConnectionKeepAlive,
    /// Client -> Server
    /// Client sends this message to terminate the connection.
    #[serde(rename = "connection_terminate")]
    ConnectionTerminate,
    /// Client -> Server
    /// Client sends this message to execute GraphQL operation
    #[serde(rename = "start")]
    Start,
    /// Server -> Client
    /// The server sends this message to transfer the GraphQL execution result from the
    /// server to the client, this message is a response for GQL_START message.
    #[serde(rename = "data")]
    Data,
    /// Server -> Client
    /// Server sends this message upon a failing operation, before the GraphQL execution,
    /// usually due to GraphQL validation errors (resolver errors are part of GQL_DATA message,
    /// and will be added as errors array)
    #[serde(rename = "error")]
    Error,
    /// Server -> Client
    /// Server sends this message to indicate that a GraphQL operation is done,
    /// and no more data will arrive for the specific operation.
    #[serde(rename = "complete")]
    Complete,
    /// Client -> Server
    /// Client sends this message in order to stop a running GraphQL operation execution
    /// (for example: unsubscribe)
    #[serde(rename = "stop")]
    Stop,
}

/// Empty SubscriptionLifeCycleHandler over WS
pub enum SubscriptionState<'a, Context>
where
    Context: Send + Sync,
{
    /// The Subscription is at the init of the connection with the client after the
    /// server receives the GQL_CONNECTION_INIT message.
    OnConnection(Option<Value>, &'a mut Context),
    /// The Subscription is at the start of a operation after the GQL_START message is
    /// is received.
    OnOperation(&'a Context),
    /// The subscription is on the end of a operation before sending the GQL_COMPLETE
    /// message to the client.
    OnOperationComplete(&'a Context),
    /// The Subscription is terminating the connection with the client.
    OnDisconnect(&'a Context),
}

/// Trait based on the SubscriptionServer [LifeCycleEvents][LifeCycleEvents]
///
/// [LifeCycleEvents]: https://www.apollographql.com/docs/graphql-subscriptions/lifecycle-events/
pub trait SubscriptionStateHandler<Context, E>
where
    Context: Send + Sync,
    E: std::error::Error,
{
    /// This function is called when the state of the Subscription changes
    /// with the actual state.
    fn handle(&self, _state: SubscriptionState<Context>) -> Result<(), E>;
}

/// A Empty Subscription Handler
#[derive(Default)]
pub struct EmptySubscriptionHandler;

impl<Context> SubscriptionStateHandler<Context, std::io::Error> for EmptySubscriptionHandler
where
    Context: Send + Sync,
{
    fn handle(&self, _state: SubscriptionState<Context>) -> Result<(), std::io::Error> {
        Ok(())
    }
}

/// Struct defining the message content sent or received by the server
#[derive(Deserialize, Serialize)]
pub struct WsPayload {
    /// ID of the Subscription operation
    pub id: Option<String>,
    /// Type of the Message
    #[serde(rename(deserialize = "type"))]
    pub type_name: GraphQLOverWebSocketMessage,
    /// Payload of the Message
    pub payload: Option<Value>,
}

impl WsPayload {
    /// Returns the transformation from the payload Value to a GraphQLPayload
    pub fn graphql_payload<S>(&self) -> Option<GraphQLPayload<S>>
    where
        S: ScalarValue + Send + Sync + 'static,
    {
        serde_json::from_value(self.payload.clone()?).ok()
    }
    /// Constructor
    pub fn new(
        id: Option<String>,
        type_name: GraphQLOverWebSocketMessage,
        payload: Option<Value>,
    ) -> Self {
        Self {
            id,
            type_name,
            payload,
        }
    }
}

/// GraphQLPayload content sent by the client to the server
#[derive(Debug, Deserialize)]
#[serde(bound = "InputValue<S>: Deserialize<'de>")]
pub struct GraphQLPayload<S>
where
    S: ScalarValue + Send + Sync + 'static,
{
    /// Variables for the Operation
    pub variables: Option<InputValue<S>>,
    /// Extensions
    pub extensions: Option<HashMap<String, String>>,
    /// Name of the Operation to be executed
    #[serde(rename(deserialize = "operationName"))]
    pub operation_name: Option<String>,
    /// Query value of the Operation
    pub query: Option<String>,
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use juniper::DefaultScalarValue;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Default)]
    struct Context {
        pub user_id: Option<String>,
        pub has_connected: bool,
        pub has_operated: AtomicBool,
        pub has_completed_operation: AtomicBool,
        pub has_disconnected: AtomicBool,
    }

    #[derive(Deserialize)]
    struct OnConnPayload {
        #[serde(rename = "userId")]
        pub user_id: Option<String>,
    }

    struct SubStateHandler;

    impl SubscriptionStateHandler<Context, std::io::Error> for SubStateHandler {
        fn handle(&self, state: SubscriptionState<Context>) -> Result<(), std::io::Error> {
            match state {
                SubscriptionState::OnConnection(payload, ctx) => {
                    if let Some(payload) = payload {
                        let result = serde_json::from_value::<OnConnPayload>(payload);
                        if let Ok(payload) = result {
                            ctx.user_id = payload.user_id;
                        }
                    }
                    ctx.has_connected = true;
                }
                SubscriptionState::OnOperation(ctx) => {
                    ctx.has_operated.store(true, Ordering::Relaxed);
                }
                SubscriptionState::OnOperationComplete(ctx) => {
                    ctx.has_completed_operation.store(true, Ordering::Relaxed);
                }
                SubscriptionState::OnDisconnect(ctx) => {
                    ctx.has_disconnected.store(true, Ordering::Relaxed);
                }
            };
            Ok(())
        }
    }

    const SUB_HANDLER: SubStateHandler = SubStateHandler {};

    fn implementation_example(msg: &str, ctx: &mut Context) -> bool {
        let ws_payload: WsPayload = serde_json::from_str(msg).unwrap();
        match ws_payload.type_name {
            GraphQLOverWebSocketMessage::ConnectionInit => {
                let state = SubscriptionState::OnConnection(ws_payload.payload, ctx);
                SUB_HANDLER.handle(state).unwrap();
                true
            }
            GraphQLOverWebSocketMessage::ConnectionTerminate => {
                let state = SubscriptionState::OnDisconnect(ctx);
                SUB_HANDLER.handle(state).unwrap();
                true
            }
            GraphQLOverWebSocketMessage::Start => {
                // Over here you can make usage of the subscriptions coordinator
                // to get the connection related to the client request. This is just a
                // testing example to show and verify usage of this module.
                let _gql_payload: GraphQLPayload<DefaultScalarValue> =
                    ws_payload.graphql_payload().unwrap();
                let state = SubscriptionState::OnOperation(ctx);
                SUB_HANDLER.handle(state).unwrap();
                true
            }
            GraphQLOverWebSocketMessage::Stop => {
                let state = SubscriptionState::OnOperationComplete(ctx);
                SUB_HANDLER.handle(state).unwrap();
                true
            }
            _ => false,
        }
    }

    #[test]
    fn on_connection() {
        let mut ctx = Context::default();
        let type_value =
            serde_json::to_string(&GraphQLOverWebSocketMessage::ConnectionInit).unwrap();
        let msg = format!(
            r#"{{"type":{}, "payload": {{ "userId": "1" }} }}"#,
            type_value
        );
        assert!(implementation_example(&msg, &mut ctx));
        assert!(ctx.has_connected);
        assert_eq!(ctx.user_id, Some(String::from("1")));
    }

    #[test]
    fn on_operation() {
        let mut ctx = Context::default();
        let type_value = serde_json::to_string(&GraphQLOverWebSocketMessage::Start).unwrap();
        let msg = format!(r#"{{"type":{}, "payload": {{}}, "id": "1" }}"#, type_value);
        assert!(implementation_example(&msg, &mut ctx));
        assert!(ctx.has_operated.load(Ordering::Relaxed));
    }

    #[test]
    fn on_operation_completed() {
        let mut ctx = Context::default();
        let type_value = serde_json::to_string(&GraphQLOverWebSocketMessage::Stop).unwrap();
        let msg = format!(r#"{{"type":{}, "payload": null, "id": "1" }}"#, type_value);
        assert!(implementation_example(&msg, &mut ctx));
        let has_completed = ctx.has_completed_operation.load(Ordering::Relaxed);
        assert!(has_completed);
    }

    #[test]
    fn on_disconnect() {
        let mut ctx = Context::default();
        let type_value =
            serde_json::to_string(&GraphQLOverWebSocketMessage::ConnectionTerminate).unwrap();
        let msg = format!(r#"{{"type":{}, "payload": null, "id": "1" }}"#, type_value);
        assert!(implementation_example(&msg, &mut ctx));
        let has_disconnected = ctx.has_disconnected.load(Ordering::Relaxed);
        assert!(has_disconnected);
    }
}
