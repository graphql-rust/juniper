//! Implementation of the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], as now
//! used by [Apollo] and [`graphql-ws` npm package].
//!
//! Implementation of the [legacy `graphql-ws` GraphQL over WebSocket Protocol][old] may be found in
//! the [`graphql_ws` module].
//!
//! [`graphql_ws` module]: crate::graphql_ws
//! [`graphql-ws` npm package]: https://npmjs.com/package/graphql-ws
//! [Apollo]: https://www.apollographql.com
//! [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
//! [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md

mod client_message;
mod server_message;

use std::{
    collections::HashMap, convert::Infallible, error::Error, mem, panic::AssertUnwindSafe,
    pin::Pin, sync::Arc, task::ready, time::Duration,
};

use derive_more::with_trait::From;
use juniper::{
    GraphQLError, OperationType, RuleError, ScalarValue, ValidatedOperation, Value,
    futures::{
        Sink, Stream,
        channel::oneshot,
        future::{self, BoxFuture, Either, FutureExt as _},
        stream::{self, BoxStream, SelectAll, StreamExt as _},
        task::{Context, Poll, Waker},
    },
};
use tokio::{sync::Notify, time};

use super::{ConnectionConfig, Init, Schema};

pub use self::{
    client_message::{ClientMessage, SubscribePayload},
    server_message::{ErrorPayload, NextPayload, ServerMessage},
};

struct ExecutionParams<S: Schema> {
    subscribe_payload: SubscribePayload<S::ScalarValue>,
    config: ConnectionConfig<S::Context, S::ScalarValue>,
    schema: S,
}

/// Possible inputs received from a client.
#[derive(Debug, From)]
pub enum Input<S> {
    /// Deserialized [`ClientMessage`].
    Message(ClientMessage<S>),

    /// Client initiated normal closing of a [`Connection`].
    Close,
}

/// Output provides the responses that should be sent to the client.
#[derive(Debug, PartialEq)]
pub enum Output<S: ScalarValue> {
    /// Message is a message that should be serialized and sent to the client.
    Message(ServerMessage<S>),
    /// Close indicates that the connection should be closed and provides a code and message to
    /// send to the client. This is always the last message in the output stream.
    Close {
        /// The WebSocket code that should be sent.
        code: u16,

        /// A message describing the reason for the connection closing.
        message: String,
    },
}

impl<S: ScalarValue + Send> Output<S> {
    /// Wraps this [`Output`] into a singe-item [`Stream`].
    fn into_stream(self) -> stream::Once<future::Ready<Self>> {
        stream::once(future::ready(self))
    }
}

enum ConnectionState<S: Schema, I: Init<S::ScalarValue, S::Context>> {
    /// PreInit is the state before a ConnectionInit message has been accepted.
    PreInit { init: I, schema: S },
    /// Active is the state after a ConnectionInit message has been accepted.
    Active {
        config: ConnectionConfig<S::Context, S::ScalarValue>,
        stoppers: HashMap<String, oneshot::Sender<()>>,
        ping: Arc<Notify>,
        schema: S,
    },
    /// Terminated is the state after a ConnectionInit message has been rejected.
    Terminated,
}

