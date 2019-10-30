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
//#![deny(warnings)]
#![doc(html_root_url = "https://docs.rs/juniper_warp/0.2.0")]

use std::pin::Pin;
use std::sync::RwLock;

use futures03::channel::mpsc;
use futures::{future::poll_fn};
use futures03::Future;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use warp::{filters::BoxedFilter, Filter};

#[cfg(feature = "async")]
use futures03::{
    future::FutureExt, stream::StreamExt,
    sink::SinkExt,
};

use juniper::{DefaultScalarValue, InputValue, ScalarRefValue, ScalarValue};
use juniper::http::{GraphQLRequest, StreamGraphQLResponse};
use std::collections::HashMap;
use warp::ws::Message;
use std::borrow::BorrowMut;

#[derive(Debug, serde_derive::Deserialize, PartialEq)]
#[serde(untagged)]
#[serde(bound = "InputValue<S>: Deserialize<'de>")]
enum GraphQLBatchRequest<S = DefaultScalarValue>
where
    S: ScalarValue,
{
    Single(juniper::http::GraphQLRequest<S>),
    Batch(Vec<juniper::http::GraphQLRequest<S>>),
}

impl<S> GraphQLBatchRequest<S>
where
    S: ScalarValue + Send + Sync + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    pub fn execute<'a, CtxT, QueryT, MutationT, SubscriptionT>(
        &'a self,
        root_node: &'a juniper::RootNode<QueryT, MutationT, SubscriptionT, S>,
        context: &CtxT,
    ) -> GraphQLBatchResponse<'a, S>
    where
        QueryT: juniper::GraphQLType<S, Context = CtxT>,
        MutationT: juniper::GraphQLType<S, Context = CtxT>,
        SubscriptionT: juniper::SubscriptionHandler<S, Context = CtxT>,
        SubscriptionT::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
    {
        match self {
            &GraphQLBatchRequest::Single(ref request) => {
                GraphQLBatchResponse::Single(request.execute(root_node, context))
            }
            &GraphQLBatchRequest::Batch(ref requests) => GraphQLBatchResponse::Batch(

                requests
                    .iter()
                    .map(|request| request.execute(root_node, context))
                    .collect(),
            ),
        }
    }

    #[cfg(feature = "async")]
    pub async fn execute_async<'a, CtxT, QueryT, MutationT, SubscriptionT>(
        &'a self,
        root_node: &'a juniper::RootNode<'a, QueryT, MutationT, SubscriptionT, S>,
        context: &'a CtxT,
    ) -> GraphQLBatchResponse<'a, S>
    where
        QueryT: juniper::GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
        QueryT::TypeInfo: Send + Sync,
        MutationT: juniper::GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
        MutationT::TypeInfo: Send + Sync,
        SubscriptionT: juniper::SubscriptionHandlerAsync<S, Context = CtxT> + Send + Sync,
        SubscriptionT::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
        S: Send + Sync,
    {
        match self {
            &GraphQLBatchRequest::Single(ref request) => {
                let res = request.execute_async(root_node, context).await;
                GraphQLBatchResponse::Single(res)
            }
            &GraphQLBatchRequest::Batch(ref requests) => {
                let futures = requests
                    .iter()
                    .map(|request| request.execute_async(root_node, context))
                    .collect::<Vec<_>>();
                let responses = futures03::future::join_all(futures).await;

                GraphQLBatchResponse::Batch(responses)
            }
        }
    }
}

