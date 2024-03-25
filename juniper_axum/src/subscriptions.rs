//! Definitions for handling GraphQL subscriptions.

use std::fmt;

use axum::{
    extract::{
        ws::{self, WebSocket, WebSocketUpgrade},
        Extension,
    },
    response::Response,
};
use futures::{future, SinkExt as _, StreamExt as _};
use juniper::ScalarValue;
use juniper_graphql_ws::{graphql_transport_ws, graphql_ws, Init, Schema};

/// Creates a [`Handler`] with the specified [`Schema`], which will serve either the
/// [legacy `graphql-ws` GraphQL over WebSocket Protocol][old] or the
/// [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], by auto-selecting between
/// them, based on the `Sec-Websocket-Protocol` HTTP header value.
///
/// > __NOTE__: This is a ready-to-go default [`Handler`] for serving GraphQL over WebSocket
/// >           Protocol. If you need to customize it (for example, configure [`WebSocketUpgrade`]
/// >           parameters), create your own [`Handler`] invoking the [`serve_ws()`] function (see
/// >           its documentation for examples).
///
/// [`Schema`] is [`extract`]ed from [`Extension`]s.
///
/// The `init` argument is used to provide the custom [`juniper::Context`] and additional
/// configuration for connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the
/// context and configuration are already known, or it can be a closure that gets executed
/// asynchronously whenever a client sends the subscription initialization message. Using a
/// closure allows to perform an authentication based on the parameters provided by a client.
///
/// # Example
///
/// ```rust
/// use std::{sync::Arc, time::Duration};
///
/// use axum::{routing::get, Extension, Router};
/// use futures::stream::{BoxStream, StreamExt as _};
/// use juniper::{
///     graphql_object, graphql_subscription, EmptyMutation, FieldError,
///     RootNode,
/// };
/// use juniper_axum::{playground, subscriptions};
/// use juniper_graphql_ws::ConnectionConfig;
/// use tokio::time::interval;
/// use tokio_stream::wrappers::IntervalStream;
///
/// type Schema = RootNode<Query, EmptyMutation, Subscription>;
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Query;
///
/// #[graphql_object]
/// impl Query {
///     /// Adds two `a` and `b` numbers.
///     fn add(a: i32, b: i32) -> i32 {
///         a + b
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Subscription;
///
/// type NumberStream = BoxStream<'static, Result<i32, FieldError>>;
///
/// #[graphql_subscription]
/// impl Subscription {
///     /// Counts seconds.
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
/// let schema = Schema::new(Query, EmptyMutation::new(), Subscription);
///
/// let app: Router = Router::new()
///     .route("/subscriptions", get(subscriptions::ws::<Arc<Schema>>(ConnectionConfig::new(()))))
///     .layer(Extension(Arc::new(schema)));
/// ```
///
/// [`extract`]: axum::extract
/// [`Handler`]: axum::handler::Handler
/// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
/// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
pub fn ws<S: Schema>(
    init: impl Init<S::ScalarValue, S::Context> + Clone + Send,
) -> impl FnOnce(Extension<S>, WebSocketUpgrade) -> future::Ready<Response> + Clone + Send {
    move |Extension(schema), ws| {
        future::ready(
            ws.protocols(["graphql-transport-ws", "graphql-ws"])
                .on_upgrade(move |socket| serve_ws(socket, schema, init)),
        )
    }
}

