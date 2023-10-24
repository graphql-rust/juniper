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
    collections::HashMap, convert::Infallible, error::Error, marker::PhantomPinned, pin::Pin,
    sync::Arc, time::Duration,
};

use juniper::{
    futures::{
        channel::oneshot,
        future::{self, BoxFuture, Either, Future, FutureExt, TryFutureExt},
        stream::{self, BoxStream, SelectAll, StreamExt},
        task::{Context, Poll, Waker},
        Sink, Stream,
    },
    GraphQLError, RuleError, ScalarValue,
};

use super::{ConnectionConfig, Init, Schema};

pub use self::{
    client_message::{ClientMessage, SubscribePayload},
    server_message::{ErrorPayload, NextPayload, ServerMessage},
};

struct ExecutionParams<S: Schema> {
    subscribe_payload: SubscribePayload<S::ScalarValue>,
    config: Arc<ConnectionConfig<S::Context>>,
    schema: S,
}

/// Possible inputs received from a client.
#[derive(Debug)]
pub enum Input<S> {
    /// Deserialized [`ClientMessage`].
    Message(ClientMessage<S>),

    /// Client initiated normal closing of a [`Connection`].
    Close,
}

impl<S> From<ClientMessage<S>> for Input<S> {
    fn from(val: ClientMessage<S>) -> Self {
        Self::Message(val)
    }
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
    /// Converts the reaction into a one-item stream.
    fn into_stream(self) -> BoxStream<'static, Self> {
        stream::once(future::ready(self)).boxed()
    }
}

