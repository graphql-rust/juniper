#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

mod response;
#[cfg(feature = "subscriptions")]
pub mod subscriptions;

use std::{collections::HashMap, fmt, str, sync::Arc};

use juniper::{
    http::{GraphQLBatchRequest, GraphQLRequest},
    ScalarValue,
};
use tokio::task;
use warp::{
    body::{self, BodyDeserializeError},
    http::{self, StatusCode},
    hyper::body::Bytes,
    query,
    reject::{self, Reject, Rejection},
    reply::{self, Reply},
    Filter,
};

use self::response::JuniperResponse;

/// Makes a [`Filter`] for handling GraphQL queries/mutations.
///
/// The `schema` argument is your [`juniper`] schema.
///
/// The `context_extractor` argument should be a [`Filter`] that provides the GraphQL context,
/// required by the `schema`.
///
/// # Example
///
/// ```rust
/// # use std::sync::Arc;
/// #
/// # use juniper::{graphql_object, EmptyMutation, EmptySubscription, RootNode};
/// # use juniper_warp::make_graphql_filter;
/// # use warp::Filter as _;
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
///     });
///
/// let graphql_endpoint = warp::path("graphql")
///     .and(make_graphql_filter(schema, context_extractor));
/// ```
///
/// # Fallible `context_extractor`
///
/// > __WARNING__: In case the `context_extractor` is fallible (e.g. implements
/// >              [`Filter`]`<Error = `[`Rejection`]`>`), it's error should be handled via
/// >              [`Filter::recover()`] to fails fast and avoid switching to other [`Filter`]s
/// >              branches, because [`Rejection` doesn't mean to abort the whole request, but
/// >              rather to say that a `Filter` couldn't fulfill its preconditions][1].
/// ```rust
/// # use std::sync::Arc;
/// #
/// # use juniper::{graphql_object, EmptyMutation, EmptySubscription, RootNode};
/// # use juniper_warp::make_graphql_filter;
/// # use warp::{http, Filter as _, Reply as _};
/// #
/// # type UserId = String;
/// # #[derive(Debug)]
/// # struct AppState(Vec<i64>);
/// # struct ExampleContext(Arc<AppState>, UserId);
/// # impl juniper::Context for ExampleContext {}
/// #
/// # struct QueryRoot;
/// #
/// # #[graphql_object(context = ExampleContext)]
/// # impl QueryRoot {
/// #     fn say_hello(context: &ExampleContext) -> String {
/// #         format!(
/// #             "good morning {}, the app state is {:?}",
/// #             context.1,
/// #             context.0,
/// #         )
/// #     }
/// # }
/// #
/// #[derive(Clone, Copy, Debug)]
/// struct NotAuthorized;
///
/// impl warp::reject::Reject for NotAuthorized {}
///
/// impl warp::Reply for NotAuthorized {
///     fn into_response(self) -> warp::reply::Response {
///         http::StatusCode::FORBIDDEN.into_response()
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
///     .and_then(|auth_header: String, app_state: Arc<AppState>| async move {
///         if auth_header == "correct" {
///             Ok(ExampleContext(app_state, auth_header))
///         } else {
///             Err(warp::reject::custom(NotAuthorized))
///         }
///     });
///
/// let graphql_endpoint = warp::path("graphql")
///     .and(make_graphql_filter(schema, context_extractor))
///     .recover(|rejection: warp::reject::Rejection| async move {
///         rejection
///             .find::<NotAuthorized>()
///             .map(|e| e.into_response())
///             .ok_or(rejection)
///     });
/// ```
///
/// [1]: https://github.com/seanmonstar/warp/issues/388#issuecomment-576453485
pub fn make_graphql_filter<S, Query, Mutation, Subscription, CtxT, CtxErr>(
    schema: impl Into<Arc<juniper::RootNode<Query, Mutation, Subscription, S>>>,
    context_extractor: impl Filter<Extract = (CtxT,), Error = CtxErr> + Send + Sync + 'static,
) -> impl Filter<Extract = (reply::Response,), Error = Rejection> + Clone + Send
where
    Query: juniper::GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = CtxT> + Send + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = CtxT> + Send + 'static,
    Subscription::TypeInfo: Send + Sync,
    CtxT: Send + Sync + 'static,
    CtxErr: Into<Rejection>,
    S: ScalarValue + Send + Sync + 'static,
{
    let schema = schema.into();
    // At the moment, `warp` doesn't allow us to make `context_extractor` filter polymorphic over
    // its `Error` type to support both `Error = Infallible` and `Error = Rejection` filters at the
    // same time. This is due to the `CombinedRejection` trait and the `FilterBase::map_err()`
    // combinator being sealed inside `warp` as private items. The only way to have input type
    // polymorphism for `Filter::Error` type is a `BoxedFilter`, which handles it internally.
    // See more in the following issues:
    // https://github.com/seanmonstar/warp/issues/299
    let context_extractor = context_extractor.boxed();

    get_query_extractor::<S>()
        .or(post_json_extractor::<S>())
        .unify()
        .or(post_graphql_extractor::<S>())
        .unify()
        .and(warp::any().map(move || schema.clone()))
        .and(context_extractor)
        .then(graphql_handler::<Query, Mutation, Subscription, CtxT, S>)
        .recover(handle_rejects)
        .unify()
}