/// Creates a [`Handler`] with the specified [`Schema`], which will serve the
/// [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new].
///
/// > __NOTE__: This is a ready-to-go default [`Handler`] for serving the
/// >           [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new]. If you need to
/// >           customize it (for example, configure [`WebSocketUpgrade`] parameters), create your
/// >           own [`Handler`] invoking the [`serve_graphql_transport_ws()`] function (see its
/// >           documentation for examples).
///
/// [`Schema`] is [`extract`]ed from [`Extension`]s.
///
/// The `init` argument is used to provide the context and additional configuration for
/// connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the context and
/// configuration are already known, or it can be a closure that gets executed asynchronously
/// when the client sends the `ConnectionInit` message. Using a closure allows to perform an
/// authentication based on the parameters provided by a client.
///
/// # Example
///
/// ```rust
/// use std::{sync::Arc, time::Duration};
///
/// use axum::{routing::get, Extension, Router};
/// use futures::stream::{BoxStream, StreamExt as _};
/// use juniper::{
///     graphql_object, graphql_subscription, EmptyMutation, FieldError,
///     RootNode,
/// };
/// use juniper_axum::{playground, subscriptions};
/// use juniper_graphql_ws::ConnectionConfig;
/// use tokio::time::interval;
/// use tokio_stream::wrappers::IntervalStream;
///
/// type Schema = RootNode<Query, EmptyMutation, Subscription>;
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Query;
///
/// #[graphql_object]
/// impl Query {
///     /// Adds two `a` and `b` numbers.
///     fn add(a: i32, b: i32) -> i32 {
///         a + b
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Subscription;
///
/// type NumberStream = BoxStream<'static, Result<i32, FieldError>>;
///
/// #[graphql_subscription]
/// impl Subscription {
///     /// Counts seconds.
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
/// let schema = Schema::new(Query, EmptyMutation::new(), Subscription);
///
/// let app: Router = Router::new()
///     .route(
///         "/subscriptions",
///         get(subscriptions::graphql_transport_ws::<Arc<Schema>>(ConnectionConfig::new(()))),
///     )
///     .layer(Extension(Arc::new(schema)));
/// ```
///
/// [`extract`]: axum::extract
/// [`Handler`]: axum::handler::Handler
/// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
pub fn graphql_transport_ws<S: Schema>(
    init: impl Init<S::ScalarValue, S::Context> + Clone + Send,
) -> impl FnOnce(Extension<S>, WebSocketUpgrade) -> future::Ready<Response> + Clone + Send {
    move |Extension(schema), ws| {
        future::ready(
            ws.protocols(["graphql-transport-ws"])
                .on_upgrade(move |socket| serve_graphql_transport_ws(socket, schema, init)),
        )
    }
}

/// Creates a [`Handler`] with the specified [`Schema`], which will serve the
/// [legacy `graphql-ws` GraphQL over WebSocket Protocol][old].
///
/// > __NOTE__: This is a ready-to-go default [`Handler`] for serving the
/// >           [legacy `graphql-ws` GraphQL over WebSocket Protocol][old]. If you need to customize
/// >           it (for example, configure [`WebSocketUpgrade`] parameters), create your own
/// >           [`Handler`] invoking the [`serve_graphql_ws()`] function (see its documentation for
/// >           examples).
///
/// [`Schema`] is [`extract`]ed from [`Extension`]s.
///
/// The `init` argument is used to provide the context and additional configuration for
/// connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the context and
/// configuration are already known, or it can be a closure that gets executed asynchronously
/// when the client sends the `GQL_CONNECTION_INIT` message. Using a closure allows to perform
/// an authentication based on the parameters provided by a client.
///
/// > __WARNING__: This protocol has been deprecated in favor of the
/// >              [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], which is
/// >              provided by the [`graphql_transport_ws()`] function.
///
/// # Example
///
/// ```rust
/// use std::{sync::Arc, time::Duration};
///
/// use axum::{routing::get, Extension, Router};
/// use futures::stream::{BoxStream, StreamExt as _};
/// use juniper::{
///     graphql_object, graphql_subscription, EmptyMutation, FieldError,
///     RootNode,
/// };
/// use juniper_axum::{playground, subscriptions};
/// use juniper_graphql_ws::ConnectionConfig;
/// use tokio::time::interval;
/// use tokio_stream::wrappers::IntervalStream;
///
/// type Schema = RootNode<Query, EmptyMutation, Subscription>;
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Query;
///
/// #[graphql_object]
/// impl Query {
///     /// Adds two `a` and `b` numbers.
///     fn add(a: i32, b: i32) -> i32 {
///         a + b
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Subscription;
///
/// type NumberStream = BoxStream<'static, Result<i32, FieldError>>;
///
/// #[graphql_subscription]
/// impl Subscription {
///     /// Counts seconds.
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
/// let schema = Schema::new(Query, EmptyMutation::new(), Subscription);
///
/// let app: Router = Router::new()
///     .route(
///         "/subscriptions",
///         get(subscriptions::graphql_ws::<Arc<Schema>>(ConnectionConfig::new(()))),
///     )
///     .layer(Extension(Arc::new(schema)));
/// ```
///
/// [`extract`]: axum::extract
/// [`Handler`]: axum::handler::Handler
/// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
/// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
pub fn graphql_ws<S: Schema>(
    init: impl Init<S::ScalarValue, S::Context> + Clone + Send,
) -> impl FnOnce(Extension<S>, WebSocketUpgrade) -> future::Ready<Response> + Clone + Send {
    move |Extension(schema), ws| {
        future::ready(
            ws.protocols(["graphql-ws"])
                .on_upgrade(move |socket| serve_graphql_ws(socket, schema, init)),
        )
    }
}

