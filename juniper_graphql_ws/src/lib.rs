#[macro_use]
extern crate serde;

mod client_message;
pub use client_message::*;

mod server_message;
pub use server_message::*;

use juniper::{
    futures::{
        channel::oneshot,
        future::{self, BoxFuture, Either, Future, FutureExt, TryFutureExt},
        stream::{self, BoxStream, SelectAll, StreamExt},
        task::{Context, Poll},
        Stream,
    },
    GraphQLError, GraphQLSubscriptionType, GraphQLTypeAsync, RootNode, RuleError, ScalarValue,
    Variables,
};
use std::{
    collections::HashMap, convert::{Infallible, TryInto}, error::Error, marker::PhantomPinned,
    pin::Pin, sync::Arc, time::Duration,
};

struct ExecutionParams<S: Schema> {
    start_payload: StartPayload<S::ScalarValue>,
    config: Arc<ConnectionConfig<S::Context>>,
    schema: S,
}

/// Schema defines the requirements for schemas that can be used for operations. Typically this is
/// just an Arc<RootNode>.
pub trait Schema: Unpin + Clone + Send + Sync + 'static {
    type Context: Unpin + Send + Sync;
    type ScalarValue: ScalarValue + Send + Sync;
    type QueryTypeInfo: Send + Sync;
    type Query: GraphQLTypeAsync<Self::ScalarValue, Context = Self::Context, TypeInfo = Self::QueryTypeInfo>
        + Send;
    type MutationTypeInfo: Send + Sync;
    type Mutation: GraphQLTypeAsync<
            Self::ScalarValue,
            Context = Self::Context,
            TypeInfo = Self::MutationTypeInfo,
        > + Send;
    type SubscriptionTypeInfo: Send + Sync;
    type Subscription: GraphQLSubscriptionType<
            Self::ScalarValue,
            Context = Self::Context,
            TypeInfo = Self::SubscriptionTypeInfo,
        > + Send;

    fn root_node(
        &self,
    ) -> &RootNode<'static, Self::Query, Self::Mutation, Self::Subscription, Self::ScalarValue>;
}

impl<QueryT, MutationT, SubscriptionT, CtxT, S> Schema
    for Arc<RootNode<'static, QueryT, MutationT, SubscriptionT, S>>
where
    QueryT: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT> + Send + 'static,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Unpin + Send + Sync,
    S: ScalarValue + Send + Sync + 'static,
{
    type Context = CtxT;
    type ScalarValue = S;
    type QueryTypeInfo = QueryT::TypeInfo;
    type Query = QueryT;
    type MutationTypeInfo = MutationT::TypeInfo;
    type Mutation = MutationT;
    type SubscriptionTypeInfo = SubscriptionT::TypeInfo;
    type Subscription = SubscriptionT;

    fn root_node(&self) -> &RootNode<'static, QueryT, MutationT, SubscriptionT, S> {
        self
    }
}

/// ConnectionConfig is used to configure the connection once the client sends the ConnectionInit
/// message.
pub struct ConnectionConfig<CtxT> {
    context: CtxT,
    max_in_flight_operations: usize,
    keep_alive_interval: Duration,
}

impl<CtxT> ConnectionConfig<CtxT> {
    /// Constructs the configuration required for a connection to be accepted.
    pub fn new(context: CtxT) -> Self {
        Self {
            context,
            max_in_flight_operations: 0,
            keep_alive_interval: Duration::from_secs(30),
        }
    }

    /// Specifies the maximum number of in-flight operations that a connection can have. If this
    /// number is exceeded, attempting to start more will result in an error. By default, there is
    /// no limit to in-flight operations.
    pub fn with_max_in_flight_operations(mut self, max: usize) -> Self {
        self.max_in_flight_operations = max;
        self
    }

    /// Specifies the interval at which to send keep-alives. Specifying a zero duration will
    /// disable keep-alives. By default, keep-alives are sent every
    /// 30 seconds.
    pub fn with_keep_alive_interval(mut self, interval: Duration) -> Self {
        self.keep_alive_interval = interval;
        self
    }
}

impl<S: Schema> Init<S> for ConnectionConfig<S::Context> {
    type Error = Infallible;
    type Future = future::Ready<Result<Self, Self::Error>>;

    fn init(self, _params: Variables<S::ScalarValue>) -> Self::Future {
        future::ready(Ok(self))
    }
}

enum Reaction<S: Schema> {
    ServerMessage(ServerMessage<S::ScalarValue>),
    Activate {
        config: ConnectionConfig<S::Context>,
        schema: S,
    },
    EndStream,
}