/// Same as [`make_graphql_filter()`], but for [executing synchronously][1].
///
/// > __NOTE__: In order to avoid blocking, this handler will use [`tokio::task::spawn_blocking()`]
/// >           on the runtime [`warp`] is running on.
///
/// [1]: GraphQLBatchRequest::execute_sync
pub fn make_graphql_filter_sync<S, Query, Mutation, Subscription, CtxT, CtxErr>(
    schema: impl Into<Arc<juniper::RootNode<Query, Mutation, Subscription, S>>>,
    context_extractor: impl Filter<Extract = (CtxT,), Error = CtxErr> + Send + Sync + 'static,
) -> impl Filter<Extract = (reply::Response,), Error = Rejection> + Clone + Send
where
    Query: juniper::GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
    CtxT: Send + Sync + 'static,
    CtxErr: Into<Rejection>,
    S: ScalarValue + Send + Sync + 'static,
{
    let schema = schema.into();
    // At the moment, `warp` doesn't allow us to make `context_extractor` filter polymorphic over
    // its `Error` type to support both `Error = Infallible` and `Error = Rejection` filters at the
    // same time. This is due to the `CombinedRejection` trait and the `FilterBase::map_err()`
    // combinator being sealed inside `warp` as private items. The only way to have input type
    // polymorphism for `Filter::Error` type is a `BoxedFilter`, which handles it internally.
    // See more in the following issues:
    // https://github.com/seanmonstar/warp/issues/299
    let context_extractor = context_extractor.boxed();

    get_query_extractor::<S>()
        .or(post_json_extractor::<S>())
        .unify()
        .or(post_graphql_extractor::<S>())
        .unify()
        .and(warp::any().map(move || schema.clone()))
        .and(context_extractor)
        .then(graphql_handler_sync::<Query, Mutation, Subscription, CtxT, S>)
        .recover(handle_rejects)
        .unify()
}

/// Executes the provided [`GraphQLBatchRequest`] against the provided `schema` in the provided
/// `context`.
async fn graphql_handler<Query, Mutation, Subscription, CtxT, S>(
    req: GraphQLBatchRequest<S>,
    schema: Arc<juniper::RootNode<Query, Mutation, Subscription, S>>,
    context: CtxT,
) -> reply::Response
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
    let resp = req.execute(&*schema, &context).await;
    JuniperResponse(resp).into_response()
}

/// Same as [`graphql_handler()`], but for [executing synchronously][1].
///
/// [1]: GraphQLBatchRequest::execute_sync
async fn graphql_handler_sync<Query, Mutation, Subscription, CtxT, S>(
    req: GraphQLBatchRequest<S>,
    schema: Arc<juniper::RootNode<Query, Mutation, Subscription, S>>,
    context: CtxT,
) -> reply::Response
where
    Query: juniper::GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
    CtxT: Send + Sync + 'static,
    S: ScalarValue + Send + Sync + 'static,
{
    task::spawn_blocking(move || req.execute_sync(&*schema, &context))
        .await
        .map(|resp| JuniperResponse(resp).into_response())
        .unwrap_or_else(|e| BlockingError(e).into_response())
}