/// Serves on the provided [`WebSocket`] by auto-selecting between the
/// [legacy `graphql-ws` GraphQL over WebSocket Protocol][old] and the
/// [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], based on the
/// `Sec-Websocket-Protocol` HTTP header value.
///
/// > __WARNING__: This function doesn't set (only checks) the `Sec-Websocket-Protocol` HTTP header
/// >              value, so this should be done manually outside (see the example below).
/// >              To have fully baked [`axum`] handler, use [`ws()`] handler instead.
///
/// The `init` argument is used to provide the custom [`juniper::Context`] and additional
/// configuration for connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the
/// context and configuration are already known, or it can be a closure that gets executed
/// asynchronously whenever a client sends the subscription initialization message. Using a
/// closure allows to perform an authentication based on the parameters provided by a client.
///
/// # Example
///
/// ```rust
/// use std::{sync::Arc, time::Duration};
///
/// use axum::{
///     extract::WebSocketUpgrade,
///     response::Response,
///     routing::get,
///     Extension, Router,
/// };
/// use futures::stream::{BoxStream, StreamExt as _};
/// use juniper::{
///     graphql_object, graphql_subscription, EmptyMutation, FieldError,
///     RootNode,
/// };
/// use juniper_axum::{playground, subscriptions};
/// use juniper_graphql_ws::ConnectionConfig;
/// use tokio::time::interval;
/// use tokio_stream::wrappers::IntervalStream;
///
/// type Schema = RootNode<Query, EmptyMutation, Subscription>;
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Query;
///
/// #[graphql_object]
/// impl Query {
///     /// Adds two `a` and `b` numbers.
///     fn add(a: i32, b: i32) -> i32 {
///         a + b
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Subscription;
///
/// type NumberStream = BoxStream<'static, Result<i32, FieldError>>;
///
/// #[graphql_subscription]
/// impl Subscription {
///     /// Counts seconds.
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
///     Extension(schema): Extension<Arc<Schema>>,
///     ws: WebSocketUpgrade,
/// ) -> Response {
///     ws.protocols(["graphql-transport-ws", "graphql-ws"])
///         .max_frame_size(1024)
///         .max_message_size(1024)
///         .max_write_buffer_size(100)
///         .on_upgrade(move |socket| {
///             subscriptions::serve_ws(socket, schema, ConnectionConfig::new(()))
///         })
/// }
///
/// let schema = Schema::new(Query, EmptyMutation::new(), Subscription);
///
/// let app: Router = Router::new()
///     .route("/subscriptions", get(juniper_subscriptions))
///     .layer(Extension(Arc::new(schema)));
/// ```
///
/// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
/// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
pub async fn serve_ws<S, I>(socket: WebSocket, schema: S, init: I)
where
    S: Schema,
    I: Init<S::ScalarValue, S::Context> + Send,
{
    if socket.protocol().map(AsRef::as_ref) == Some("graphql-ws".as_bytes()) {
        serve_graphql_ws(socket, schema, init).await;
    } else {
        serve_graphql_transport_ws(socket, schema, init).await;
    }
}