impl<S: Schema, I: Init<S::ScalarValue, S::Context>> ConnectionState<S, I> {
    // Each message we receive results in a stream of zero or more reactions. For example, a
    // Ping message results in a one-item stream with the Pong message reaction.
    async fn handle_message(
        self,
        msg: ClientMessage<S::ScalarValue>,
    ) -> (Self, BoxStream<'static, Output<S::ScalarValue>>) {
        match self {
            Self::PreInit { init, schema } => match msg {
                ClientMessage::ConnectionInit { payload } => match init.init(payload).await {
                    Ok(config) => {
                        let keep_alive_interval = config.keep_alive.interval;
                        let keep_alive_timeout = config.keep_alive.timeout;

                        let ping = Arc::new(Notify::new());

                        let s = Output::Message(ServerMessage::ConnectionAck).into_stream();

                        let s = if keep_alive_interval > Duration::from_secs(0) {
                            s.chain(Output::Message(ServerMessage::Pong).into_stream())
                                .chain(stream::repeat(()).then(move |()| {
                                    tokio::time::sleep(keep_alive_interval)
                                        .map(|()| Output::Message(ServerMessage::Pong))
                                }))
                                .right_stream()
                        } else {
                            s.left_stream()
                        };

                        let s = if keep_alive_timeout > Duration::from_secs(0) {
                            let ping_rx = ping.clone();
                            stream::select_all([
                                s.boxed(),
                                stream::repeat(())
                                    .then(move |()| {
                                        let ping_rx = ping_rx.clone();
                                        async move {
                                            time::timeout(keep_alive_timeout, ping_rx.notified())
                                                .await
                                                .is_err()
                                                .then(|| Output::Close {
                                                    code: 1000,
                                                    message: "Connection lost unexpectedly".into(),
                                                })
                                        }
                                    })
                                    .filter_map(future::ready)
                                    .boxed(),
                            ])
                            .boxed()
                        } else {
                            s.boxed()
                        };

                        (
                            Self::Active {
                                config,
                                stoppers: HashMap::new(),
                                ping,
                                schema,
                            },
                            s,
                        )
                    }
                    Err(e) => (
                        Self::Terminated,
                        Output::Close {
                            code: 4403,
                            message: e.to_string(),
                        }
                        .into_stream()
                        .boxed(),
                    ),
                },
                ClientMessage::Ping { .. } => (
                    Self::PreInit { init, schema },
                    Output::Message(ServerMessage::Pong).into_stream().boxed(),
                ),
                ClientMessage::Subscribe { .. } => (
                    Self::PreInit { init, schema },
                    Output::Close {
                        code: 4401,
                        message: "Unauthorized".to_string(),
                    }
                    .into_stream()
                    .boxed(),
                ),
                _ => (Self::PreInit { init, schema }, stream::empty().boxed()),
            },
            Self::Active {
                config,
                mut stoppers,
                ping,
                schema,
            } => {
                let reactions = match msg {
                    ClientMessage::Subscribe { id, payload } => {
                        // Prune stoppers which streams are already completed or canceled.
                        stoppers.retain(|_, tx| !tx.is_canceled());

                        if stoppers.contains_key(&id) {
                            // We already have an operation with this id. We must close the connection.
                            Output::Close {
                                code: 4409,
                                message: format!("Subscriber for {id} already exists"),
                            }
                            .into_stream()
                            .boxed()
                        } else if config.max_in_flight_operations > 0
                            && stoppers.len() >= config.max_in_flight_operations
                        {
                            // Too many in-flight operations. Just send back a validation error.
                            stream::iter(vec![
                                Output::Message(ServerMessage::Error {
                                    id: id.clone(),
                                    payload: GraphQLError::from(RuleError::new(
                                        "Too many in-flight operations.",
                                        &[],
                                    ))
                                    .into(),
                                }),
                                Output::Message(ServerMessage::Complete { id }),
                            ])
                            .boxed()
                        } else {
                            // Create a channel that we can use to cancel the operation.
                            let (tx, rx) = oneshot::channel::<()>();
                            stoppers.insert(id.clone(), tx);

                            // Create the operation stream. This stream will emit Next and Error
                            // messages, but will not emit Complete – that part is up to us.
                            let s = Self::start(
                                id.clone(),
                                ExecutionParams {
                                    subscribe_payload: payload,
                                    config: config.clone(),
                                    schema: schema.clone(),
                                },
                            )
                            .into_stream()
                            .flatten();

                            // Combine this with our oneshot channel so that the stream ends if the
                            // oneshot is ever fired.
                            let s = stream::unfold((rx, s.boxed()), async |(rx, mut s)| {
                                let next = match future::select(rx, s.next()).await {
                                    Either::Left(_) => None,
                                    Either::Right((r, rx)) => r.map(|r| (r, rx)),
                                };
                                next.map(|(r, rx)| (r, (rx, s)))
                            });

                            // Once the stream ends, send the Complete message.
                            let s = s.chain(
                                Output::Message(ServerMessage::Complete { id }).into_stream(),
                            );

                            s.boxed()
                        }
                    }
                    ClientMessage::Complete { id } => {
                        stoppers.remove(&id);
                        stream::empty().boxed()
                    }
                    ClientMessage::Ping { .. } => {
                        ping.notify_waiters();
                        Output::Message(ServerMessage::Pong).into_stream().boxed()
                    }
                    _ => stream::empty().boxed(),
                };
                (
                    Self::Active {
                        config,
                        stoppers,
                        ping,
                        schema,
                    },
                    reactions,
                )
            }
            Self::Terminated => (self, stream::empty().boxed()),
        }
    }

