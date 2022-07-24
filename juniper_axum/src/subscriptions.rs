use axum::extract::ws::{Message, WebSocket};
use juniper::{
    futures::{SinkExt, StreamExt, TryStreamExt},
    ScalarValue,
};
use juniper_graphql_ws::{ClientMessage, Connection, ConnectionConfig, Schema, WebsocketError};

#[derive(Debug)]
struct AxumMessage(Message);

#[derive(Debug)]
enum SubscriptionError {
    Juniper(WebsocketError),
    Axum(axum::Error),
    Serde(serde_json::Error),
}

impl<S: ScalarValue> TryFrom<AxumMessage> for ClientMessage<S> {
    type Error = serde_json::Error;

    fn try_from(msg: AxumMessage) -> serde_json::Result<Self> {
        serde_json::from_slice(&msg.0.into_data())
    }
}

/// Redirect the axum [`Websocket`] to a juniper [`Connection`] and vice versa.
///
/// # Example
///
///
pub async fn handle_graphql_socket<S: Schema>(socket: WebSocket, schema: S, context: S::Context) {
    let config = ConnectionConfig::new(context);
    let (ws_tx, ws_rx) = socket.split();
    let (juniper_tx, juniper_rx) = Connection::new(schema, config).split();

    // In the following section we make the streams and sinks from
    // Axum and Juniper compatible with each other. This makes it
    // possible to forward an incoming message from Axum to Juniper
    // and vice versa.
    let juniper_tx = juniper_tx.sink_map_err(SubscriptionError::Juniper);

    let send_websocket_message_to_juniper = ws_rx
        .map_err(SubscriptionError::Axum)
        .map(|result| result.map(AxumMessage))
        .forward(juniper_tx);

    let ws_tx = ws_tx.sink_map_err(SubscriptionError::Axum);

    let send_juniper_message_to_axum = juniper_rx
        .map(|msg| serde_json::to_string(&msg).map(Message::Text))
        .map_err(SubscriptionError::Serde)
        .forward(ws_tx);

    // Start listening for messages from axum, and redirect them to juniper
    let _result = futures::future::select(
        send_websocket_message_to_juniper,
        send_juniper_message_to_axum,
    )
    .await;
}