enum ConnectionState<S: Schema, I: Init<S::ScalarValue, S::Context>> {
    /// PreInit is the state before a ConnectionInit message has been accepted.
    PreInit { init: I, schema: S },
    /// Active is the state after a ConnectionInit message has been accepted.
    Active {
        config: Arc<ConnectionConfig<S::Context>>,
        stoppers: HashMap<String, oneshot::Sender<()>>,
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
                        let keep_alive_interval = config.keep_alive_interval;

                        let mut s =
                            stream::iter(vec![Output::Message(ServerMessage::ConnectionAck)])
                                .boxed();

                        if keep_alive_interval > Duration::from_secs(0) {
                            s = s
                                .chain(Output::Message(ServerMessage::Pong).into_stream())
                                .boxed();
                            s = s
                                .chain(stream::unfold((), move |_| async move {
                                    tokio::time::sleep(keep_alive_interval).await;
                                    Some((Output::Message(ServerMessage::Pong), ()))
                                }))
                                .boxed();
                        }

                        (
                            Self::Active {
                                config: Arc::new(config),
                                stoppers: HashMap::new(),
                                schema,
                            },
                            s,
                        )
                    }
                    Err(e) => (
                        Self::Terminated,
                        stream::iter(vec![Output::Close {
                            code: 4403,
                            message: e.to_string(),
                        }])
                        .boxed(),
                    ),
                },
                ClientMessage::Ping { .. } => (
                    Self::PreInit { init, schema },
                    stream::iter(vec![Output::Message(ServerMessage::Pong)]).boxed(),
                ),
                ClientMessage::Subscribe { .. } => (
                    Self::PreInit { init, schema },
                    stream::iter(vec![Output::Close {
                        code: 4401,
                        message: "Unauthorized".to_string(),
                    }])
                    .boxed(),
                ),
                _ => (Self::PreInit { init, schema }, stream::empty().boxed()),
            },
            Self::Active {
                config,
                mut stoppers,
                schema,
            } => {
                let reactions = match msg {
                    ClientMessage::Subscribe { id, payload } => {
                        if stoppers.contains_key(&id) {
                            // We already have an operation with this id. We must close the connection.
                            Output::Close {
                                code: 4409,
                                message: format!("Subscriber for {} already exists", id),
                            }
                            .into_stream()
                        } else {
                            // Go ahead and prune canceled stoppers before adding a new one.
                            stoppers.retain(|_, tx| !tx.is_canceled());

                            if config.max_in_flight_operations > 0
                                && stoppers.len() >= config.max_in_flight_operations
                            {
                                // Too many in-flight operations. Just send back a validation error.
                                stream::iter(vec![
                                    Output::Message(ServerMessage::Error {
                                        id: id.clone(),
                                        payload: GraphQLError::ValidationError(vec![
                                            RuleError::new("Too many in-flight operations.", &[]),
                                        ])
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
                                let s = stream::unfold((rx, s.boxed()), |(rx, mut s)| async move {
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
                    }
                    ClientMessage::Complete { id } => {
                        stoppers.remove(&id);
                        stream::empty().boxed()
                    }
                    ClientMessage::Ping { .. } => {
                        stream::iter(vec![Output::Message(ServerMessage::Pong)]).boxed()
                    }
                    _ => stream::empty().boxed(),
                };
                (
                    Self::Active {
                        config,
                        stoppers,
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
        // TODO: This could be made more efficient if `juniper` exposed
        //       functionality to allow us to parse and validate the query,
        //       determine whether it's a subscription, and then execute it.
        //       For now, the query gets parsed and validated twice.

        let params = Arc::new(params);

        // Try to execute this as a query or mutation.
        match juniper::execute(
            &params.subscribe_payload.query,
            params.subscribe_payload.operation_name.as_deref(),
            params.schema.root_node(),
            &params.subscribe_payload.variables,
            &params.config.context,
        )
        .await
        {
            Ok((data, errors)) => {
                return Output::Message(ServerMessage::Next {
                    id: id.clone(),
                    payload: NextPayload { data, errors },
                })
                .into_stream();
            }
            Err(GraphQLError::IsSubscription) => {}
            Err(e) => {
                return Output::Message(ServerMessage::Error {
                    id: id.clone(),
                    payload: ErrorPayload::new(Box::new(params.clone()), e),
                })
                .into_stream();
            }
        }

        // Try to execute as a subscription.
        SubscriptionStart::new(id, params.clone()).boxed()
    }
}

struct InterruptableStream<S> {
    stream: S,
    rx: oneshot::Receiver<()>,
}

impl<S: Stream + Unpin> Stream for InterruptableStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(_) => return Poll::Ready(None),
            Poll::Pending => {}
        }
        Pin::new(&mut self.stream).poll_next(cx)
    }
}

/// SubscriptionStartState is the state for a subscription operation.
enum SubscriptionStartState<S: Schema> {
    /// Init is the start before being polled for the first time.
    Init { id: String },
    /// ResolvingIntoStream is the state after being polled for the first time. In this state,
    /// we're parsing, validating, and getting the actual event stream.
    ResolvingIntoStream {
        id: String,
        future: BoxFuture<
            'static,
            Result<juniper_subscriptions::Connection<'static, S::ScalarValue>, GraphQLError>,
        >,
    },
    /// Streaming is the state after we've successfully obtained the event stream for the
    /// subscription. In this state, we're just forwarding events back to the client.
    Streaming {
        id: String,
        stream: juniper_subscriptions::Connection<'static, S::ScalarValue>,
    },
    /// Terminated is the state once we're all done.
    Terminated,
}

/// SubscriptionStart is the stream for a subscription operation.
struct SubscriptionStart<S: Schema> {
    params: Arc<ExecutionParams<S>>,
    state: SubscriptionStartState<S>,
    _marker: PhantomPinned,
}

impl<S: Schema> SubscriptionStart<S> {
    fn new(id: String, params: Arc<ExecutionParams<S>>) -> Pin<Box<Self>> {
        Box::pin(Self {
            params,
            state: SubscriptionStartState::Init { id },
            _marker: PhantomPinned,
        })
    }
}

impl<S: Schema> Stream for SubscriptionStart<S> {
    type Item = Output<S::ScalarValue>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let (params, state) = unsafe {
            // XXX: The execution parameters are referenced by state and must not be modified.
            // Modifying state is fine though.
            let inner = self.get_unchecked_mut();
            (&inner.params, &mut inner.state)
        };

        loop {
            match state {
                SubscriptionStartState::Init { id } => {
                    // XXX: resolve_into_stream returns a Future that references the execution
                    // parameters, and the returned stream also references them. We can guarantee
                    // that everything has the same lifetime in this self-referential struct.
                    let params = Arc::as_ptr(params);
                    *state = SubscriptionStartState::ResolvingIntoStream {
                        id: id.clone(),
                        future: unsafe {
                            juniper::resolve_into_stream(
                                &(*params).subscribe_payload.query,
                                (*params).subscribe_payload.operation_name.as_deref(),
                                (*params).schema.root_node(),
                                &(*params).subscribe_payload.variables,
                                &(*params).config.context,
                            )
                        }
                        .map_ok(|(stream, errors)| {
                            juniper_subscriptions::Connection::from_stream(stream, errors)
                        })
                        .boxed(),
                    };
                }
                SubscriptionStartState::ResolvingIntoStream {
                    ref id,
                    ref mut future,
                } => match future.as_mut().poll(cx) {
                    Poll::Ready(r) => match r {
                        Ok(stream) => {
                            *state = SubscriptionStartState::Streaming {
                                id: id.clone(),
                                stream,
                            }
                        }
                        Err(e) => {
                            return Poll::Ready(Some(Output::Message(ServerMessage::Error {
                                id: id.clone(),
                                payload: ErrorPayload::new(Box::new(params.clone()), e),
                            })));
                        }
                    },
                    Poll::Pending => return Poll::Pending,
                },
                SubscriptionStartState::Streaming {
                    ref id,
                    ref mut stream,
                } => match Pin::new(stream).poll_next(cx) {
                    Poll::Ready(Some(output)) => {
                        return Poll::Ready(Some(Output::Message(ServerMessage::Next {
                            id: id.clone(),
                            payload: NextPayload {
                                data: output.data,
                                errors: output.errors,
                            },
                        })));
                    }
                    Poll::Ready(None) => {
                        *state = SubscriptionStartState::Terminated;
                        return Poll::Ready(None);
                    }
                    Poll::Pending => return Poll::Pending,
                },
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
        #[allow(clippy::type_complexity)]
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
            ConnectionSinkState::HandlingMessage { ref mut result } => {
                match Pin::new(result).poll(cx) {
                    Poll::Ready((state, reactions)) => {
                        self.reactions.push(reactions);
                        self.sink_state = ConnectionSinkState::Ready { state };
                        if let Some(waker) = self.stream_waker.take() {
                            waker.wake();
                        }
                        Poll::Ready(Ok(()))
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
            ConnectionSinkState::Closed => Poll::Ready(Err("polled after close")),
        }
    }
}

impl<S, I, T> Sink<T> for Connection<S, I>
where
    T: TryInto<Input<S::ScalarValue>>,
    T::Error: Error,
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
                            .into_stream(),
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
                            .into_stream(),
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
    use std::{convert::Infallible, io};

    use juniper::{
        futures::sink::SinkExt,
        graphql_input_value, graphql_object, graphql_subscription, graphql_value, graphql_vars,
        parser::{ParseError, Spanning},
        DefaultScalarValue, EmptyMutation, FieldError, FieldResult, RootNode, Variables,
    };

    use super::*;

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

    fn new_test_schema() -> Arc<RootNode<'static, Query, EmptyMutation<Context>, Subscription>> {
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
        let mut conn = Connection::new(new_test_schema(), |params: Variables| async move {
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
        let mut conn = Connection::new(new_test_schema(), |params: Variables| async move {
            assert_eq!(params.get("foo"), Some(&graphql_input_value!("bar")));
            Err(io::Error::new(io::ErrorKind::Other, "init error"))
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
    async fn test_keep_alives() {
        let mut conn = Connection::new(
            new_test_schema(),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_millis(20)),
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
            conn.next().await.unwrap()
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
}