    async fn start(
        id: String,
        params: ExecutionParams<S>,
    ) -> BoxStream<'static, Output<S::ScalarValue>> {
        let params = Arc::new(params);

        let fut = async {
            // SAFETY: The heap allocation of `params` neither moves nor changes. The borrows of
            //         `operation` end in this function before `params` drops, or move into a
            //         `SubscriptionStart` together with a clone of `params`.
            let borrowed: &'static ExecutionParams<S> = unsafe { &*Arc::as_ptr(&params) };
            let operation = match ValidatedOperation::new(
                &borrowed.subscribe_payload.query,
                borrowed.subscribe_payload.operation_name.as_deref(),
                borrowed.schema.root_node(),
            ) {
                Ok(operation) => operation,
                Err(e) => {
                    return Output::Message(ServerMessage::Error {
                        id: id.clone(),
                        payload: e.into(),
                    })
                    .into_stream()
                    .left_stream();
                }
            };
            if operation.operation_type() == OperationType::Subscription {
                // SAFETY: `operation` borrows only from the heap allocation of `params`.
                return unsafe { SubscriptionStart::new(id.clone(), params.clone(), operation) }
                    .right_stream();
            }
            match operation
                .execute(&params.subscribe_payload.variables, &params.config.context)
                .await
            {
                Ok((data, errors)) => Output::Message(ServerMessage::Next {
                    id: id.clone(),
                    payload: NextPayload { data, errors },
                }),
                Err(e) => Output::Message(ServerMessage::Error {
                    id: id.clone(),
                    payload: e.into(),
                }),
            }
            .into_stream()
            .left_stream()
        };
        if let Some(panic_handler) = params.config.panic_handler.as_ref().map(Arc::clone) {
            let stream = AssertUnwindSafe(fut)
                .catch_unwind()
                .await
                .unwrap_or_else(|e| {
                    if let Some(e) = panic_handler(e, &params.config.context) {
                        Output::Message(ServerMessage::Next {
                            id: id.clone(),
                            payload: NextPayload {
                                data: Value::null(),
                                errors: vec![e],
                            },
                        })
                    } else {
                        Output::Close {
                            code: 1000,
                            message: "Operation execution panicked".into(),
                        }
                    }
                    .into_stream()
                    .left_stream()
                });
            AssertUnwindSafe(stream)
                .catch_unwind()
                .map(move |res| match res {
                    Ok(item) => item,
                    Err(e) => {
                        if let Some(e) = panic_handler(e, &params.config.context) {
                            Output::Message(ServerMessage::Next {
                                id: id.clone(),
                                payload: NextPayload {
                                    data: Value::null(),
                                    errors: vec![e],
                                },
                            })
                        } else {
                            Output::Close {
                                code: 1000,
                                message: "Subscription execution panicked".into(),
                            }
                        }
                    }
                })
                .boxed()
        } else {
            fut.await.boxed()
        }
    }
}

/// State of a [`SubscriptionStart`].
enum SubscriptionStartState<S: Schema> {
    /// Resolving the subscription into its event stream.
    ResolvingIntoStream {
        id: String,
        future: BoxFuture<
            'static,
            Result<juniper_subscriptions::Connection<'static, S::ScalarValue>, GraphQLError>,
        >,
    },

    /// Forwarding the events of the stream to the client.
    Streaming {
        id: String,
        stream: juniper_subscriptions::Connection<'static, S::ScalarValue>,
    },

    /// Done after the end of the stream or after an error.
    Terminated,
}

/// Stream of a subscription operation.
///
/// The `state` borrows from the heap allocation of the `_params` with a `'static` lifetime. Moving
/// a [`SubscriptionStart`] does not move that allocation. Fields drop in declaration order, so the
/// `state` drops before the `_params`.
struct SubscriptionStart<S: Schema> {
    state: SubscriptionStartState<S>,
    _params: Arc<ExecutionParams<S>>,
}

