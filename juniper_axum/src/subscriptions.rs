//! Definitions for handling GraphQL subscriptions.

use std::fmt;

use axum::extract::ws::{self, WebSocket};
use futures::{future, SinkExt as _, StreamExt as _};
use juniper::ScalarValue;
use juniper_graphql_ws::{graphql_transport_ws, graphql_ws, Init, Schema};

/// Serves the [legacy `graphql-ws` GraphQL over WebSocket Protocol][old] on the provided
/// [`WebSocket`].
///
/// > __WARNING__: This function doesn't check or set the `Sec-Websocket-Protocol` HTTP header value
/// >              as `graphql-ws`, this should be done manually outside.
/// >              For fully baked [`axum`] handler for
/// >              [legacy `graphql-ws` GraphQL over WebSocket Protocol][old], use [`graphql_ws()`]
/// >              handler instead.
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
/// # Example
///
/// ```rust
/// use std::{pin::Pin, time::Duration};
///
/// use axum::{
///     extract::WebSocketUpgrade,
///     body::Body,
///     response::Response,
///     routing::get,
///     Extension, Router,
/// };
/// use futures::{Stream, StreamExt as _};
/// use juniper::{
///     graphql_object, graphql_subscription, EmptyMutation, FieldError,
///     RootNode,
/// };
/// use juniper_axum::{playground, subscriptions};
/// use juniper_graphql_ws::ConnectionConfig;
/// use tokio::time::interval;
/// use tokio_stream::wrappers::IntervalStream;
///
/// type Schema = RootNode<'static, Query, EmptyMutation, Subscription>;
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Query;
///
/// #[graphql_object]
/// impl Query {
///     /// Add two numbers a and b
///     fn add(a: i32, b: i32) -> i32 {
///         a + b
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Subscription;
///
/// type NumberStream = Pin<Box<dyn Stream<Item = Result<i32, FieldError>> + Send>>;
///
/// #[graphql_subscription]
/// impl Subscription {
///     /// Count seconds
///     async fn count() -> NumberStream {
///         let mut value = 0;
///         let stream = IntervalStream::new(interval(Duration::from_secs(1))).map(move |_| {
///             value += 1;
///             Ok(value)
///         });
///         Box::pin(stream)
///     }
/// }
///
/// async fn juniper_subscriptions(
///     Extension(schema): Extension<Schema>,
///     ws: WebSocketUpgrade,
/// ) -> Response {
///     ws.protocols(["graphql-ws"])
///         .max_frame_size(1024)
///         .max_message_size(1024)
///         .max_write_buffer_size(100)
///         .on_upgrade(move |socket| {
///             subscriptions::serve_graphql_ws(socket, schema, ConnectionConfig::new(()))
///         })
/// }
///
/// let schema = Schema::new(Query, EmptyMutation::new(), Subscription);
///
/// let app: Router<Body> = Router::new()
///     .route("/subscriptions", get(juniper_subscriptions))
///     .layer(Extension(schema));
/// ```
///
/// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
/// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
pub async fn serve_graphql_ws<S, I>(socket: WebSocket, schema: S, init: I)
where
    S: Schema,
    I: Init<S::ScalarValue, S::Context> + Send,
{
    let (ws_tx, ws_rx) = socket.split();
    let (s_tx, s_rx) = graphql_ws::Connection::new(schema, init).split();

    let input = ws_rx
        .map(|r| r.map(Message))
        .forward(s_tx.sink_map_err(|e| match e {}));

    let output = s_rx
        .map(|msg| {
            Ok(serde_json::to_string(&msg)
                .map(ws::Message::Text)
                .unwrap_or_else(|e| {
                    ws::Message::Close(Some(ws::CloseFrame {
                        code: 1011, // CloseCode::Error
                        reason: format!("error serializing response: {e}").into(),
                    }))
                }))
        })
        .forward(ws_tx);

    // No errors can be returned here, so ignoring is OK.
    _ = future::select(input, output).await;
}

/// Wrapper around [`ws::Message`] allowing to define custom conversions.
#[derive(Debug)]
struct Message(ws::Message);

impl<S: ScalarValue> TryFrom<Message> for graphql_transport_ws::Input<S> {
    type Error = Error;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg.0 {
            ws::Message::Text(text) => serde_json::from_slice(text.as_bytes())
                .map(Self::Message)
                .map_err(Error::Serde),
            ws::Message::Binary(bytes) => serde_json::from_slice(bytes.as_ref())
                .map(Self::Message)
                .map_err(Error::Serde),
            ws::Message::Close(_) => Ok(Self::Close),
            other => Err(Error::UnexpectedClientMessage(other)),
        }
    }
}

impl<S: ScalarValue> TryFrom<Message> for graphql_ws::ClientMessage<S> {
    type Error = Error;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg.0 {
            ws::Message::Text(text) => {
                serde_json::from_slice(text.as_bytes()).map_err(Error::Serde)
            }
            ws::Message::Binary(bytes) => {
                serde_json::from_slice(bytes.as_ref()).map_err(Error::Serde)
            }
            ws::Message::Close(_) => Ok(Self::ConnectionTerminate),
            other => Err(Error::UnexpectedClientMessage(other)),
        }
    }
}

#[derive(Debug)]
enum Error {
    //Axum(axum::Error),
    /// Deserializing of a client [`ws::Message`] failed.
    Serde(serde_json::Error),

    /// Unexpected client [`ws::Message`].
    UnexpectedClientMessage(ws::Message),
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
