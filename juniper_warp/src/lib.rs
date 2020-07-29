/*!

# juniper_warp

This repository contains the [warp][warp] web server integration for
[Juniper][Juniper], a [GraphQL][GraphQL] implementation for Rust.

## Documentation

For documentation, including guides and examples, check out [Juniper][Juniper].

A basic usage example can also be found in the [Api documentation][documentation].

## Examples

Check [examples/warp_server][example] for example code of a working warp
server with GraphQL handlers.

## Links

* [Juniper][Juniper]
* [Api Reference][documentation]
* [warp][warp]

## License

This project is under the BSD-2 license.

Check the LICENSE file for details.

[warp]: https://github.com/seanmonstar/warp
[Juniper]: https://github.com/graphql-rust/juniper
[GraphQL]: http://graphql.org
[documentation]: https://docs.rs/juniper_warp
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_warp/examples/warp_server

*/

#![deny(missing_docs)]
#![deny(warnings)]
#![doc(html_root_url = "https://docs.rs/juniper_warp/0.2.0")]

use anyhow::anyhow;
use bytes::Bytes;
use futures::{FutureExt as _, TryFutureExt};
use juniper::{
    http::{GraphQLBatchRequest, GraphQLRequest},
    ScalarValue,
};
use std::{collections::HashMap, str, sync::Arc};
use tokio::task;
use warp::{body, filters::BoxedFilter, header, http, query, Filter};

