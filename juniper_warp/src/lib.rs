#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(warnings)]

use std::{collections::HashMap, str, sync::Arc};

use anyhow::anyhow;
use futures::{FutureExt as _, TryFutureExt};
use juniper::{
    http::{GraphQLBatchRequest, GraphQLRequest},
    ScalarValue,
};
use tokio::task;
use warp::{body, filters::BoxedFilter, http, hyper::body::Bytes, query, Filter};

/// Makes a filter for GraphQL queries/mutations.
///
/// The `schema` argument is your [`juniper`] schema.
///
/// The `context_extractor` argument should be a filter that provides the GraphQL context required by the schema.
///
/// In order to avoid blocking, this helper will use the `tokio_threadpool` threadpool created by hyper to resolve GraphQL requests.
///
/// # Example
///
/// ```rust
/// # use std::sync::Arc;
/// # use warp::Filter;
/// # use juniper::{graphql_object, EmptyMutation, EmptySubscription, RootNode};
/// # use juniper_warp::make_graphql_filter;
/// #
/// type UserId = String;
/// # #[derive(Debug)]
/// struct AppState(Vec<i64>);
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
/// let schema = RootNode::new(QueryRoot, EmptyMutation::new(), EmptySubscription::new());
///
/// let app_state = Arc::new(AppState(vec![3, 4, 5]));
/// let app_state = warp::any().map(move || app_state.clone());
///
/// let context_extractor = warp::any()
///     .and(warp::header::<String>("authorization"))
///     .and(app_state)
///     .map(|auth_header: String, app_state: Arc<AppState>| {
///         let user_id = auth_header; // we believe them
///         ExampleContext(app_state, user_id)
///     })
///     .boxed();
///
/// let graphql_filter = make_graphql_filter(schema, context_extractor);
///
/// let graphql_endpoint = warp::path("graphql")
///     .and(warp::post())
///     .and(graphql_filter);
/// ```
pub fn make_graphql_filter<Query, Mutation, Subscription, CtxT, S>(
    schema: impl Into<Arc<juniper::RootNode<'static, Query, Mutation, Subscription, S>>>,
    context_extractor: BoxedFilter<(CtxT,)>,
) -> BoxedFilter<(http::Response<Vec<u8>>,)>
where
    Query: juniper::GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = CtxT> + Send + 'static,
    Subscription::TypeInfo: Send + Sync,
    CtxT: Send + Sync + 'static,
    S: ScalarValue + Send + Sync + 'static,
{
    let schema = schema.into();
    let post_json_schema = schema.clone();
    let post_graphql_schema = schema.clone();

    let handle_post_json_request = move |context: CtxT, req: GraphQLBatchRequest<S>| {
        let schema = post_json_schema.clone();
        async move {
            let resp = req.execute(&schema, &context).await;

            Ok::<_, warp::Rejection>(build_response(
                serde_json::to_vec(&resp)
                    .map(|json| (json, resp.is_ok()))
                    .map_err(Into::into),
            ))
        }
    };
    let post_json_filter = warp::post()
        .and(context_extractor.clone())
        .and(body::json())
        .and_then(handle_post_json_request);

    let handle_post_graphql_request = move |context: CtxT, body: Bytes| {
        let schema = post_graphql_schema.clone();
        async move {
            let query = str::from_utf8(body.as_ref())
                .map_err(|e| anyhow!("Request body query is not a valid UTF-8 string: {e}"))?;
            let req = GraphQLRequest::new(query.into(), None, None);

            let resp = req.execute(&schema, &context).await;

            Ok((serde_json::to_vec(&resp)?, resp.is_ok()))
        }
        .then(|res| async { Ok::<_, warp::Rejection>(build_response(res)) })
    };
    let post_graphql_filter = warp::post()
        .and(context_extractor.clone())
        .and(body::bytes())
        .and_then(handle_post_graphql_request);

    let handle_get_request = move |context: CtxT, mut qry: HashMap<String, String>| {
        let schema = schema.clone();
        async move {
            let req = GraphQLRequest::new(
                qry.remove("query")
                    .ok_or_else(|| anyhow!("Missing GraphQL query string in query parameters"))?,
                qry.remove("operation_name"),
                qry.remove("variables")
                    .map(|vs| serde_json::from_str(&vs))
                    .transpose()?,
            );

            let resp = req.execute(&schema, &context).await;

            Ok((serde_json::to_vec(&resp)?, resp.is_ok()))
        }
        .then(|res| async move { Ok::<_, warp::Rejection>(build_response(res)) })
    };
    let get_filter = warp::get()
        .and(context_extractor)
        .and(query::query())
        .and_then(handle_get_request);

    get_filter
        .or(post_json_filter)
        .unify()
        .or(post_graphql_filter)
        .unify()
        .boxed()
}