#[derive(serde_derive::Serialize)]
#[serde(untagged)]
enum GraphQLBatchResponse<'a, S = DefaultScalarValue>
where
    S: ScalarValue,
{
    Single(juniper::http::GraphQLResponse<'a, S>),
    Batch(Vec<juniper::http::GraphQLResponse<'a, S>>),
}

impl<'a, S> GraphQLBatchResponse<'a, S>
where
    S: ScalarValue,
{
    fn is_ok(&self) -> bool {
        match self {
            GraphQLBatchResponse::Single(res) => res.is_ok(),
            GraphQLBatchResponse::Batch(reses) => reses.iter().all(|res| res.is_ok()),
        }
    }
}

/// Make a filter for graphql endpoint.
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
/// # use juniper::{EmptyMutation, RootNode};
/// # use juniper_warp::make_graphql_filter;
/// #
/// # fn main() {
/// type UserId = String;
/// # #[derive(Debug)]
/// struct AppState(Vec<i64>);
/// struct ExampleContext(Arc<AppState>, UserId);
///
/// struct QueryRoot;
///
/// #[juniper::object(
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
/// let schema = RootNode::new(QueryRoot, EmptyMutation::new());
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
///     .and(warp::post2())
///     .and(graphql_filter);
/// # }
/// ```
pub fn make_graphql_filter<Query, Mutation, Subscription, Context, S>(
    schema: juniper::RootNode<'static, Query, Mutation, Subscription, S>,
    context_extractor: BoxedFilter<(Context,)>,
) -> BoxedFilter<(warp::http::Response<Vec<u8>>,)>
where
    S: ScalarValue + Send + Sync + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
    Context: Send + 'static,
    Query: juniper::GraphQLType<S, Context = Context, TypeInfo = ()> + Send + Sync + 'static,
    Mutation: juniper::GraphQLType<S, Context = Context, TypeInfo = ()> + Send + Sync + 'static,
    Subscription: juniper::SubscriptionHandler<S, Context = Context, TypeInfo = ()> + Send + Sync + 'static,
    Context: Send + Sync,
{
    use futures::future::Future;

    let schema = Arc::new(schema);
    let post_schema = schema.clone();

    let handle_post_request =
        move |context: Context, request: GraphQLBatchRequest<S>| -> Response {
            let schema = post_schema.clone();

            futures03::compat::Compat01As03::new(
                poll_fn(move || {
                    tokio_threadpool::blocking(|| {
                        let response = request.execute(&schema, &context);
                        Ok((serde_json::to_vec(&response)?, response.is_ok()))
                    })
                })
                .and_then(|result| ::futures::future::done(Ok(build_response(result))))
                .map_err(|e: tokio_threadpool::BlockingError| warp::reject::custom(e))
            )
                .boxed()

        };

    let post_filter = warp::post2()
        .and(context_extractor.clone())
        .and(warp::body::json())
        .and_then(handle_post_request);

    let handle_get_request =
        move |context: Context, mut request: std::collections::HashMap<String, String>|
          -> Response {
        let schema = schema.clone();
        futures03::compat::Compat01As03::new(
            poll_fn(move || {
                tokio_threadpool::blocking(|| {
                    let variables = match request.remove("variables") {
                        None => None,
                        Some(vs) => serde_json::from_str(&vs)?,
                    };

                    let graphql_request = juniper::http::GraphQLRequest::new(
                        request.remove("query").ok_or_else(|| {
                            failure::format_err!("Missing GraphQL query string in query parameters")
                        })?,
                        request.get("operation_name").map(|s| s.to_owned()),
                        variables,
                    );

                    let response = graphql_request.execute(&schema, &context);
                    Ok((serde_json::to_vec(&response)?, response.is_ok()))
                })
            })
            .and_then(|result| ::futures::future::done(Ok(build_response(result))))
            .map_err(|e: tokio_threadpool::BlockingError| warp::reject::custom(e)),
        )
            .boxed()
    };

    let get_filter = warp::get2()
        .and(context_extractor.clone())
        .and(warp::filters::query::query())
        .and_then(handle_get_request);

    get_filter.or(post_filter).unify().boxed()
}

/// FIXME: docs
#[cfg(feature = "async")]
pub fn make_graphql_filter_async<Query, Mutation, Subscription, Context, S>(
    schema: juniper::RootNode<'static, Query, Mutation, Subscription, S>,
    context_extractor: BoxedFilter<(Context,)>,
) -> BoxedFilter<(warp::http::Response<Vec<u8>>,)>
where
    S: ScalarValue + Send + Sync + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
    Context: Send + Sync + 'static,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::SubscriptionHandlerAsync<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    let schema = Arc::new(schema);
    let post_schema = schema.clone();

    let handle_post_request =
        move |context: Context, request: GraphQLBatchRequest<S>| {
            let schema = post_schema.clone();

            async move {
                let res =
                    request.execute_async(&schema, &context)
                        .await;

                match serde_json::to_vec(&res) {
                    Ok(json) => Ok(build_response(Ok((json, res.is_ok())))),
                    Err(e) => Err(warp::reject::custom(e)),
                }
            }
        };

    let post_filter = warp::post2()
        .and(context_extractor.clone())
        .and(warp::body::json())
        .and_then(handle_post_request);

    let handle_get_request =
        move |context: Context, mut request: std::collections::HashMap<String, String>| {
                let schema = schema.clone();

                async move {
                    let variables = match request.remove("variables") {
                        None => None,
                        Some(vs) => serde_json::from_str(&vs)?,
                    };

                    let graphql_request = juniper::http::GraphQLRequest::new(
                        request.remove("query").ok_or_else(|| {
                            failure::format_err!("Missing GraphQL query string in query parameters")
                        })?,
                        request.get("operation_name").map(|s| s.to_owned()),
                        variables,
                    );

                    let response = graphql_request.execute_async(&schema, &context).await;

                    Ok((serde_json::to_vec(&response)?, response.is_ok()))
                }
                    .map(|result| -> Result<_, warp::reject::Rejection> {
                        Ok(build_response(result))
                    })
        };

    let get_filter = warp::get2()
        .and(context_extractor.clone())
        .and(warp::filters::query::query())
        .and_then(handle_get_request);

    get_filter.or(post_filter).unify().boxed()
}


/// FIXME: docs
#[cfg(feature = "async")]
pub fn make_graphql_subscriptions_async<Query, Mutation, Subscription, Context, S>(
    websocket: warp::ws::WebSocket,
    schema: Arc<juniper::RootNode<'static, Query, Mutation, Subscription, S>>,
    context: Context
) -> impl Future<Output = ()> + Send
where
    S: ScalarValue + Send + Sync + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
    Context: Clone + Send + Sync + 'static,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::SubscriptionHandlerAsync<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    let (mut user_ws_tx, mut user_ws_rx) = websocket.split();

     let (tx, rx) = mpsc::unbounded();
     warp::spawn(
         rx
             .forward(user_ws_tx)
             .map(|result| {
                 if let Err(e) = result {
                     println!("websocket send error: {}", e);
                 }
             })
     );

    let schema = Arc::new(schema);
    let context = Arc::new(context);
//    let mut user_ws_tx = Arc::new(user_ws_tx);

    user_ws_rx
        .then(move |msg| {
            let schema = schema.clone();
            let context = context.clone();
            let user_tx = tx.clone();

            async move {
                let msg = msg.unwrap();
                println!("got message: {:?}", msg);
                let msg = msg.to_str().unwrap();
                println!("   message text: {}", msg);

                let schema = schema.clone();
                let context = context.clone();
                let request: WS = serde_json::from_str(&msg).unwrap();

                match request.type_name.as_str() {
                    "connection_init" => { },
                    "start" => {
                        println!("message payload: {:#?}", request.payload);
                        let payload = request.payload.unwrap();

                        // execute subscription
                        let graphql_request = GraphQLRequest::<S>::new(
                            payload.query.unwrap(),
                            None,
                            //todo: variables
                            None,
//                            payload.variables.map(|s| juniper::InputValue::Object(s)),
                        );

                        let mut executor = juniper::SubscriptionsExecutor::<'_, Context, S>::new();
                        let response_stream = graphql_request.subscribe_async(
                            &schema,
                            &context,
                            &mut executor
                        )
                            .await;

                        let stream = response_stream
                            .into_stream()
                            .expect("Response stream is none");

                        stream.for_each(move |response| {
                            let mut response_text = serde_json::to_string(&response).unwrap();
                            println!("Got response {:?}", response_text);
                            response_text = format!(r#"
                            {{"type":"data","id":"1","payload":{} }}
                            "#, response_text);
                            println!("    will send response {:?}", response_text);


                            println!("unbounded_send: {:?}", user_tx.unbounded_send(Ok(Message::text(response_text.clone()))));

                            async move {
//                                    let mut user_tx = Arc::get_mut(&mut user_ws_tx).expect("unable to get mut reference");
//                                    user_tx.send(Message::text(response_text.clone())).await;
                            }
                        }).await;

                    },
                    "stop" => { },
                    _ => panic!("unknown type")
                }

            }
        })
        .for_each(|x| { async {} })
////        .for_each(move |response_stream: Option<_>| {
////            if let Some(r_stream) = response_stream {
////                async move {
////                    stream
////                        .for_each(move |response| {
////                            let response_text = serde_json::to_string(&response).unwrap();
////                            println!("Got response {:?}", response_text);
////                            async {}
////                        }).await;
////                }
////            }
////            else {
////                async {}
////            }
////        })
////                .map(move |x| {
////                    user_ws_tx.send(Message::text(x.clone()));
////                })
////        })
////        .map(|response_text| {
//
////            let user_tx = user_ws_tx.clone();
//
////                        match
////            user_ws_tx.send(Message::text(response_text.clone()));
////                            {
////                            Ok(()) => (),
////                            Err(_disconnected) => {
////                                ()
////                                // The tx is disconnected, our `user_disconnected` code
////                                // should be happening in another task, nothing more to
////                                // do here.
////                            }
////        })
        .then(move |result| {
            println!("Got result: {:?}", result);
            async {}
        })

}

#[derive(Deserialize)]
struct WS {
    id: Option<String>,
    #[serde(rename(deserialize = "type"))]
    type_name: String,
        payload: Option<GraphQLPayload>,
//    payload: Option<GraphQLRequest>,
}

#[derive(Debug)]
#[derive(Deserialize)]
struct GraphQLPayload {
    variables: Option<HashMap<String, String>>,
    extensions: Option<HashMap<String, String>>,
    operatonName: Option<String>,
    query: Option<String>,
}

#[derive(Serialize)]
struct Output {
    data: String,
    variables: String,
}

fn build_response(
    response: Result<(Vec<u8>, bool), failure::Error>,
) -> warp::http::Response<Vec<u8>> {
    match response {
        Ok((body, is_ok)) => warp::http::Response::builder()
            .status(if is_ok { 200 } else { 400 })
            .header("content-type", "application/json")
            .body(body)
            .expect("response is valid"),
        Err(_) => warp::http::Response::builder()
            .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(Vec::new())
            .expect("status code is valid"),
    }
}

type Response =
    Pin<Box<dyn Future<Output = Result<warp::http::Response<Vec<u8>>, warp::reject::Rejection>> + Send>>;

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
/// # fn main() {
/// let graphiql_route = warp::path("graphiql").and(graphiql_filter("/graphql"));
/// # }
/// ```
pub fn graphiql_filter(
    graphql_endpoint_url: &'static str,
) -> warp::filters::BoxedFilter<(warp::http::Response<Vec<u8>>,)> {
    warp::any()
        .map(move || graphiql_response(graphql_endpoint_url))
        .boxed()
}

fn graphiql_response(graphql_endpoint_url: &'static str) -> warp::http::Response<Vec<u8>> {
    warp::http::Response::builder()
        .header("content-type", "text/html;charset=utf-8")
        .body(juniper::http::graphiql::graphiql_source(graphql_endpoint_url).into_bytes())
        .expect("response is valid")
}

/// Create a filter that replies with an HTML page containing GraphQL Playground. This does not handle routing, so you can mount it on any endpoint.
pub fn playground_filter(
    graphql_endpoint_url: &'static str,
    subscriptions_endpoint_url: &'static str,
) -> warp::filters::BoxedFilter<(warp::http::Response<Vec<u8>>,)> {
    warp::any()
        .map(move || playground_response(graphql_endpoint_url, subscriptions_endpoint_url))
        .boxed()
}

fn playground_response(
    graphql_endpoint_url: &'static str,
    subscriptions_endpoint_url: &'static str
) -> warp::http::Response<Vec<u8>> {
    warp::http::Response::builder()
        .header("content-type", "text/html;charset=utf-8")
        .body(juniper::http::playground::playground_source(
            graphql_endpoint_url,
            subscriptions_endpoint_url
        ).into_bytes())
        .expect("response is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::{http, test::request};

    #[test]
    fn graphiql_response_does_not_panic() {
        graphiql_response("/abcd");
    }

    #[test]
    fn graphiql_endpoint_matches() {
        let filter = warp::get2()
            .and(warp::path("graphiql"))
            .and(graphiql_filter("/graphql"));
        let result = request()
            .method("GET")
            .path("/graphiql")
            .header("accept", "text/html")
            .filter(&filter);

        assert!(result.is_ok());
    }

    #[test]
    fn graphiql_endpoint_returns_graphiql_source() {
        let filter = warp::get2()
            .and(warp::path("dogs-api"))
            .and(warp::path("graphiql"))
            .and(graphiql_filter("/dogs-api/graphql"));
        let response = request()
            .method("GET")
            .path("/dogs-api/graphiql")
            .header("accept", "text/html")
            .reply(&filter);

        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/html;charset=utf-8"
        );
        let body = String::from_utf8(response.body().to_vec()).unwrap();

        assert!(body.contains("<script>var GRAPHQL_URL = '/dogs-api/graphql';</script>"));
    }

    #[test]
    fn playground_endpoint_matches() {
        let filter = warp::get2()
            .and(warp::path("playground"))
            .and(playground_filter("/graphql"));
        let result = request()
            .method("GET")
            .path("/playground")
            .header("accept", "text/html")
            .filter(&filter);

        assert!(result.is_ok());
    }

    #[test]
    fn playground_endpoint_returns_playground_source() {
        let filter = warp::get2()
            .and(warp::path("dogs-api"))
            .and(warp::path("playground"))
            .and(playground_filter("/dogs-api/graphql"));
        let response = request()
            .method("GET")
            .path("/dogs-api/playground")
            .header("accept", "text/html")
            .reply(&filter);

        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/html;charset=utf-8"
        );
        let body = String::from_utf8(response.body().to_vec()).unwrap();

        assert!(body.contains("GraphQLPlayground.init(root, { endpoint: '/dogs-api/graphql' })"));
    }

    #[test]
    fn graphql_handler_works_json_post() {
        use juniper::{
            tests::{model::Database, schema::Query},
            EmptyMutation, RootNode,
        };

        type Schema = juniper::RootNode<'static, Query, EmptyMutation<Database>>;

        let schema: Schema = RootNode::new(Query, EmptyMutation::<Database>::new());

        let state = warp::any().map(move || Database::new());
        let filter = warp::path("graphql2").and(make_graphql_filter(schema, state.boxed()));

        let response = request()
            .method("POST")
            .path("/graphql2")
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .body(r##"{ "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" }"##)
            .reply(&filter);

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

    #[test]
    fn batch_requests_work() {
        use juniper::{
            tests::{model::Database, schema::Query},
            EmptyMutation, RootNode,
        };

        type Schema = juniper::RootNode<'static, Query, EmptyMutation<Database>>;

        let schema: Schema = RootNode::new(Query, EmptyMutation::<Database>::new());

        let state = warp::any().map(move || Database::new());
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
            .reply(&filter);

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
        http::tests::{run_http_test_suite, HTTPIntegration, TestResponse},
        tests::{model::Database, schema::Query},
        EmptyMutation, RootNode,
    };
    use warp::{self, Filter};

    type Schema = juniper::RootNode<'static, Query, EmptyMutation<Database>>;

    fn warp_server() -> warp::filters::BoxedFilter<(warp::http::Response<Vec<u8>>,)> {
        let schema: Schema = RootNode::new(Query, EmptyMutation::<Database>::new());

        let state = warp::any().map(move || Database::new());
        let filter = warp::filters::path::end().and(make_graphql_filter(schema, state.boxed()));

        filter.boxed()
    }

    struct TestWarpIntegration {
        filter: warp::filters::BoxedFilter<(warp::http::Response<Vec<u8>>,)>,
    }

    // This can't be implemented with the From trait since TestResponse is not defined in this crate.
    fn test_response_from_http_response(response: warp::http::Response<Vec<u8>>) -> TestResponse {
        TestResponse {
            status_code: response.status().as_u16() as i32,
            body: Some(String::from_utf8(response.body().to_owned()).unwrap()),
            content_type: response
                .headers()
                .get("content-type")
                .expect("missing content-type header in warp response")
                .to_str()
                .expect("invalid content-type string")
                .to_owned(),
        }
    }

    impl HTTPIntegration for TestWarpIntegration {
        fn get(&self, url: &str) -> TestResponse {
            use percent_encoding::{percent_encode, DEFAULT_ENCODE_SET};
            let url: String = percent_encode(url.replace("/?", "").as_bytes(), DEFAULT_ENCODE_SET)
                .into_iter()
                .collect::<Vec<_>>()
                .join("");

            let response = warp::test::request()
                .method("GET")
                .path(&format!("/?{}", url))
                .filter(&self.filter)
                .unwrap_or_else(|rejection| {
                    warp::http::Response::builder()
                        .status(rejection.status())
                        .header("content-type", "application/json")
                        .body(Vec::new())
                        .unwrap()
                });
            test_response_from_http_response(response)
        }

        fn post(&self, url: &str, body: &str) -> TestResponse {
            let response = warp::test::request()
                .method("POST")
                .header("content-type", "application/json")
                .path(url)
                .body(body)
                .filter(&self.filter)
                .unwrap_or_else(|rejection| {
                    warp::http::Response::builder()
                        .status(rejection.status())
                        .header("content-type", "application/json")
                        .body(Vec::new())
                        .unwrap()
                });
            test_response_from_http_response(response)
        }
    }

    #[test]
    fn test_warp_integration() {
        let integration = TestWarpIntegration {
            filter: warp_server(),
        };

        run_http_test_suite(&integration);
    }
}