impl<S: Schema> Reaction<S> {
    /// Converts the reaction into a one-item stream.
    fn to_stream(self) -> BoxStream<'static, Self> {
        stream::once(future::ready(self)).boxed()
    }
}

/// Init defines the requirements for types that can provide connection configurations when
/// ConnectionInit messages are received. It is automatically implemented for closures that meet
/// the requirements.
pub trait Init<S: Schema>: Unpin + 'static {
    type Error: Error;
    type Future: Future<Output = Result<ConnectionConfig<S::Context>, Self::Error>> + Send + 'static;

    fn init(self, params: Variables<S::ScalarValue>) -> Self::Future;
}

impl<F, S, Fut, E> Init<S> for F
where
    S: Schema,
    F: FnOnce(Variables<S::ScalarValue>) -> Fut + Unpin + 'static,
    Fut: Future<Output = Result<ConnectionConfig<S::Context>, E>> + Send + 'static,
    E: Error,
{
    type Error = E;
    type Future = Fut;

    fn init(self, params: Variables<S::ScalarValue>) -> Fut {
        self(params)
    }
}

enum ConnectionState<S: Schema, I: Init<S>> {
    /// PreInit is the state before a ConnectionInit message has been accepted.
    PreInit { init: I, schema: S },
    /// Initializing is the state after a ConnectionInit message has been received, but before the
    /// init future has resolved.
    Initializing,
    /// Active is the state after a ConnectionInit message has been accepted.
    Active {
        config: Arc<ConnectionConfig<S::Context>>,
        stoppers: HashMap<String, oneshot::Sender<()>>,
        schema: S,
    },
}

impl<S: Schema, I: Init<S>> ConnectionState<S, I> {
    // Each message we receive results in a stream of zero or more reactions. For example, a
    // ConnectionTerminate message results in a one-item stream with the EndStream reaction.
    fn handle_message(
        &mut self,
        msg: ClientMessage<S::ScalarValue>,
    ) -> BoxStream<'static, Reaction<S>> {
        if let ClientMessage::ConnectionTerminate = msg {
            return Reaction::EndStream.to_stream();
        }

