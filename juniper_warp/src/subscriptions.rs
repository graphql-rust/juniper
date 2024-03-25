//! GraphQL subscriptions handler implementation.

use std::{convert::Infallible, fmt, sync::Arc};

use futures::{
    future::{self, Either},
    sink::SinkExt as _,
    stream::StreamExt as _,
};
use juniper::{GraphQLSubscriptionType, GraphQLTypeAsync, RootNode, ScalarValue};
use juniper_graphql_ws::{graphql_transport_ws, graphql_ws};
use warp::{filters::BoxedFilter, reply::Reply, Filter as _};

struct Message(warp::ws::Message);

impl<S: ScalarValue> TryFrom<Message> for graphql_ws::ClientMessage<S> {
    type Error = serde_json::Error;

    fn try_from(msg: Message) -> serde_json::Result<Self> {
        if msg.0.is_close() {
            Ok(Self::ConnectionTerminate)
        } else {
            serde_json::from_slice(msg.0.as_bytes())
        }
    }
}

impl<S: ScalarValue> TryFrom<Message> for graphql_transport_ws::Input<S> {
    type Error = serde_json::Error;

    fn try_from(msg: Message) -> serde_json::Result<Self> {
        if msg.0.is_close() {
            Ok(Self::Close)
        } else {
            serde_json::from_slice(msg.0.as_bytes()).map(Self::Message)
        }
    }
}

/// Errors that can happen while serving a connection.
#[derive(Debug)]
pub enum Error {
    /// Errors that can happen in Warp while serving a connection.
    Warp(warp::Error),