/// Make a filter for graphql queries/mutations.
///
/// The `schema` argument is your juniper schema.
///
/// The `context_extractor` argument should be a filter that provides the GraphQL context required by the schema.
///
/// In order to avoid blocking, this helper will use the `tokio_threadpool` threadpool created by hyper to resolve GraphQL requests.
///
/// Example:
///
/// ```
/// # extern crate juniper_warp;
/// # extern crate juniper;
/// # extern crate warp;
/// #
/// # use std::sync::Arc;
/// # use warp::Filter;
/// # use juniper::{EmptyMutation, EmptySubscription, RootNode};
/// # use juniper_warp::make_graphql_filter;
/// #
/// type UserId = String;
/// # #[derive(Debug)]
/// struct AppState(Vec<i64>);
/// struct ExampleContext(Arc<AppState>, UserId);
///
/// struct QueryRoot;
///
/// #[juniper::graphql_object(
///    Context = ExampleContext
/// )]
/// impl QueryRoot {
///     fn say_hello(context: &ExampleContext) -> String {
///         format!(
///             "good morning {}, the app state is {:?}",
///             context.1,
///             context.0
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
    schema: juniper::RootNode<'static, Query, Mutation, Subscription, S>,
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
    let schema = Arc::new(schema);
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
        .and(header::exact_ignore_case(
            "content-type",
            "application/json",
        ))
        .and(context_extractor.clone())
        .and(body::json())
        .and_then(handle_post_json_request);

    let handle_post_graphql_request = move |context: CtxT, body: Bytes| {
        let schema = post_graphql_schema.clone();
        async move {
            let query = str::from_utf8(body.as_ref())
                .map_err(|e| anyhow!("Request body query is not a valid UTF-8 string: {}", e))?;
            let req = GraphQLRequest::new(query.into(), None, None);

            let resp = req.execute(&schema, &context).await;

            Ok((serde_json::to_vec(&resp)?, resp.is_ok()))
        }
        .then(|res| async { Ok::<_, warp::Rejection>(build_response(res)) })
    };
    let post_graphql_filter = warp::post()
        .and(header::exact_ignore_case(
            "content-type",
            "application/graphql",
        ))
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
    schema: juniper::RootNode<'static, Query, Mutation, Subscription, S>,
    context_extractor: BoxedFilter<(CtxT,)>,
) -> BoxedFilter<(http::Response<Vec<u8>>,)>
where
    Query: juniper::GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    Mutation: juniper::GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    Subscription: juniper::GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    CtxT: Send + Sync + 'static,
    S: ScalarValue + Send + Sync + 'static,
{
    let schema = Arc::new(schema);
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
        .and(header::exact_ignore_case(
            "content-type",
            "application/json",
        ))
        .and(context_extractor.clone())
        .and(body::json())
        .and_then(handle_post_json_request);

    let handle_post_graphql_request = move |context: CtxT, body: Bytes| {
        let schema = post_graphql_schema.clone();
        async move {
            let res = task::spawn_blocking(move || {
                let query = str::from_utf8(body.as_ref())
                    .map_err(|e| anyhow!("Request body is not a valid UTF-8 string: {}", e))?;
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
        .and(header::exact_ignore_case(
            "content-type",
            "application/graphql",
        ))
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
/// # extern crate warp;
/// # extern crate juniper_warp;
/// #
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
/// # extern crate warp;
/// # extern crate juniper_warp;
/// #
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

/// `juniper_warp` subscriptions handler implementation.
/// Cannot be merged to `juniper_warp` yet as GraphQL over WS[1]
/// is not fully supported in current implementation.
///
/// *Note: this implementation is in an alpha state.*
///
/// [1]: https://github.com/apollographql/subscriptions-transport-ws/blob/master/PROTOCOL.md
#[cfg(feature = "subscriptions")]
pub mod subscriptions {
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    use anyhow::anyhow;
    use futures::{channel::mpsc, Future, StreamExt as _, TryFutureExt as _, TryStreamExt as _};
    use juniper::{http::GraphQLRequest, InputValue, ScalarValue, SubscriptionCoordinator as _};
    use juniper_subscriptions::Coordinator;
    use serde::{Deserialize, Serialize};
    use warp::ws::Message;

    /// Listen to incoming messages and do one of the following:
    ///  - execute subscription and return values from stream
    ///  - stop stream and close ws connection
    #[allow(dead_code)]
    pub fn graphql_subscriptions<Query, Mutation, Subscription, CtxT, S>(
        websocket: warp::ws::WebSocket,
        coordinator: Arc<Coordinator<'static, Query, Mutation, Subscription, CtxT, S>>,
        context: CtxT,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send
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
        let (sink_tx, sink_rx) = websocket.split();
        let (ws_tx, ws_rx) = mpsc::unbounded();
        tokio::task::spawn(
            ws_rx
                .take_while(|v: &Option<_>| futures::future::ready(v.is_some()))
                .map(|x| x.unwrap())
                .forward(sink_tx),
        );

        let context = Arc::new(context);
        let got_close_signal = Arc::new(AtomicBool::new(false));
        let got_close_signal2 = got_close_signal.clone();

        struct SubscriptionState {
            should_stop: AtomicBool,
        }
        let subscription_states = HashMap::<String, Arc<SubscriptionState>>::new();

        sink_rx
            .map_err(move |e| {
                got_close_signal2.store(true, Ordering::Relaxed);
                anyhow!("Websocket error: {}", e)
            })
            .try_fold(subscription_states, move |mut subscription_states, msg| {
                let coordinator = coordinator.clone();
                let context = context.clone();
                let got_close_signal = got_close_signal.clone();
                let ws_tx = ws_tx.clone();

                async move {
                    if msg.is_close() {
                        return Ok(subscription_states);
                    }

                    let msg = msg
                        .to_str()
                        .map_err(|_| anyhow!("Non-text messages are not accepted"))?;
                    let request: WsPayload<S> = serde_json::from_str(msg)
                        .map_err(|e| anyhow!("Invalid WsPayload: {}", e))?;

                    match request.type_name.as_str() {
                        "connection_init" => {}
                        "start" => {
                            if got_close_signal.load(Ordering::Relaxed) {
                                return Ok(subscription_states);
                            }

                            let request_id = request.id.clone().unwrap_or("1".to_owned());

                            if let Some(existing) = subscription_states.get(&request_id) {
                                existing.should_stop.store(true, Ordering::Relaxed);
                            }
                            let state = Arc::new(SubscriptionState {
                                should_stop: AtomicBool::new(false),
                            });
                            subscription_states.insert(request_id.clone(), state.clone());

                            let ws_tx = ws_tx.clone();

                            if let Some(ref payload) = request.payload {
                                if payload.query.is_none() {
                                    return Err(anyhow!("Query not found"));
                                }
                            } else {
                                return Err(anyhow!("Payload not found"));
                            }

                            tokio::task::spawn(async move {
                                let payload = request.payload.unwrap();

                                let graphql_request = GraphQLRequest::<S>::new(
                                    payload.query.unwrap(),
                                    None,
                                    payload.variables,
                                );

                                let values_stream = match coordinator
                                    .subscribe(&graphql_request, &context)
                                    .await
                                {
                                    Ok(s) => s,
                                    Err(err) => {
                                        let _ =
                                            ws_tx.unbounded_send(Some(Ok(Message::text(format!(
                                                r#"{{"type":"error","id":"{}","payload":{}}}"#,
                                                request_id,
                                                serde_json::ser::to_string(&err).unwrap_or(
                                                    "Error deserializing GraphQLError".to_owned()
                                                )
                                            )))));

                                        let close_message = format!(
                                            r#"{{"type":"complete","id":"{}","payload":null}}"#,
                                            request_id
                                        );
                                        let _ = ws_tx
                                            .unbounded_send(Some(Ok(Message::text(close_message))));
                                        // close channel
                                        let _ = ws_tx.unbounded_send(None);
                                        return;
                                    }
                                };

                                values_stream
                                    .take_while(move |response| {
                                        let request_id = request_id.clone();
                                        let should_stop = state.should_stop.load(Ordering::Relaxed)
                                            || got_close_signal.load(Ordering::Relaxed);
                                        if !should_stop {
                                            let mut response_text = serde_json::to_string(
                                                &response,
                                            )
                                            .unwrap_or("Error deserializing response".to_owned());

                                            response_text = format!(
                                                r#"{{"type":"data","id":"{}","payload":{} }}"#,
                                                request_id, response_text
                                            );

                                            let _ = ws_tx.unbounded_send(Some(Ok(Message::text(
                                                response_text,
                                            ))));
                                        }

                                        async move { !should_stop }
                                    })
                                    .for_each(|_| async {})
                                    .await;
                            });
                        }
                        "stop" => {
                            let request_id = request.id.unwrap_or("1".to_owned());
                            if let Some(existing) = subscription_states.get(&request_id) {
                                existing.should_stop.store(true, Ordering::Relaxed);
                                subscription_states.remove(&request_id);
                            }

                            let close_message = format!(
                                r#"{{"type":"complete","id":"{}","payload":null}}"#,
                                request_id
                            );
                            let _ = ws_tx.unbounded_send(Some(Ok(Message::text(close_message))));

                            // close channel
                            let _ = ws_tx.unbounded_send(None);
                        }
                        _ => {}
                    }

                    Ok(subscription_states)
                }
            })
            .map_ok(|_| ())
    }

    #[derive(Deserialize)]
    #[serde(bound = "GraphQLPayload<S>: Deserialize<'de>")]
    struct WsPayload<S>
    where
        S: ScalarValue + Send + Sync,
    {
        id: Option<String>,
        #[serde(rename(deserialize = "type"))]
        type_name: String,
        payload: Option<GraphQLPayload<S>>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(bound = "InputValue<S>: Deserialize<'de>")]
    struct GraphQLPayload<S>
    where
        S: ScalarValue + Send + Sync,
    {
        variables: Option<InputValue<S>>,
        extensions: Option<HashMap<String, String>>,
        #[serde(rename(deserialize = "operationName"))]
        operaton_name: Option<String>,
        query: Option<String>,
    }

    #[derive(Serialize)]
    struct Output {
        data: String,
        variables: String,
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

        assert!(body.contains("<script>var GRAPHQL_URL = '/dogs-api/graphql';</script>"));
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

        assert!(body.contains("GraphQLPlayground.init(root, { endpoint: '/dogs-api/graphql', subscriptionEndpoint: '/dogs-api/subscriptions' })"));
    }

    #[tokio::test]
    async fn graphql_handler_works_json_post() {
        use juniper::{
            tests::fixtures::starwars::{model::Database, schema::Query},
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
            .body(r##"{ "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" }"##)
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
            tests::fixtures::starwars::{model::Database, schema::Query},
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
                r##"[
                     { "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" },
                     { "variables": null, "query": "{ hero(episode: EMPIRE) { id name } }" }
                 ]"##,
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
    use super::*;
    use juniper::{
        http::tests::{run_http_test_suite, HttpIntegration, TestResponse},
        tests::fixtures::starwars::{model::Database, schema::Query},
        EmptyMutation, EmptySubscription, RootNode,
    };
    use warp::{
        self,
        filters::{path, BoxedFilter},
        Filter,
    };

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
            let state = warp::any().map(move || Database::new());

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
            let mut rt = tokio::runtime::Runtime::new().expect("Failed to create tokio::Runtime");
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

            let url = Url::parse(&format!("http://localhost:3000{}", url)).expect("url to parse");

            let url: String = utf8_percent_encode(url.query().unwrap_or(""), QUERY_ENCODE_SET)
                .into_iter()
                .collect::<Vec<_>>()
                .join("");

            self.make_request(
                warp::test::request()
                    .method("GET")
                    .path(&format!("/?{}", url)),
            )
        }

        fn post_json(&self, url: &str, body: &str) -> TestResponse {
            self.make_request(
                warp::test::request()
                    .method("POST")
                    .header("content-type", "application/json")
                    .path(url)
                    .body(body),
            )
        }

        fn post_graphql(&self, url: &str, body: &str) -> TestResponse {
            self.make_request(
                warp::test::request()
                    .method("POST")
                    .header("content-type", "application/graphql")
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
                .to_owned(),
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