/// Make a synchronous filter for graphql endpoint.
pub fn make_graphql_filter_sync<Query, Mutation, Subscription, CtxT, S>(
    schema: impl Into<Arc<juniper::RootNode<'static, Query, Mutation, Subscription, S>>>,
    context_extractor: BoxedFilter<(CtxT,)>,
) -> BoxedFilter<(http::Response<Vec<u8>>,)>
where
    Query: juniper::GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    Mutation: juniper::GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    Subscription: juniper::GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    CtxT: Send + Sync + 'static,
    S: ScalarValue + Send + Sync + 'static,
{
    let schema = schema.into();
    let post_json_schema = schema.clone();
    let post_graphql_schema = schema.clone();

    let handle_post_json_request = move |context: CtxT, req: GraphQLBatchRequest<S>| {
        let schema = post_json_schema.clone();
        async move {
            let res = task::spawn_blocking(move || {
                let resp = req.execute_sync(&schema, &context);
                Ok((serde_json::to_vec(&resp)?, resp.is_ok()))
            })
            .await?;

            Ok(build_response(res))
        }
        .map_err(|e: task::JoinError| warp::reject::custom(JoinError(e)))
    };
    let post_json_filter = warp::post()
        .and(context_extractor.clone())
        .and(body::json())
        .and_then(handle_post_json_request);

    let handle_post_graphql_request = move |context: CtxT, body: Bytes| {
        let schema = post_graphql_schema.clone();
        async move {
            let res = task::spawn_blocking(move || {
                let query = str::from_utf8(body.as_ref())
                    .map_err(|e| anyhow!("Request body is not a valid UTF-8 string: {e}"))?;
                let req = GraphQLRequest::new(query.into(), None, None);

                let resp = req.execute_sync(&schema, &context);
                Ok((serde_json::to_vec(&resp)?, resp.is_ok()))
            })
            .await?;

            Ok(build_response(res))
        }
        .map_err(|e: task::JoinError| warp::reject::custom(JoinError(e)))
    };
    let post_graphql_filter = warp::post()
        .and(context_extractor.clone())
        .and(body::bytes())
        .and_then(handle_post_graphql_request);

    let handle_get_request = move |context: CtxT, mut qry: HashMap<String, String>| {
        let schema = schema.clone();
        async move {
            let res = task::spawn_blocking(move || {
                let req = GraphQLRequest::new(
                    qry.remove("query").ok_or_else(|| {
                        anyhow!("Missing GraphQL query string in query parameters")
                    })?,
                    qry.remove("operation_name"),
                    qry.remove("variables")
                        .map(|vs| serde_json::from_str(&vs))
                        .transpose()?,
                );

                let resp = req.execute_sync(&schema, &context);
                Ok((serde_json::to_vec(&resp)?, resp.is_ok()))
            })
            .await?;

            Ok(build_response(res))
        }
        .map_err(|e: task::JoinError| warp::reject::custom(JoinError(e)))
    };
    let get_filter = warp::get()
        .and(context_extractor)
        .and(query::query())
        .and_then(handle_get_request);

    get_filter
        .or(post_json_filter)
        .unify()
        .or(post_graphql_filter)
        .unify()
        .boxed()
}