    /// Errors that can happen while serializing outgoing messages. Note that errors that occur
    /// while deserializing incoming messages are handled internally by the protocol.
    Serde(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warp(e) => write!(f, "`warp` error: {e}"),
            Self::Serde(e) => write!(f, "`serde` error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<warp::Error> for Error {
    fn from(err: warp::Error) -> Self {
        Self::Warp(err)
    }
}

impl From<Infallible> for Error {
    fn from(_err: Infallible) -> Self {
        unreachable!()
    }
}

/// Makes a filter for GraphQL subscriptions.
///
/// This filter auto-selects between the
/// [legacy `graphql-ws` GraphQL over WebSocket Protocol][old] and the
/// [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], based on the
/// `Sec-Websocket-Protocol` HTTP header value.
///
/// The `schema` argument is your [`juniper`] schema.
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
/// # use std::{convert::Infallible, pin::Pin, sync::Arc, time::Duration};
/// #
/// # use futures::Stream;
/// # use juniper::{graphql_object, graphql_subscription, EmptyMutation, RootNode};
/// # use juniper_graphql_ws::ConnectionConfig;
/// # use juniper_warp::make_graphql_filter;
/// # use warp::Filter as _;
/// #
/// type UserId = String;
/// # #[derive(Debug)]
/// struct AppState(Vec<i64>);
/// #[derive(Clone)]
/// struct ExampleContext(Arc<AppState>, UserId);
/// # impl juniper::Context for ExampleContext {}
///
/// struct QueryRoot;
///
/// #[graphql_object(context = ExampleContext)]
/// impl QueryRoot {
///     fn say_hello(context: &ExampleContext) -> String {
///         format!(
///             "good morning {}, the app state is {:?}",
///             context.1,
///             context.0,
///         )
///     }
/// }
///
/// type StringsStream = Pin<Box<dyn Stream<Item = String> + Send>>;
///
/// struct SubscriptionRoot;
///
/// #[graphql_subscription(context = ExampleContext)]
/// impl SubscriptionRoot {
///     async fn say_hellos(context: &ExampleContext) -> StringsStream {
///         let mut interval = tokio::time::interval(Duration::from_secs(1));
///         let context = context.clone();
///         Box::pin(async_stream::stream! {
///             let mut counter = 0;
///             while counter < 5 {
///                 counter += 1;
///                 interval.tick().await;
///                 yield format!(
///                     "{counter}: good morning {}, the app state is {:?}",
///                      context.1,
///                      context.0,
///                 )
///             }
///         })
///     }
/// }
///
/// let schema = Arc::new(RootNode::new(QueryRoot, EmptyMutation::new(), SubscriptionRoot));
/// let app_state = Arc::new(AppState(vec![3, 4, 5]));
/// let app_state_for_ws = app_state.clone();
///
/// let context_extractor = warp::any()
///     .and(warp::header::<String>("authorization"))
///     .and(warp::any().map(move || app_state.clone()))
///     .map(|auth_header: String, app_state: Arc<AppState>| {
///         let user_id = auth_header; // we believe them
///         ExampleContext(app_state, user_id)
///     })
///     .boxed();
///
/// let graphql_endpoint = (warp::path("graphql")
///         .and(warp::post())
///         .and(make_graphql_filter(schema.clone(), context_extractor)))
///     .or(warp::path("subscriptions")
///         .and(juniper_warp::subscriptions::make_ws_filter(
///             schema,
///             move |variables: juniper::Variables| {
///                 let user_id = variables
///                     .get("authorization")
///                     .map(ToString::to_string)
///                     .unwrap_or_default(); // we believe them
///                 async move {
///                     Ok::<_, Infallible>(ConnectionConfig::new(
///                         ExampleContext(app_state_for_ws.clone(), user_id),
///                     ))
///                 }
///             },
///         )));
/// ```
///
/// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
/// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
pub fn make_ws_filter<Query, Mutation, Subscription, CtxT, S, I>(
    schema: impl Into<Arc<RootNode<Query, Mutation, Subscription, S>>>,
    init: I,
) -> BoxedFilter<(impl Reply,)>
where
    Query: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: GraphQLSubscriptionType<S, Context = CtxT> + Send + 'static,
    Subscription::TypeInfo: Send + Sync,
    CtxT: Unpin + Send + Sync + 'static,
    S: ScalarValue + Send + Sync + 'static,
    I: juniper_graphql_ws::Init<S, CtxT> + Clone + Send + Sync,
{
    let schema = schema.into();

    warp::ws()
        .and(warp::filters::header::value("sec-websocket-protocol"))
        .map(move |ws: warp::ws::Ws, subproto| {
            let schema = schema.clone();
            let init = init.clone();

            let is_legacy = subproto == "graphql-ws";

            warp::reply::with_header(
                ws.on_upgrade(move |ws| async move {
                    if is_legacy {
                        serve_graphql_ws(ws, schema, init).await
                    } else {
                        serve_graphql_transport_ws(ws, schema, init).await
                    }
                    .unwrap_or_else(|e| {
                        log::error!("GraphQL over WebSocket Protocol error: {e}");
                    })
                }),
                "sec-websocket-protocol",
                if is_legacy {
                    "graphql-ws"
                } else {
                    "graphql-transport-ws"
                },
            )
        })
        .boxed()
}

/// Serves the [legacy `graphql-ws` GraphQL over WebSocket Protocol][old].
///
/// The `init` argument is used to provide the context and additional configuration for
/// connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the context and
/// configuration are already known, or it can be a closure that gets executed asynchronously
/// when the client sends the `GQL_CONNECTION_INIT` message. Using a closure allows to perform
/// an authentication based on the parameters provided by a client.
///
/// > __WARNING__: This protocol has been deprecated in favor of the
///                [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], which is
///                provided by the [`serve_graphql_transport_ws()`] function.
///
/// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
/// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
pub async fn serve_graphql_ws<Query, Mutation, Subscription, CtxT, S, I>(
    websocket: warp::ws::WebSocket,
    root_node: Arc<RootNode<Query, Mutation, Subscription, S>>,
    init: I,
) -> Result<(), Error>
where
    Query: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: GraphQLSubscriptionType<S, Context = CtxT> + Send + 'static,
    Subscription::TypeInfo: Send + Sync,
    CtxT: Unpin + Send + Sync + 'static,
    S: ScalarValue + Send + Sync + 'static,
    I: juniper_graphql_ws::Init<S, CtxT> + Send,
{
    let (ws_tx, ws_rx) = websocket.split();
    let (s_tx, s_rx) =
        graphql_ws::Connection::new(juniper_graphql_ws::ArcSchema(root_node), init).split();

    let ws_rx = ws_rx.map(|r| r.map(Message));
    let s_rx = s_rx.map(|msg| {
        serde_json::to_string(&msg)
            .map(warp::ws::Message::text)
            .map_err(Error::Serde)
    });

    match future::select(
        ws_rx.forward(s_tx.sink_err_into()),
        s_rx.forward(ws_tx.sink_err_into()),
    )
    .await
    {
        Either::Left((r, _)) => r.map_err(|e| e.into()),
        Either::Right((r, _)) => r,
    }
}

/// Serves the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new].
///
/// The `init` argument is used to provide the context and additional configuration for
/// connections. This can be a [`juniper_graphql_ws::ConnectionConfig`] if the context and
/// configuration are already known, or it can be a closure that gets executed asynchronously
/// when the client sends the `ConnectionInit` message. Using a closure allows to perform an
/// authentication based on the parameters provided by a client.
///
/// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
pub async fn serve_graphql_transport_ws<Query, Mutation, Subscription, CtxT, S, I>(
    websocket: warp::ws::WebSocket,
    root_node: Arc<RootNode<Query, Mutation, Subscription, S>>,
    init: I,
) -> Result<(), Error>
where
    Query: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: GraphQLSubscriptionType<S, Context = CtxT> + Send + 'static,
    Subscription::TypeInfo: Send + Sync,
    CtxT: Unpin + Send + Sync + 'static,
    S: ScalarValue + Send + Sync + 'static,
    I: juniper_graphql_ws::Init<S, CtxT> + Send,
{
    let (ws_tx, ws_rx) = websocket.split();
    let (s_tx, s_rx) =
        graphql_transport_ws::Connection::new(juniper_graphql_ws::ArcSchema(root_node), init)
            .split();

    let ws_rx = ws_rx.map(|r| r.map(Message));
    let s_rx = s_rx.map(|output| match output {
        graphql_transport_ws::Output::Message(msg) => serde_json::to_string(&msg)
            .map(warp::ws::Message::text)
            .map_err(Error::Serde),
        graphql_transport_ws::Output::Close { code, message } => {
            Ok(warp::ws::Message::close_with(code, message))
        }
    });

    match future::select(
        ws_rx.forward(s_tx.sink_err_into()),
        s_rx.forward(ws_tx.sink_err_into()),
    )
    .await
    {
        Either::Left((r, _)) => r.map_err(|e| e.into()),
        Either::Right((r, _)) => r,
    }
}