/// Extracts a [`GraphQLBatchRequest`] from a POST `application/json` HTTP request.
fn post_json_extractor<S>(
) -> impl Filter<Extract = (GraphQLBatchRequest<S>,), Error = Rejection> + Clone + Send
where
    S: ScalarValue + Send,
{
    warp::post().and(body::json())
}

/// Extracts a [`GraphQLBatchRequest`] from a POST `application/graphql` HTTP request.
fn post_graphql_extractor<S>(
) -> impl Filter<Extract = (GraphQLBatchRequest<S>,), Error = Rejection> + Clone + Send
where
    S: ScalarValue + Send,
{
    warp::post()
        .and(body::bytes())
        .and_then(|body: Bytes| async move {
            let query = str::from_utf8(body.as_ref())
                .map_err(|e| reject::custom(FilterError::NonUtf8Body(e)))?;
            let req = GraphQLRequest::new(query.into(), None, None);
            Ok::<GraphQLBatchRequest<S>, Rejection>(GraphQLBatchRequest::Single(req))
        })
}

/// Extracts a [`GraphQLBatchRequest`] from a GET HTTP request.
fn get_query_extractor<S>(
) -> impl Filter<Extract = (GraphQLBatchRequest<S>,), Error = Rejection> + Clone + Send
where
    S: ScalarValue + Send,
{
    warp::get()
        .and(query::query())
        .and_then(|mut qry: HashMap<String, String>| async move {
            let req = GraphQLRequest::new(
                qry.remove("query")
                    .ok_or_else(|| reject::custom(FilterError::MissingPathQuery))?,
                qry.remove("operation_name"),
                qry.remove("variables")
                    .map(|vs| serde_json::from_str(&vs))
                    .transpose()
                    .map_err(|e| reject::custom(FilterError::InvalidPathVariables(e)))?,
            );
            Ok::<GraphQLBatchRequest<S>, Rejection>(GraphQLBatchRequest::Single(req))
        })
}

/// Handles all the [`Rejection`]s happening in [`make_graphql_filter()`] to fail fast, if required.
async fn handle_rejects(rej: Rejection) -> Result<reply::Response, Rejection> {
    let (status, msg) = if let Some(e) = rej.find::<FilterError>() {
        (StatusCode::BAD_REQUEST, e.to_string())
    } else if let Some(e) = rej.find::<warp::reject::InvalidQuery>() {
        (StatusCode::BAD_REQUEST, e.to_string())
    } else if let Some(e) = rej.find::<BodyDeserializeError>() {
        (StatusCode::BAD_REQUEST, e.to_string())
    } else {
        return Err(rej);
    };

    Ok(http::Response::builder()
        .status(status)
        .body(msg.into())
        .unwrap())
}

/// Possible errors happening in [`Filter`]s during [`GraphQLBatchRequest`] extraction.
#[derive(Debug)]
enum FilterError {
    /// GET HTTP request misses query parameters.
    MissingPathQuery,

    /// GET HTTP request contains ivalid `path` query parameter.
    InvalidPathVariables(serde_json::Error),

    /// POST HTTP request contains non-UTF-8 body.
    NonUtf8Body(str::Utf8Error),
}
impl Reject for FilterError {}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPathQuery => {
                write!(f, "Missing GraphQL `query` string in query parameters")
            }
            Self::InvalidPathVariables(e) => write!(
                f,
                "Failed to deserialize GraphQL `variables` from JSON: {e}",
            ),
            Self::NonUtf8Body(e) => write!(f, "Request body is not a valid UTF-8 string: {e}"),
        }
    }
}

/// Error raised by [`tokio::task::spawn_blocking()`] if the thread pool has been shutdown.
#[derive(Debug)]
struct BlockingError(task::JoinError);

impl Reply for BlockingError {
    fn into_response(self) -> reply::Response {
        http::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!("Failed to execute synchronous GraphQL request: {}", self.0).into())
            .unwrap_or_else(|e| {
                unreachable!("cannot build `reply::Response` out of `BlockingError`: {e}")
            })
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

#[cfg(test)]
mod tests {
    mod make_graphql_filter {
        use std::future;