/// Error raised by `tokio_threadpool` if the thread pool has been shutdown.
///
/// Wrapper type is needed as inner type does not implement `warp::reject::Reject`.
#[derive(Debug)]
pub struct JoinError(task::JoinError);

impl warp::reject::Reject for JoinError {}

fn build_response(response: Result<(Vec<u8>, bool), anyhow::Error>) -> http::Response<Vec<u8>> {
    match response {
        Ok((body, is_ok)) => http::Response::builder()
            .status(if is_ok { 200 } else { 400 })
            .header("content-type", "application/json")
            .body(body)
            .expect("response is valid"),
        Err(_) => http::Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(Vec::new())
            .expect("status code is valid"),
    }
}

/// Create a filter that replies with an HTML page containing GraphiQL. This does not handle routing, so you can mount it on any endpoint.
///
/// For example:
///
/// ```
/// # use warp::Filter;
/// # use juniper_warp::graphiql_filter;
/// #
/// let graphiql_route = warp::path("graphiql").and(graphiql_filter("/graphql",
/// None));
/// ```
///
/// Or with subscriptions support, provide the subscriptions endpoint URL:
///
/// ```
/// # use warp::Filter;
/// # use juniper_warp::graphiql_filter;
/// #
/// let graphiql_route = warp::path("graphiql").and(graphiql_filter("/graphql",
/// Some("ws://localhost:8080/subscriptions")));
/// ```
pub fn graphiql_filter(
    graphql_endpoint_url: &'static str,
    subscriptions_endpoint: Option<&'static str>,
) -> warp::filters::BoxedFilter<(http::Response<Vec<u8>>,)> {
    warp::any()
        .map(move || graphiql_response(graphql_endpoint_url, subscriptions_endpoint))
        .boxed()
}

fn graphiql_response(
    graphql_endpoint_url: &'static str,
    subscriptions_endpoint: Option<&'static str>,
) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .header("content-type", "text/html;charset=utf-8")
        .body(
            juniper::http::graphiql::graphiql_source(graphql_endpoint_url, subscriptions_endpoint)
                .into_bytes(),
        )
        .expect("response is valid")
}

/// Create a filter that replies with an HTML page containing GraphQL Playground. This does not handle routing, so you can mount it on any endpoint.
pub fn playground_filter(
    graphql_endpoint_url: &'static str,
    subscriptions_endpoint_url: Option<&'static str>,
) -> warp::filters::BoxedFilter<(http::Response<Vec<u8>>,)> {
    warp::any()
        .map(move || playground_response(graphql_endpoint_url, subscriptions_endpoint_url))
        .boxed()
}

fn playground_response(
    graphql_endpoint_url: &'static str,
    subscriptions_endpoint_url: Option<&'static str>,
) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .header("content-type", "text/html;charset=utf-8")
        .body(
            juniper::http::playground::playground_source(
                graphql_endpoint_url,
                subscriptions_endpoint_url,
            )
            .into_bytes(),
        )
        .expect("response is valid")
}

#[cfg(feature = "subscriptions")]
/// `juniper_warp` subscriptions handler implementation.
pub mod subscriptions {
    use std::{convert::Infallible, fmt, sync::Arc};

    use juniper::{
        futures::{
            future::{self, Either},
            sink::SinkExt,
            stream::StreamExt,
        },
        GraphQLSubscriptionType, GraphQLTypeAsync, RootNode, ScalarValue,
    };
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
        schema: impl Into<Arc<juniper::RootNode<'static, Query, Mutation, Subscription, S>>>,
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
        root_node: Arc<RootNode<'static, Query, Mutation, Subscription, S>>,
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
        root_node: Arc<RootNode<'static, Query, Mutation, Subscription, S>>,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::{http, test::request};