impl<S: Schema> SubscriptionStart<S> {
    /// Creates the stream of the subscription `operation`.
    ///
    /// # Safety
    ///
    /// The `operation` must borrow only from the heap allocation of the `params`.
    unsafe fn new(
        id: String,
        params: Arc<ExecutionParams<S>>,
        operation: ValidatedOperation<
            'static,
            S::Query,
            S::Mutation,
            S::Subscription,
            S::ScalarValue,
        >,
    ) -> Self {
        // SAFETY: The `state` keeps the borrows and drops before the `_params`.
        let borrowed: &'static ExecutionParams<S> = unsafe { &*Arc::as_ptr(&params) };
        let future = async move {
            let (stream, errors) = operation
                .resolve_into_stream(
                    &borrowed.subscribe_payload.variables,
                    &borrowed.config.context,
                )
                .await?;
            Ok(juniper_subscriptions::Connection::from_stream(
                stream, errors,
            ))
        }
        .boxed();

        Self {
            state: SubscriptionStartState::ResolvingIntoStream { id, future },
            _params: params,
        }
    }
}

impl<S: Schema> Stream for SubscriptionStart<S> {
    type Item = Output<S::ScalarValue>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match &mut this.state {
                SubscriptionStartState::ResolvingIntoStream { id, future } => {
                    match ready!(future.as_mut().poll(cx)) {
                        Ok(stream) => {
                            this.state = SubscriptionStartState::Streaming {
                                id: mem::take(id),
                                stream,
                            };
                        }
                        Err(e) => {
                            let id = mem::take(id);
                            this.state = SubscriptionStartState::Terminated;
                            return Poll::Ready(Some(Output::Message(ServerMessage::Error {
                                id,
                                payload: e.into(),
                            })));
                        }
                    }
                }
                SubscriptionStartState::Streaming { id, stream } => {
                    let Some(output) = ready!(Pin::new(stream).poll_next(cx)) else {
                        this.state = SubscriptionStartState::Terminated;
                        return Poll::Ready(None);
                    };
                    return Poll::Ready(Some(Output::Message(ServerMessage::Next {
                        id: id.clone(),
                        payload: NextPayload {
                            data: output.data,
                            errors: output.errors,
                        },
                    })));
                }
                SubscriptionStartState::Terminated => return Poll::Ready(None),
            }
        }
    }
}

enum ConnectionSinkState<S: Schema, I: Init<S::ScalarValue, S::Context>> {
    Ready {
        state: ConnectionState<S, I>,
    },
    HandlingMessage {
        #[expect(clippy::type_complexity, reason = "not really")]
        result: BoxFuture<
            'static,
            (
                ConnectionState<S, I>,
                BoxStream<'static, Output<S::ScalarValue>>,
            ),
        >,
    },
    Closed,
}

/// Implements the `graphql-ws` protocol.
/// This is a sink for `TryInto<Input>` messages and a stream of `Output` messages.
pub struct Connection<S: Schema, I: Init<S::ScalarValue, S::Context>> {
    reactions: SelectAll<BoxStream<'static, Output<S::ScalarValue>>>,
    stream_waker: Option<Waker>,
    stream_terminated: bool,
    sink_state: ConnectionSinkState<S, I>,
}