        match self {
            Self::PreInit { .. } => match msg {
                ClientMessage::ConnectionInit { payload } => {
                    match std::mem::replace(self, Self::Initializing) {
                        Self::PreInit { init, schema } => init
                            .init(payload)
                            .map(|r| match r {
                                Ok(config) => {
                                    let keep_alive_interval = config.keep_alive_interval;

                                    let mut s = stream::iter(vec![
                                        Reaction::Activate { config, schema },
                                        Reaction::ServerMessage(ServerMessage::ConnectionAck),
                                    ])
                                    .boxed();

                                    if keep_alive_interval > Duration::from_secs(0) {
                                        s = s
                                            .chain(
                                                Reaction::ServerMessage(
                                                    ServerMessage::ConnectionKeepAlive,
                                                )
                                                .to_stream(),
                                            )
                                            .boxed();
                                        s = s
                                            .chain(stream::unfold((), move |_| async move {
                                                tokio::time::delay_for(keep_alive_interval).await;
                                                Some((
                                                    Reaction::ServerMessage(
                                                        ServerMessage::ConnectionKeepAlive,
                                                    ),
                                                    (),
                                                ))
                                            }))
                                            .boxed();
                                    }

                                    s
                                }
                                Err(e) => stream::iter(vec![
                                    Reaction::ServerMessage(ServerMessage::ConnectionError {
                                        payload: ConnectionErrorPayload {
                                            message: e.to_string(),
                                        },
                                    }),
                                    Reaction::EndStream,
                                ])
                                .boxed(),
                            })
                            .into_stream()
                            .flatten()
                            .boxed(),
                        _ => unreachable!(),
                    }
                }
                _ => stream::empty().boxed(),
            },
            Self::Initializing => stream::empty().boxed(),
            Self::Active {
                config,
                stoppers,
                schema,
            } => {
                match msg {
                    ClientMessage::Start { id, payload } => {
                        if stoppers.contains_key(&id) {
                            // We already have an operation with this id, so we can't start a new
                            // one.
                            return stream::empty().boxed();
                        }

                        // Go ahead and prune canceled stoppers before adding a new one.
                        stoppers.retain(|_, tx| !tx.is_canceled());

                        if config.max_in_flight_operations > 0
                            && stoppers.len() >= config.max_in_flight_operations
                        {
                            // Too many in-flight operations. Just send back a validation error.
                            return stream::iter(vec![
                                Reaction::ServerMessage(ServerMessage::Error {
                                    id: id.clone(),
                                    payload: GraphQLError::ValidationError(vec![RuleError::new(
                                        "Too many in-flight operations.",
                                        &[],
                                    )])
                                    .into(),
                                }),
                                Reaction::ServerMessage(ServerMessage::Complete { id }),
                            ])
                            .boxed();
                        }

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
                            Reaction::ServerMessage(ServerMessage::Complete { id }).to_stream(),
                        );

                        s.boxed()
                    }
                    ClientMessage::Stop { id } => {
                        stoppers.remove(&id);
                        stream::empty().boxed()
                    }
                    _ => stream::empty().boxed(),
                }
            }
        }
    }

    async fn start(id: String, params: ExecutionParams<S>) -> BoxStream<'static, Reaction<S>> {
        // TODO: This could be made more efficient if juniper exposed functionality to allow us to
        // parse and validate the query, determine whether it's a subscription, and then execute
        // it. For now, the query gets parsed and validated twice.

        let params = Arc::new(params);

        // Try to execute this as a query or mutation.
        match juniper::execute(
            &params.start_payload.query,
            params
                .start_payload
                .operation_name
                .as_ref()
                .map(|s| s.as_str()),
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
                .to_stream();
            }
            Err(GraphQLError::IsSubscription) => {}
            Err(e) => {
                return Reaction::ServerMessage(ServerMessage::Error {
                    id: id.clone(),
                    payload: unsafe { ErrorPayload::new_unchecked(Box::new(params.clone()), e) },
                })
                .to_stream()
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
            Result<
                juniper_subscriptions::Connection<'static, S::ScalarValue>,
                GraphQLError<'static>,
            >,
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
                                (*params)
                                    .start_payload
                                    .operation_name
                                    .as_ref()
                                    .map(|s| s.as_str()),
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
                                    payload: unsafe {
                                        ErrorPayload::new_unchecked(Box::new(params.clone()), e)
                                    },
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
                    Poll::Ready(Some((data, errors))) => {
                        return Poll::Ready(Some(Reaction::ServerMessage(ServerMessage::Data {
                            id: id.clone(),
                            payload: DataPayload { data, errors },
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

pub fn serve<St, StT, StE, S, I>(stream: St, schema: S, init: I) -> Serve<St, S, I>
where
    St: Stream<Item = StT> + Unpin,
    StT: TryInto<ClientMessage<S::ScalarValue>, Error = StE>,
    StE: Error,
    S: Schema,
    I: Init<S>,
{
    Serve {
        stream,
        reactions: SelectAll::new(),
        state: ConnectionState::PreInit { init, schema },
    }
}

/// Stream for the serve function.
pub struct Serve<St, S: Schema, I: Init<S>> {
    stream: St,
    reactions: SelectAll<BoxStream<'static, Reaction<S>>>,
    state: ConnectionState<S, I>,
}

impl<St, StT, StE, S, I> Stream for Serve<St, S, I>
where
    St: Stream<Item = StT> + Unpin,
    StT: TryInto<ClientMessage<S::ScalarValue>, Error = StE>,
    StE: Error,
    S: Schema,
    I: Init<S>,
{
    type Item = ServerMessage<S::ScalarValue>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        // Poll the connection for new incoming messages.
        loop {
            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Ready(Some(msg)) => {
                    // We have a new message. Try to parse it and add the reaction stream.
                    let reactions = match msg.try_into() {
                        Ok(msg) => self.state.handle_message(msg),
                        Err(e) => {
                            // If we weren't able to parse the message, just send back an error and
                            // carry on.
                            Reaction::ServerMessage(ServerMessage::ConnectionError {
                                payload: ConnectionErrorPayload {
                                    message: e.to_string(),
                                },
                            })
                            .to_stream()
                        }
                    };
                    self.reactions.push(reactions);
                }
                Poll::Ready(None) => {
                    // The connection stream has ended, so we should end too.
                    return Poll::Ready(None);
                }
                Poll::Pending => break,
            }
        }

        // Poll the reactions for new outgoing messages.
        loop {
            if !self.reactions.is_empty() {
                match Pin::new(&mut self.reactions).poll_next(cx) {
                    Poll::Ready(Some(reaction)) => match reaction {
                        Reaction::ServerMessage(msg) => return Poll::Ready(Some(msg)),
                        Reaction::Activate { config, schema } => {
                            self.state = ConnectionState::Active {
                                config: Arc::new(config),
                                stoppers: HashMap::new(),
                                schema,
                            }
                        }
                        Reaction::EndStream => return Poll::Ready(None),
                    },
                    Poll::Ready(None) => {
                        // In rare cases, the reaction stream may terminate. For example, this will
                        // happen if the first message we receive does not require any reaction. Just
                        // recreate it in that case.
                        self.reactions = SelectAll::new();
                        return Poll::Pending;
                    }
                    Poll::Pending => return Poll::Pending,
                }
            } else {
                return Poll::Pending;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::convert::Infallible;
    use super::*;
    use juniper::{
        futures::channel::mpsc, DefaultScalarValue, EmptyMutation, FieldResult, InputValue,
        RootNode, Value,
    };

    struct Context(i32);

    struct Query;

    #[juniper::graphql_object(Context=Context)]
    impl Query {
        /// context just resolves to the current context.
        async fn context(context: &Context) -> i32 {
            context.0
        }
    }

    struct Subscription;

    #[juniper::graphql_subscription(Context=Context)]
    impl Subscription {
        /// context emits the current context once, then never emits anything else.
        async fn context(context: &Context) -> BoxStream<'static, FieldResult<i32>> {
            stream::once(future::ready(Ok(context.0)))
                .chain(
                    tokio::time::delay_for(Duration::from_secs(10000))
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
        let (tx, rx) = mpsc::unbounded::<ClientMessage>();
        let mut rx = serve(
            rx,
            new_test_schema(),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_secs(0)),
        );

        tx.unbounded_send(ClientMessage::ConnectionInit {
            payload: Variables::default(),
        })
        .unwrap();

        assert_eq!(ServerMessage::ConnectionAck, rx.next().await.unwrap());

        tx.unbounded_send(ClientMessage::Start {
            id: "foo".to_string(),
            payload: StartPayload {
                query: "{context}".to_string(),
                variables: Variables::default(),
                operation_name: None,
            },
        })
        .unwrap();

        assert_eq!(
            ServerMessage::Data {
                id: "foo".to_string(),
                payload: DataPayload {
                    data: Value::Object(
                        [("context", Value::Scalar(DefaultScalarValue::Int(1)))]
                            .iter()
                            .cloned()
                            .collect()
                    ),
                    errors: vec![],
                },
            },
            rx.next().await.unwrap()
        );

        assert_eq!(
            ServerMessage::Complete {
                id: "foo".to_string(),
            },
            rx.next().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_subscription() {
        let (tx, rx) = mpsc::unbounded::<ClientMessage>();
        let mut rx = serve(
            rx,
            new_test_schema(),
            ConnectionConfig::new(Context(1)).with_keep_alive_interval(Duration::from_secs(0)),
        );

        tx.unbounded_send(ClientMessage::ConnectionInit {
            payload: Variables::default(),
        })
        .unwrap();

        assert_eq!(ServerMessage::ConnectionAck, rx.next().await.unwrap());

        tx.unbounded_send(ClientMessage::Start {
            id: "foo".to_string(),
            payload: StartPayload {
                query: "subscription Foo {context}".to_string(),
                variables: Variables::default(),
                operation_name: None,
            },
        })
        .unwrap();

        assert_eq!(
            ServerMessage::Data {
                id: "foo".to_string(),
                payload: DataPayload {
                    data: Value::Object(
                        [("context", Value::scalar(1))]
                            .iter()
                            .cloned()
                            .collect()
                    ),
                    errors: vec![],
                },
            },
            rx.next().await.unwrap()
        );

        tx.unbounded_send(ClientMessage::Stop {
            id: "foo".to_string(),
        })
        .unwrap();

        assert_eq!(
            ServerMessage::Complete {
                id: "foo".to_string(),
            },
            rx.next().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_init_params_ok() {
        let (tx, rx) = mpsc::unbounded::<ClientMessage>();
        let mut rx = serve(
            rx,
            new_test_schema(),
            |params: Variables<DefaultScalarValue>| async move {
                assert_eq!(params.get("foo"), Some(&InputValue::scalar("bar")));
                Ok(ConnectionConfig::new(Context(1))) as Result<_, Infallible>
            },
        );

        tx.unbounded_send(ClientMessage::ConnectionInit {
            payload: [(
                "foo".to_string(),
                InputValue::scalar("bar".to_string())
            )]
            .iter()
            .cloned()
            .collect(),
        })
        .unwrap();

        assert_eq!(ServerMessage::ConnectionAck, rx.next().await.unwrap());
    }
}