/// Serves the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new] on the provided
/// [`WebSocket`].
///
/// > __WARNING__: This function doesn't check or set the `Sec-Websocket-Protocol` HTTP header value
/// >              as `graphql-transport-ws`, so this should be done manually outside (see the
/// >              example below).
/// >              To have fully baked [`axum`] handler for
/// >              [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], use
/// >              [`graphql_transport_ws()`] handler instead.
///
/// The `init` argument is used to provide the context and additional configuration for
/// connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the context and
/// configuration are already known, or it can be a closure that gets executed asynchronously
/// when the client sends the `ConnectionInit` message. Using a closure allows to perform an
/// authentication based on the parameters provided by a client.
///
/// # Example
///
/// ```rust
/// use std::{sync::Arc, time::Duration};
///
/// use axum::{
///     extract::WebSocketUpgrade,
///     response::Response,
///     routing::get,
///     Extension, Router,
/// };
/// use futures::stream::{BoxStream, StreamExt as _};
/// use juniper::{
///     graphql_object, graphql_subscription, EmptyMutation, FieldError,
///     RootNode,
/// };
/// use juniper_axum::{playground, subscriptions};
/// use juniper_graphql_ws::ConnectionConfig;
/// use tokio::time::interval;
/// use tokio_stream::wrappers::IntervalStream;
///
/// type Schema = RootNode<Query, EmptyMutation, Subscription>;
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Query;
///
/// #[graphql_object]
/// impl Query {
///     /// Adds two `a` and `b` numbers.
///     fn add(a: i32, b: i32) -> i32 {
///         a + b
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Subscription;
///
/// type NumberStream = BoxStream<'static, Result<i32, FieldError>>;
///
/// #[graphql_subscription]
/// impl Subscription {
///     /// Counts seconds.
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
///     Extension(schema): Extension<Arc<Schema>>,
///     ws: WebSocketUpgrade,
/// ) -> Response {
///     ws.protocols(["graphql-transport-ws"])
///         .max_frame_size(1024)
///         .max_message_size(1024)
///         .max_write_buffer_size(100)
///         .on_upgrade(move |socket| {
///             subscriptions::serve_graphql_transport_ws(socket, schema, ConnectionConfig::new(()))
///         })
/// }
///
/// let schema = Schema::new(Query, EmptyMutation::new(), Subscription);
///
/// let app: Router = Router::new()
///     .route("/subscriptions", get(juniper_subscriptions))
///     .layer(Extension(Arc::new(schema)));
/// ```
///
/// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
pub async fn serve_graphql_transport_ws<S, I>(socket: WebSocket, schema: S, init: I)
where
    S: Schema,
    I: Init<S::ScalarValue, S::Context> + Send,
{
    let (ws_tx, ws_rx) = socket.split();
    let (s_tx, s_rx) = graphql_transport_ws::Connection::new(schema, init).split();

    let input = ws_rx
        .map(|r| r.map(Message))
        .forward(s_tx.sink_map_err(|e| match e {}));

    let output = s_rx
        .map(|output| {
            Ok(match output {
                graphql_transport_ws::Output::Message(msg) => {
                    serde_json::to_string(&msg)
                        .map(ws::Message::Text)
                        .unwrap_or_else(|e| {
                            ws::Message::Close(Some(ws::CloseFrame {
                                code: 1011, // CloseCode::Error
                                reason: format!("error serializing response: {e}").into(),
                            }))
                        })
                }
                graphql_transport_ws::Output::Close { code, message } => {
                    ws::Message::Close(Some(ws::CloseFrame {
                        code,
                        reason: message.into(),
                    }))
                }
            })
        })
        .forward(ws_tx);

    // No errors can be returned here, so ignoring is OK.
    _ = future::select(input, output).await;
}

/// Serves the [legacy `graphql-ws` GraphQL over WebSocket Protocol][old] on the provided
/// [`WebSocket`].
///
/// > __WARNING__: This function doesn't check or set the `Sec-Websocket-Protocol` HTTP header value
/// >              as `graphql-ws`, so this should be done manually outside (see the example below).
/// >              To have fully baked [`axum`] handler for
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
/// >              [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], which is
/// >              provided by the [`serve_graphql_transport_ws()`] function.
///
/// # Example
///
/// ```rust
/// use std::{sync::Arc, time::Duration};
///
/// use axum::{
///     extract::WebSocketUpgrade,
///     response::Response,
///     routing::get,
///     Extension, Router,
/// };
/// use futures::stream::{BoxStream, StreamExt as _};
/// use juniper::{
///     graphql_object, graphql_subscription, EmptyMutation, FieldError,
///     RootNode,
/// };
/// use juniper_axum::{playground, subscriptions};
/// use juniper_graphql_ws::ConnectionConfig;
/// use tokio::time::interval;
/// use tokio_stream::wrappers::IntervalStream;
///
/// type Schema = RootNode<Query, EmptyMutation, Subscription>;
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Query;
///
/// #[graphql_object]
/// impl Query {
///     /// Adds two `a` and `b` numbers.
///     fn add(a: i32, b: i32) -> i32 {
///         a + b
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// pub struct Subscription;
///
/// type NumberStream = BoxStream<'static, Result<i32, FieldError>>;
///
/// #[graphql_subscription]
/// impl Subscription {
///     /// Counts seconds.
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
///     Extension(schema): Extension<Arc<Schema>>,
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
/// let app: Router = Router::new()
///     .route("/subscriptions", get(juniper_subscriptions))
///     .layer(Extension(Arc::new(schema)));
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

/// Possible errors of serving a [`WebSocket`] connection.
#[derive(Debug)]
enum Error {
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
