//! Implementation of the [legacy `graphql-ws` GraphQL over WebSocket Protocol][old], as formerly
//! used by [Apollo] and [`subscriptions-transport-ws` npm package].
//!
//! It has now been deprecated in favor of the
//! [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], implemented by the
//! [`graphql_transport_ws` module] and new [`graphql-ws` npm package].
//!
//! [`graphql_transport_ws` module]: crate::graphql_transport_ws
//! [`graphql-ws` npm package]: https://npmjs.com/package/graphql-ws
//! [`subscriptions-transport-ws` npm package]: https://npmjs.com/package/subscriptions-transport-ws
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
    GraphQLError, RuleError,
};

use super::{ConnectionConfig, Init, Schema};

pub use self::{
    client_message::{ClientMessage, StartPayload},
    server_message::{ConnectionErrorPayload, DataPayload, ErrorPayload, ServerMessage},
};

struct ExecutionParams<S: Schema> {
    start_payload: StartPayload<S::ScalarValue>,
    config: Arc<ConnectionConfig<S::Context>>,
    schema: S,
}

enum Reaction<S: Schema> {
    ServerMessage(ServerMessage<S::ScalarValue>),
    EndStream,
}

impl<S: Schema> Reaction<S> {
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
    // ConnectionTerminate message results in a one-item stream with the EndStream reaction.
    async fn handle_message(
        self,
        msg: ClientMessage<S::ScalarValue>,
    ) -> (Self, BoxStream<'static, Reaction<S>>) {
        if let ClientMessage::ConnectionTerminate = msg {
            return (self, Reaction::EndStream.into_stream());
        }

        match self {
            Self::PreInit { init, schema } => match msg {
                ClientMessage::ConnectionInit { payload } => match init.init(payload).await {
                    Ok(config) => {
                        let keep_alive_interval = config.keep_alive_interval;

                        let mut s = stream::iter(vec![Reaction::ServerMessage(
                            ServerMessage::ConnectionAck,
                        )])
                        .boxed();

                        if keep_alive_interval > Duration::from_secs(0) {
                            s = s
                                .chain(
                                    Reaction::ServerMessage(ServerMessage::ConnectionKeepAlive)
                                        .into_stream(),
                                )
                                .boxed();
                            s = s
                                .chain(stream::unfold((), move |_| async move {
                                    tokio::time::sleep(keep_alive_interval).await;
                                    Some((
                                        Reaction::ServerMessage(ServerMessage::ConnectionKeepAlive),
                                        (),
                                    ))
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
                        stream::iter(vec![
                            Reaction::ServerMessage(ServerMessage::ConnectionError {
                                payload: ConnectionErrorPayload {
                                    message: e.to_string(),
                                },
                            }),
                            Reaction::EndStream,
                        ])
                        .boxed(),
                    ),
                },
                _ => (Self::PreInit { init, schema }, stream::empty().boxed()),
            },
            Self::Active {
                config,
                mut stoppers,
                schema,
            } => {
                let reactions = match msg {
                    ClientMessage::Start { id, payload } => {
                        if stoppers.contains_key(&id) {
                            // We already have an operation with this id, so we can't start a new
                            // one.
                            stream::empty().boxed()
                        } else {
                            // Go ahead and prune canceled stoppers before adding a new one.
                            stoppers.retain(|_, tx| !tx.is_canceled());

                            if config.max_in_flight_operations > 0
                                && stoppers.len() >= config.max_in_flight_operations
                            {
                                // Too many in-flight operations. Just send back a validation error.
                                stream::iter(vec![
                                    Reaction::ServerMessage(ServerMessage::Error {
                                        id: id.clone(),
                                        payload: GraphQLError::ValidationError(vec![
                                            RuleError::new("Too many in-flight operations.", &[]),
                                        ])
                                        .into(),
                                    }),
                                    Reaction::ServerMessage(ServerMessage::Complete { id }),
                                ])
                                .boxed()
                            } else {
                                // Create a channel that we can use to cancel the operation.
                                let (tx, rx) = oneshot::channel::<()>();
                                stoppers.insert(id.clone(), tx);

                                // Create the operation stream. This stream will emit Data and Error
                                // messages, but will not emit Complete – that part is up to us.
                                let s = Self::start(
                                    id.clone(),
                                    ExecutionParams {
                                        start_payload: payload,
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
                                    Reaction::ServerMessage(ServerMessage::Complete { id })
                                        .into_stream(),
                                );

                                s.boxed()
                            }
                        }
                    }
                    ClientMessage::Stop { id } => {
                        stoppers.remove(&id);
                        stream::empty().boxed()
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

    async fn start(id: String, params: ExecutionParams<S>) -> BoxStream<'static, Reaction<S>> {
        // TODO: This could be made more efficient if `juniper` exposed
        //       functionality to allow us to parse and validate the query,
        //       determine whether it's a subscription, and then execute it.
        //       For now, the query gets parsed and validated twice.

        let params = Arc::new(params);

        // Try to execute this as a query or mutation.
        match juniper::execute(
            &params.start_payload.query,
            params.start_payload.operation_name.as_deref(),
            params.schema.root_node(),
            &params.start_payload.variables,
            &params.config.context,
        )
        .await
        {
            Ok((data, errors)) => {
                return Reaction::ServerMessage(ServerMessage::Data {
                    id: id.clone(),
                    payload: DataPayload { data, errors },
                })
                .into_stream();
            }
            Err(GraphQLError::IsSubscription) => {}
            Err(e) => {
                return Reaction::ServerMessage(ServerMessage::Error {
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
    type Item = Reaction<S>;

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
                                &(*params).start_payload.query,
                                (*params).start_payload.operation_name.as_deref(),
                                (*params).schema.root_node(),
                                &(*params).start_payload.variables,
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
                            return Poll::Ready(Some(Reaction::ServerMessage(
                                ServerMessage::Error {
                                    id: id.clone(),
                                    payload: ErrorPayload::new(Box::new(params.clone()), e),
                                },
                            )));
                        }
                    },
                    Poll::Pending => return Poll::Pending,
                },
                SubscriptionStartState::Streaming {
                    ref id,
                    ref mut stream,
                } => match Pin::new(stream).poll_next(cx) {
                    Poll::Ready(Some(output)) => {
                        return Poll::Ready(Some(Reaction::ServerMessage(ServerMessage::Data {
                            id: id.clone(),
                            payload: DataPayload {
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
        result: BoxFuture<'static, (ConnectionState<S, I>, BoxStream<'static, Reaction<S>>)>,
    },
    Closed,
}

/// Implements the graphql-ws protocol. This is a sink for `TryInto<ClientMessage>` and a stream of
/// `ServerMessage`.
pub struct Connection<S: Schema, I: Init<S::ScalarValue, S::Context>> {
    reactions: SelectAll<BoxStream<'static, Reaction<S>>>,
    stream_waker: Option<Waker>,
    sink_state: ConnectionSinkState<S, I>,
}

impl<S, I> Connection<S, I>
where
    S: Schema,
    I: Init<S::ScalarValue, S::Context>,
{
    /// Creates a new connection, which is a sink for `TryInto<ClientMessage>` and a stream of `ServerMessage`.
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
            sink_state: ConnectionSinkState::Ready {
                state: ConnectionState::PreInit { init, schema },
            },
        }
    }
}

impl<S, I, T> Sink<T> for Connection<S, I>
where
    T: TryInto<ClientMessage<S::ScalarValue>>,
    T::Error: Error,
    S: Schema,
    I: Init<S::ScalarValue, S::Context> + Send,
{
    type Error = Infallible;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match &mut self.sink_state {
            ConnectionSinkState::Ready { .. } => Poll::Ready(Ok(())),
            ConnectionSinkState::HandlingMessage { ref mut result } => {
                match Pin::new(result).poll(cx) {
                    Poll::Ready((state, reactions)) => {
                        self.reactions.push(reactions);
                        self.sink_state = ConnectionSinkState::Ready { state };
                        Poll::Ready(Ok(()))
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
            ConnectionSinkState::Closed => panic!("poll_ready called after close"),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let s = self.get_mut();
        let state = &mut s.sink_state;
        *state = match std::mem::replace(state, ConnectionSinkState::Closed) {
            ConnectionSinkState::Ready { state } => {
                match item.try_into() {
                    Ok(msg) => ConnectionSinkState::HandlingMessage {
                        result: state.handle_message(msg).boxed(),
                    },
                    Err(e) => {
                        // If we weren't able to parse the message, send back an error.
                        s.reactions.push(
                            Reaction::ServerMessage(ServerMessage::ConnectionError {
                                payload: ConnectionErrorPayload {
                                    message: e.to_string(),
                                },
                            })
                            .into_stream(),
                        );
                        ConnectionSinkState::Ready { state }
                    }
                }
            }
            _ => panic!("start_send called when not ready"),
        };
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        <Self as Sink<T>>::poll_ready(self, cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.sink_state = ConnectionSinkState::Closed;
        if let Some(waker) = self.stream_waker.take() {
            // Wake up the stream so it can close too.
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
    type Item = ServerMessage<S::ScalarValue>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.stream_waker = Some(cx.waker().clone());

        if let ConnectionSinkState::Closed = self.sink_state {
            return Poll::Ready(None);
        }

        // Poll the reactions for new outgoing messages.
        if !self.reactions.is_empty() {
            match Pin::new(&mut self.reactions).poll_next(cx) {
                Poll::Ready(Some(reaction)) => match reaction {
                    Reaction::ServerMessage(msg) => return Poll::Ready(Some(msg)),
                    Reaction::EndStream => return Poll::Ready(None),
                },
                Poll::Ready(None) => {
                    // In rare cases, the reaction stream may terminate. For example, this will
                    // happen if the first message we receive does not require any reaction. Just
                    // recreate it in that case.
                    self.reactions = SelectAll::new();
                }
                _ => (),
            }
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

        assert_eq!(ServerMessage::ConnectionAck, conn.next().await.unwrap());

        conn.send(ClientMessage::Start {
            id: "foo".into(),
            payload: StartPayload {
                query: "{context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
            },
        })
        .await
        .unwrap();

        assert_eq!(
            ServerMessage::Data {
                id: "foo".into(),
                payload: DataPayload {
                    data: graphql_value!({"context": 1}),
                    errors: vec![],
                },
            },
            conn.next().await.unwrap()
        );

        assert_eq!(
            ServerMessage::Complete { id: "foo".into() },
            conn.next().await.unwrap()
        );
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

        assert_eq!(ServerMessage::ConnectionAck, conn.next().await.unwrap());

        conn.send(ClientMessage::Start {
            id: "foo".into(),
            payload: StartPayload {
                query: "subscription Foo {context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
            },
        })
        .await
        .unwrap();

        assert_eq!(
            ServerMessage::Data {
                id: "foo".into(),
                payload: DataPayload {
                    data: graphql_value!({"context": 1}),
                    errors: vec![],
                },
            },
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Start {
            id: "bar".into(),
            payload: StartPayload {
                query: "subscription Bar {context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
            },
        })
        .await
        .unwrap();

        assert_eq!(
            ServerMessage::Data {
                id: "bar".into(),
                payload: DataPayload {
                    data: graphql_value!({"context": 1}),
                    errors: vec![],
                },
            },
            conn.next().await.unwrap()
        );

        conn.send(ClientMessage::Stop { id: "foo".into() })
            .await
            .unwrap();

        assert_eq!(
            ServerMessage::Complete { id: "foo".into() },
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

        assert_eq!(ServerMessage::ConnectionAck, conn.next().await.unwrap());
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
            ServerMessage::ConnectionError {
                payload: ConnectionErrorPayload {
                    message: "init error".into(),
                },
            },
            conn.next().await.unwrap()
        );
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

        assert_eq!(ServerMessage::ConnectionAck, conn.next().await.unwrap());

        conn.send(ClientMessage::Start {
            id: "foo".into(),
            payload: StartPayload {
                query: "subscription Foo {never}".into(),
                variables: graphql_vars! {},
                operation_name: None,
            },
        })
        .await
        .unwrap();

        conn.send(ClientMessage::Start {
            id: "bar".into(),
            payload: StartPayload {
                query: "subscription Bar {never}".into(),
                variables: graphql_vars! {},
                operation_name: None,
            },
        })
        .await
        .unwrap();

        match conn.next().await.unwrap() {
            ServerMessage::Error { id, .. } => {
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

        assert_eq!(ServerMessage::ConnectionAck, conn.next().await.unwrap());

        conn.send(ClientMessage::Start {
            id: "foo".into(),
            payload: StartPayload {
                query: "asd".into(),
                variables: graphql_vars! {},
                operation_name: None,
            },
        })
        .await
        .unwrap();

        match conn.next().await.unwrap() {
            ServerMessage::Error { id, payload } => {
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

        assert_eq!(ServerMessage::ConnectionAck, conn.next().await.unwrap());

        for _ in 0..10 {
            assert_eq!(
                ServerMessage::ConnectionKeepAlive,
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
        conn.send(ClientMessage::Start {
            id: "foo".into(),
            payload: StartPayload {
                query: "{context}".into(),
                variables: graphql_vars! {},
                operation_name: None,
            },
        })
        .await
        .unwrap();

        assert_eq!(ServerMessage::ConnectionAck, conn.next().await.unwrap());

        assert_eq!(
            ServerMessage::Data {
                id: "foo".into(),
                payload: DataPayload {
                    data: graphql_value!({"context": 1}),
                    errors: vec![],
                },
            },
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

        assert_eq!(ServerMessage::ConnectionAck, conn.next().await.unwrap());

        conn.send(ClientMessage::Start {
            id: "foo".into(),
            payload: StartPayload {
                query: "subscription Foo {error}".into(),
                variables: graphql_vars! {},
                operation_name: None,
            },
        })
        .await
        .unwrap();

        match conn.next().await.unwrap() {
            ServerMessage::Data {
                id,
                payload: DataPayload { data, errors },
            } => {
                assert_eq!(id, "foo");
                assert_eq!(data, graphql_value!({ "error": null }));
                assert_eq!(errors.len(), 1);
            }
            msg => panic!("expected data, got: {msg:?}"),
        }
    }
}