    #[test]
    fn graphiql_response_does_not_panic() {
        graphiql_response("/abcd", None);
    }

    #[tokio::test]
    async fn graphiql_endpoint_matches() {
        let filter = warp::get()
            .and(warp::path("graphiql"))
            .and(graphiql_filter("/graphql", None));
        let result = request()
            .method("GET")
            .path("/graphiql")
            .header("accept", "text/html")
            .filter(&filter)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn graphiql_endpoint_returns_graphiql_source() {
        let filter = warp::get()
            .and(warp::path("dogs-api"))
            .and(warp::path("graphiql"))
            .and(graphiql_filter("/dogs-api/graphql", None));
        let response = request()
            .method("GET")
            .path("/dogs-api/graphiql")
            .header("accept", "text/html")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/html;charset=utf-8"
        );
        let body = String::from_utf8(response.body().to_vec()).unwrap();

        assert!(body.contains("var JUNIPER_URL = '/dogs-api/graphql';"));
    }

    #[tokio::test]
    async fn graphiql_endpoint_with_subscription_matches() {
        let filter = warp::get().and(warp::path("graphiql")).and(graphiql_filter(
            "/graphql",
            Some("ws:://localhost:8080/subscriptions"),
        ));
        let result = request()
            .method("GET")
            .path("/graphiql")
            .header("accept", "text/html")
            .filter(&filter)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn playground_endpoint_matches() {
        let filter = warp::get()
            .and(warp::path("playground"))
            .and(playground_filter("/graphql", Some("/subscripitons")));

        let result = request()
            .method("GET")
            .path("/playground")
            .header("accept", "text/html")
            .filter(&filter)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn playground_endpoint_returns_playground_source() {
        let filter = warp::get()
            .and(warp::path("dogs-api"))
            .and(warp::path("playground"))
            .and(playground_filter(
                "/dogs-api/graphql",
                Some("/dogs-api/subscriptions"),
            ));
        let response = request()
            .method("GET")
            .path("/dogs-api/playground")
            .header("accept", "text/html")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/html;charset=utf-8"
        );
        let body = String::from_utf8(response.body().to_vec()).unwrap();

        assert!(body.contains(
            "endpoint: '/dogs-api/graphql', subscriptionEndpoint: '/dogs-api/subscriptions'",
        ));
    }

    #[tokio::test]
    async fn graphql_handler_works_json_post() {
        use juniper::{
            tests::fixtures::starwars::schema::{Database, Query},
            EmptyMutation, EmptySubscription, RootNode,
        };

        type Schema =
            juniper::RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;

        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let state = warp::any().map(Database::new);
        let filter = warp::path("graphql2").and(make_graphql_filter(schema, state.boxed()));

        let response = request()
            .method("POST")
            .path("/graphql2")
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .body(r#"{ "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" }"#)
            .reply(&filter)
            .await;

        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json",
        );
        assert_eq!(
            String::from_utf8(response.body().to_vec()).unwrap(),
            r#"{"data":{"hero":{"name":"R2-D2"}}}"#
        );
    }

    #[tokio::test]
    async fn batch_requests_work() {
        use juniper::{
            tests::fixtures::starwars::schema::{Database, Query},
            EmptyMutation, EmptySubscription, RootNode,
        };

        type Schema =
            juniper::RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;

        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let state = warp::any().map(Database::new);
        let filter = warp::path("graphql2").and(make_graphql_filter(schema, state.boxed()));

        let response = request()
            .method("POST")
            .path("/graphql2")
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .body(
                r#"[
                     { "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" },
                     { "variables": null, "query": "{ hero(episode: EMPIRE) { id name } }" }
                 ]"#,
            )
            .reply(&filter)
            .await;

        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(
            String::from_utf8(response.body().to_vec()).unwrap(),
            r#"[{"data":{"hero":{"name":"R2-D2"}}},{"data":{"hero":{"id":"1000","name":"Luke Skywalker"}}}]"#
        );
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json",
        );
    }

    #[test]
    fn batch_request_deserialization_can_fail() {
        let json = r#"blah"#;
        let result: Result<GraphQLBatchRequest, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod tests_http_harness {
    use juniper::{
        http::tests::{run_http_test_suite, HttpIntegration, TestResponse},
        tests::fixtures::starwars::schema::{Database, Query},
        EmptyMutation, EmptySubscription, RootNode,
    };
    use warp::{
        filters::{path, BoxedFilter},
        Filter,
    };

    use super::*;

    struct TestWarpIntegration {
        filter: BoxedFilter<(http::Response<Vec<u8>>,)>,
    }

    impl TestWarpIntegration {
        fn new(is_sync: bool) -> Self {
            let schema = RootNode::new(
                Query,
                EmptyMutation::<Database>::new(),
                EmptySubscription::<Database>::new(),
            );
            let state = warp::any().map(Database::new);

            let filter = path::end().and(if is_sync {
                make_graphql_filter_sync(schema, state.boxed())
            } else {
                make_graphql_filter(schema, state.boxed())
            });
            Self {
                filter: filter.boxed(),
            }
        }

        fn make_request(&self, req: warp::test::RequestBuilder) -> TestResponse {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio::Runtime");
            make_test_response(rt.block_on(async move {
                req.filter(&self.filter).await.unwrap_or_else(|rejection| {
                    let code = if rejection.is_not_found() {
                        http::StatusCode::NOT_FOUND
                    } else if let Some(body::BodyDeserializeError { .. }) = rejection.find() {
                        http::StatusCode::BAD_REQUEST
                    } else {
                        http::StatusCode::INTERNAL_SERVER_ERROR
                    };
                    http::Response::builder()
                        .status(code)
                        .header("content-type", "application/json")
                        .body(Vec::new())
                        .unwrap()
                })
            }))
        }
    }

    impl HttpIntegration for TestWarpIntegration {
        fn get(&self, url: &str) -> TestResponse {
            use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
            use url::Url;

            /// https://url.spec.whatwg.org/#query-state
            const QUERY_ENCODE_SET: &AsciiSet =
                &CONTROLS.add(b' ').add(b'"').add(b'#').add(b'<').add(b'>');

            let url = Url::parse(&format!("http://localhost:3000{url}")).expect("url to parse");

            let url: String = utf8_percent_encode(url.query().unwrap_or(""), QUERY_ENCODE_SET)
                .collect::<Vec<_>>()
                .join("");

            self.make_request(
                warp::test::request()
                    .method("GET")
                    .path(&format!("/?{url}")),
            )
        }

        fn post_json(&self, url: &str, body: &str) -> TestResponse {
            self.make_request(
                warp::test::request()
                    .method("POST")
                    .header("content-type", "application/json; charset=utf-8")
                    .path(url)
                    .body(body),
            )
        }

        fn post_graphql(&self, url: &str, body: &str) -> TestResponse {
            self.make_request(
                warp::test::request()
                    .method("POST")
                    .header("content-type", "application/graphql; charset=utf-8")
                    .path(url)
                    .body(body),
            )
        }
    }

    fn make_test_response(resp: http::Response<Vec<u8>>) -> TestResponse {
        TestResponse {
            status_code: resp.status().as_u16() as i32,
            body: Some(String::from_utf8(resp.body().to_owned()).unwrap()),
            content_type: resp
                .headers()
                .get("content-type")
                .expect("missing content-type header in warp response")
                .to_str()
                .expect("invalid content-type string")
                .into(),
        }
    }

    #[test]
    fn test_warp_integration() {
        run_http_test_suite(&TestWarpIntegration::new(false));
    }

    #[test]
    fn test_sync_warp_integration() {
        run_http_test_suite(&TestWarpIntegration::new(true));
    }
}