impl<S, I> Connection<S, I>
where
    S: Schema,
    I: Init<S::ScalarValue, S::Context>,
{
    /// Creates a new connection, which is a sink for `TryInto<Input>` messages and a stream of
    /// `Output` messages.
    ///
    /// The `schema` argument should typically be an `Arc<RootNode<...>>`.
    ///
    /// The `init` argument is used to provide the context and additional configuration for
    /// connections. This can be a `ConnectionConfig` if the context and configuration are already
    /// known, or it can be a closure that gets executed asynchronously when the client sends the
    /// ConnectionInit message. Using a closure allows you to perform authentication based on the
    /// parameters provided by the client.
    pub fn new(schema: S, init: I) -> Self {
        Self {
            reactions: SelectAll::new(),
            stream_waker: None,
            stream_terminated: false,
            sink_state: ConnectionSinkState::Ready {
                state: ConnectionState::PreInit { init, schema },
            },
        }
    }

    /// Performs polling of the [`Sink`] part of this [`Connection`].
    ///
    /// Effectively represents an implementation of [`Sink::poll_ready()`] and
    /// [`Sink::poll_flush()`] methods.
    fn poll_sink(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), &'static str>> {
        match &mut self.sink_state {
            ConnectionSinkState::Ready { .. } => Poll::Ready(Ok(())),
            ConnectionSinkState::HandlingMessage { result } => match Pin::new(result).poll(cx) {
                Poll::Ready((state, reactions)) => {
                    self.reactions.push(reactions);
                    self.sink_state = ConnectionSinkState::Ready { state };
                    if let Some(waker) = self.stream_waker.take() {
                        waker.wake();
                    }
                    Poll::Ready(Ok(()))
                }
                Poll::Pending => Poll::Pending,
            },
            ConnectionSinkState::Closed => Poll::Ready(Err("polled after close")),
        }
    }
}

impl<S, I, T> Sink<T> for Connection<S, I>
where
    T: TryInto<Input<S::ScalarValue>, Error: Error>,
    S: Schema,
    I: Init<S::ScalarValue, S::Context> + Send,
{
    type Error = Infallible;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.poll_sink(cx)
            .map_err(|e| panic!("`Connection::poll_ready()`: {e}"))
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let s = self.get_mut();
        let state = &mut s.sink_state;
        *state = match std::mem::replace(state, ConnectionSinkState::Closed) {
            ConnectionSinkState::Ready { state } => {
                match item.try_into() {
                    Ok(Input::Message(msg)) => ConnectionSinkState::HandlingMessage {
                        result: state.handle_message(msg).boxed(),
                    },
                    Ok(Input::Close) => {
                        s.reactions.push(
                            Output::Close {
                                code: 1000,
                                message: "Normal Closure".into(),
                            }
                            .into_stream()
                            .boxed(),
                        );
                        ConnectionSinkState::Closed
                    }
                    Err(e) => {
                        // If we weren't able to parse the message, we must close the connection.
                        s.reactions.push(
                            Output::Close {
                                code: 4400,
                                message: e.to_string(),
                            }
                            .into_stream()
                            .boxed(),
                        );
                        ConnectionSinkState::Closed
                    }
                }
            }
            _ => panic!("`Sink::start_send()`: called when not ready"),
        };
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.poll_sink(cx).map(|_| Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.sink_state = ConnectionSinkState::Closed;
        if let Some(waker) = self.stream_waker.take() {
            // Wake up the `Stream` so it can close too.
            waker.wake();
        }
        Poll::Ready(Ok(()))
    }
}