        use juniper::{
            http::GraphQLBatchRequest,
            tests::fixtures::starwars::schema::{Database, Query},
            EmptyMutation, EmptySubscription,
        };
        use warp::{
            http,
            reject::{self, Reject},
            test::request,
            Filter as _, Reply,
        };

        use super::super::make_graphql_filter;

        #[tokio::test]
        async fn post_json() {
            type Schema =
                juniper::RootNode<Query, EmptyMutation<Database>, EmptySubscription<Database>>;

            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            let db = warp::any().map(Database::new);
            let filter = warp::path("graphql2").and(make_graphql_filter(schema, db));

            let response = request()
                .method("POST")
                .path("/graphql2")
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(r#"{"variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }"}"#)
                .reply(&filter)
                .await;

            assert_eq!(response.status(), http::StatusCode::OK);
            assert_eq!(
                response.headers().get("content-type").unwrap(),
                "application/json",
            );
            assert_eq!(
                String::from_utf8(response.body().to_vec()).unwrap(),
                r#"{"data":{"hero":{"name":"R2-D2"}}}"#,
            );
        }

        #[tokio::test]
        async fn rejects_fast_when_context_extractor_fails() {
            use std::sync::{
                atomic::{AtomicBool, Ordering},
                Arc,
            };

            #[derive(Clone, Copy, Debug)]
            struct ExtractionError;

            impl Reject for ExtractionError {}

            impl warp::Reply for ExtractionError {
                fn into_response(self) -> warp::reply::Response {
                    http::StatusCode::IM_A_TEAPOT.into_response()
                }
            }

            type Schema =
                juniper::RootNode<Query, EmptyMutation<Database>, EmptySubscription<Database>>;

            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            // Should error on first extraction only, to check whether it rejects fast and doesn't
            // switch to other `.or()` filter branches. See #1177 for details:
            // https://github.com/graphql-rust/juniper/issues/1177
            let is_called = Arc::new(AtomicBool::new(false));
            let context_extractor = warp::any().and_then(move || {
                future::ready(if is_called.swap(true, Ordering::Relaxed) {
                    Ok(Database::new())
                } else {
                    Err(reject::custom(ExtractionError))
                })
            });

            let filter = warp::path("graphql")
                .and(make_graphql_filter(schema, context_extractor))
                .recover(|rejection: warp::reject::Rejection| async move {
                    rejection
                        .find::<ExtractionError>()
                        .map(|e| e.into_response())
                        .ok_or(rejection)
                });

            let resp = request()
                .method("POST")
                .path("/graphql")
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(r#"{"variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }"}"#)
                .reply(&filter)
                .await;

            assert_eq!(
                resp.status(),
                http::StatusCode::IM_A_TEAPOT,
                "response: {resp:#?}",
            );
        }

        #[tokio::test]
        async fn batch_requests() {
            type Schema =
                juniper::RootNode<Query, EmptyMutation<Database>, EmptySubscription<Database>>;

            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            let db = warp::any().map(Database::new);
            let filter = warp::path("graphql2").and(make_graphql_filter(schema, db));

            let response = request()
                .method("POST")
                .path("/graphql2")
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(
                    r#"[
                        {"variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }"},
                        {"variables": null, "query": "{ hero(episode: EMPIRE) { id name } }"}
                    ]"#,
                )
                .reply(&filter)
                .await;

            assert_eq!(response.status(), http::StatusCode::OK);
            assert_eq!(
                String::from_utf8(response.body().to_vec()).unwrap(),
                r#"[{"data":{"hero":{"name":"R2-D2"}}},{"data":{"hero":{"id":"1000","name":"Luke Skywalker"}}}]"#,
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

    mod graphiql_filter {
        use warp::{http, test::request, Filter as _};

        use super::super::{graphiql_filter, graphiql_response};

        #[test]
        fn response_does_not_panic() {
            graphiql_response("/abcd", None);
        }

        #[tokio::test]
        async fn endpoint_matches() {
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
        async fn returns_graphiql_source() {
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
        async fn endpoint_with_subscription_matches() {
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
    }

    mod playground_filter {
        use warp::{http, test::request, Filter as _};

        use super::super::playground_filter;

        #[tokio::test]
        async fn endpoint_matches() {
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
        async fn returns_playground_source() {
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
    }
}