impl<S, I> Stream for Connection<S, I>
where
    S: Schema,
    I: Init<S::ScalarValue, S::Context>,
{
    type Item = Output<S::ScalarValue>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.stream_waker = Some(cx.waker().clone());

        if self.stream_terminated {
            return Poll::Ready(None);
        }

        // Poll the reactions for new outgoing messages.
        if !self.reactions.is_empty() {
            match Pin::new(&mut self.reactions).poll_next(cx) {
                Poll::Ready(Some(Output::Close { code, message })) => {
                    self.stream_terminated = true;
                    return Poll::Ready(Some(Output::Close { code, message }));
                }
                Poll::Ready(Some(reaction)) => return Poll::Ready(Some(reaction)),
                Poll::Ready(None) => {
                    // In rare cases, the reaction stream may terminate. For example, this will
                    // happen if the first message we receive does not require any reaction. Just
                    // recreate it in that case.
                    self.reactions = SelectAll::new();
                }
                _ => (),
            }
        }

        if let ConnectionSinkState::Closed = self.sink_state {
            return Poll::Ready(None);
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod test {
    use std::{convert::Infallible, io, sync::Arc, time::Duration};

    use juniper::{
        DefaultScalarValue, EmptyMutation, EmptySubscription, FieldError, FieldResult,
        GraphQLError, OperationType, RootNode, Variables,
        futures::{
            future::{self, FutureExt as _},
            sink::SinkExt,
            stream::{self, BoxStream, StreamExt as _},
        },
        graphql_input_value, graphql_object, graphql_subscription, graphql_value, graphql_vars,
        parser::{ParseError, Spanning},
    };

    use super::{Connection, ConnectionConfig, NextPayload, Output, SubscribePayload};

    #[derive(Clone, Copy)]
    struct Context(i32);

    impl juniper::Context for Context {}

    struct Query;

    #[graphql_object(context = Context)]
    impl Query {
        /// context just resolves to the current context.
        async fn context(context: &Context) -> i32 {
            context.0
        }
    }

    struct Subscription;

    #[graphql_subscription(context = Context)]
    impl Subscription {
        /// never never emits anything.
        async fn never(_context: &Context) -> BoxStream<'static, FieldResult<i32>> {
            tokio::time::sleep(Duration::from_secs(10000))
                .map(|_| unreachable!())
                .into_stream()
                .boxed()
        }

        /// context emits the current context once, then never emits anything else.
        async fn context(context: &Context) -> BoxStream<'static, FieldResult<i32>> {
            stream::once(future::ready(Ok(context.0)))
                .chain(
                    tokio::time::sleep(Duration::from_secs(10000))
                        .map(|_| unreachable!())
                        .into_stream(),
                )
                .boxed()
        }

        /// error emits an error once, then never emits anything else.
        async fn error(_context: &Context) -> BoxStream<'static, FieldResult<i32>> {
            stream::once(future::ready(Err(FieldError::new(
                "field error",
                graphql_value!(null),
            ))))
            .chain(
                tokio::time::sleep(Duration::from_secs(10000))
                    .map(|_| unreachable!())
                    .into_stream(),
            )
            .boxed()
        }
    }

    type ClientMessage = super::ClientMessage<DefaultScalarValue>;
    type ServerMessage = super::ServerMessage<DefaultScalarValue>;

    fn new_test_schema() -> Arc<RootNode<Query, EmptyMutation<Context>, Subscription>> {
        Arc::new(RootNode::new(Query, EmptyMutation::new(), Subscription))
    }

    #[tokio::test]
    async fn test_query() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_secs(0)),
        );

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Subscribe {
            id: "foo".into(),
            payload: SubscribePayload {
                query: "{context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::Next {
                id: "foo".into(),
                payload: NextPayload {
                    data: graphql_value!({"context": 1}),
                    errors: vec![],
                },
            }),
            conn.next().await.unwrap()
        );

        assert_eq!(
            Output::Message(ServerMessage::Complete { id: "foo".into() }),
            conn.next().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_premature_query() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_secs(0)),
        );

        conn.send(ClientMessage::Subscribe {
            id: "foo".into(),
            payload: SubscribePayload {
                query: "{context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Close {
                code: 4401,
                message: "Unauthorized".into(),
            },
            conn.next().await.unwrap()
        );

        assert_eq!(None, conn.next().await);
    }

    #[tokio::test]
    async fn test_subscriptions() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_secs(0)),
        );

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Subscribe {
            id: "foo".into(),
            payload: SubscribePayload {
                query: "subscription Foo {context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::Next {
                id: "foo".into(),
                payload: NextPayload {
                    data: graphql_value!({"context": 1}),
                    errors: vec![],
                },
            }),
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Subscribe {
            id: "bar".into(),
            payload: SubscribePayload {
                query: "subscription Bar {context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::Next {
                id: "bar".into(),
                payload: NextPayload {
                    data: graphql_value!({"context": 1}),
                    errors: vec![],
                },
            }),
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Complete { id: "foo".into() })
            .await
            .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::Complete { id: "foo".into() }),
            conn.next().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_init_params_ok() {
        let mut conn = Connection::new(new_test_schema(), async |params: Variables| {
            assert_eq!(params.get("foo"), Some(&graphql_input_value!("bar")));
            Ok(ConnectionConfig::new(Context(1))) as Result<_, Infallible>
        });

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {"foo": "bar"},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_init_params_error() {
        let mut conn = Connection::new(new_test_schema(), async |params: Variables| {
            assert_eq!(params.get("foo"), Some(&graphql_input_value!("bar")));
            Err(io::Error::other("init error"))
        });

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {"foo": "bar"},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Close {
                code: 4403,
                message: "init error".into(),
            },
            conn.next().await.unwrap()
        );

        assert_eq!(None, conn.next().await);
    }

    #[tokio::test]
    async fn test_max_in_flight_operations() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1))
                .with_keep_alive_interval(Duration::from_secs(0))
                .with_max_in_flight_operations(1),
        );

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Subscribe {
            id: "foo".into(),
            payload: SubscribePayload {
                query: "subscription Foo {never}".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        conn.send(ClientMessage::Subscribe {
            id: "bar".into(),
            payload: SubscribePayload {
                query: "subscription Bar {never}".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        match conn.next().await.unwrap() {
            Output::Message(ServerMessage::Error { id, .. }) => {
                assert_eq!(id, "bar");
            }
            msg => panic!("expected error, got: {msg:?}"),
        }
    }

    #[tokio::test]
    async fn test_parse_error() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_secs(0)),
        );

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Subscribe {
            id: "foo".into(),
            payload: SubscribePayload {
                query: "asd".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        match conn.next().await.unwrap() {
            Output::Message(ServerMessage::Error { id, payload }) => {
                assert_eq!(id, "foo");
                match payload.graphql_error() {
                    GraphQLError::ParseError(Spanning {
                        item: ParseError::UnexpectedToken(token),
                        ..
                    }) => assert_eq!(token, "asd"),
                    p => panic!("expected graphql parse error, got: {p:?}"),
                }
            }
            msg => panic!("expected error, got: {msg:?}"),
        }
    }

    #[tokio::test]
    async fn test_keep_alive_interval() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1))
                .with_keep_alive_interval(Duration::from_millis(20))
                .with_keep_alive_timeout(Duration::from_secs(0)),
        );

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap()
        );

        for _ in 0..10 {
            assert_eq!(
                Output::Message(ServerMessage::Pong),
                conn.next().await.unwrap()
            );
        }
    }

    #[tokio::test]
    async fn test_keep_alive_timeout() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1))
                .with_keep_alive_interval(Duration::from_millis(0))
                .with_keep_alive_timeout(Duration::from_millis(20)),
        );

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap()
        );

        assert_eq!(
            Output::Close {
                code: 1000,
                message: "Connection lost unexpectedly".into(),
            },
            conn.next().await.unwrap(),
        );
    }

    #[tokio::test]
    async fn test_slow_init() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_secs(0)),
        );

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {},
        })
        .await
        .unwrap();

        // If we send the start message before the init is handled, we should still get results.
        conn.send(ClientMessage::Subscribe {
            id: "foo".into(),
            payload: SubscribePayload {
                query: "{context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap(),
        );

        assert_eq!(
            Output::Message(ServerMessage::Next {
                id: "foo".into(),
                payload: NextPayload {
                    data: graphql_value!({"context": 1}),
                    errors: vec![],
                },
            }),
            conn.next().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_subscription_field_error() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_secs(0)),
        );

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Subscribe {
            id: "foo".into(),
            payload: SubscribePayload {
                query: "subscription Foo {error}".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        match conn.next().await.unwrap() {
            Output::Message(ServerMessage::Next {
                id,
                payload: NextPayload { data, errors },
            }) => {
                assert_eq!(id, "foo");
                assert_eq!(data, graphql_value!({ "error": null }));
                assert_eq!(errors.len(), 1);
            }
            msg => panic!("expected data, got: {msg:?}"),
        }
    }

    #[tokio::test]
    async fn test_subscription_not_supported() {
        let mut conn = Connection::new(
            Arc::new(RootNode::new(
                Query,
                EmptyMutation::<Context>::new(),
                EmptySubscription::<Context>::new(),
            )),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_secs(0)),
        );

        conn.send(ClientMessage::ConnectionInit {
            payload: graphql_vars! {},
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::ConnectionAck),
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Subscribe {
            id: "foo".into(),
            payload: SubscribePayload {
                query: "subscription Foo {context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
                extensions: Default::default(),
            },
        })
        .await
        .unwrap();

        assert_eq!(
            Output::Message(ServerMessage::Error {
                id: "foo".into(),
                payload: GraphQLError::NotSupported(OperationType::Subscription).into(),
            }),
            conn.next().await.unwrap()
        );

        assert_eq!(
            Output::Message(ServerMessage::Complete { id: "foo".into() }),
            conn.next().await.unwrap()
        );
    }

    /// Tests of the order in which a cancelled subscription drops its parts.
    mod drop_order {
        use std::{
            sync::{Arc, Mutex, Weak},
            time::Duration,
        };

        use juniper::{
            EmptyMutation, FieldResult, RootNode,
            futures::{
                sink::SinkExt as _,
                stream::{self, BoxStream, StreamExt as _},
            },
            graphql_object, graphql_subscription, graphql_value, graphql_vars,
        };

        use super::{
            ClientMessage, Connection, ConnectionConfig, NextPayload, Output, ServerMessage,
            SubscribePayload,
        };

        /// Context with a liveness token of its own for each clone.
        struct Context {
            /// Liveness token of this clone of the context.
            token: Arc<()>,

            /// Whether the context of the subscription stream was alive when the stream dropped.
            alive_on_stream_drop: Arc<Mutex<Option<bool>>>,
        }

        impl Clone for Context {
            fn clone(&self) -> Self {
                Self {
                    token: Arc::new(()),
                    alive_on_stream_drop: Arc::clone(&self.alive_on_stream_drop),
                }
            }
        }

        impl juniper::Context for Context {}

        /// Records whether the context of a subscription stream is alive when the stream drops.
        struct DropCheck {
            context_token: Weak<()>,
            alive_on_drop: Arc<Mutex<Option<bool>>>,
        }

        impl Drop for DropCheck {
            fn drop(&mut self) {
                *self.alive_on_drop.lock().unwrap() = Some(self.context_token.upgrade().is_some());
            }
        }

        struct Query;

        #[graphql_object(context = Context)]
        impl Query {
            fn ping() -> bool {
                true
            }
        }

        struct Subscription;

        #[graphql_subscription(context = Context)]
        impl Subscription {
            /// Emits one value and then never ends.
            async fn once(context: &Context) -> BoxStream<'static, FieldResult<i32>> {
                let check = DropCheck {
                    context_token: Arc::downgrade(&context.token),
                    alive_on_drop: Arc::clone(&context.alive_on_stream_drop),
                };
                stream::iter([Ok(1)])
                    .chain(stream::pending())
                    .inspect(move |_| _ = &check)
                    .boxed()
            }
        }

        #[tokio::test]
        async fn stream_before_context() {
            let alive_on_stream_drop = Arc::new(Mutex::new(None));
            let context = Context {
                token: Arc::new(()),
                alive_on_stream_drop: Arc::clone(&alive_on_stream_drop),
            };
            let mut conn = Connection::new(
                Arc::new(RootNode::new(Query, EmptyMutation::new(), Subscription)),
                ConnectionConfig::new(context).with_keep_alive_interval(Duration::from_secs(0)),
            );

            conn.send(ClientMessage::ConnectionInit {
                payload: graphql_vars! {},
            })
            .await
            .unwrap();

            assert_eq!(
                Output::Message(ServerMessage::ConnectionAck),
                conn.next().await.unwrap()
            );

            conn.send(ClientMessage::Subscribe {
                id: "foo".into(),
                payload: SubscribePayload {
                    query: "subscription Foo {once}".into(),
                    variables: graphql_vars! {},
                    operation_name: None,
                    extensions: Default::default(),
                },
            })
            .await
            .unwrap();

            assert_eq!(
                Output::Message(ServerMessage::Next {
                    id: "foo".into(),
                    payload: NextPayload {
                        data: graphql_value!({"once": 1}),
                        errors: vec![],
                    },
                }),
                conn.next().await.unwrap()
            );

            conn.send(ClientMessage::Complete { id: "foo".into() })
                .await
                .unwrap();

            assert_eq!(
                Output::Message(ServerMessage::Complete { id: "foo".into() }),
                conn.next().await.unwrap()
            );
            assert_eq!(*alive_on_stream_drop.lock().unwrap(), Some(true));
        }
    }
}
