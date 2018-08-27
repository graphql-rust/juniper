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
[example]: https://github.com/graphql-rust/juniper_warp/blob/master/examples/warp_server

*/

#![deny(missing_docs)]
#![deny(warnings)]

#[macro_use]
extern crate failure;
extern crate futures;
extern crate futures_cpupool;
extern crate juniper;
extern crate serde_json;
extern crate warp;

#[cfg(test)]
extern crate percent_encoding;

use futures::Future;
use futures_cpupool::CpuPool;
use std::sync::Arc;
use warp::{filters::BoxedFilter, Filter};

/// Make a filter for graphql endpoint.
///
/// The `schema` argument is your juniper schema.
///
/// The `context_extractor` argument should be a filter that provides the GraphQL context required by the schema.
///
/// In order to avoid blocking, this helper will create a [CpuPool](../futures_cpupool/struct.CpuPool.html) to resolve GraphQL requests.
///
/// If you want to pass your own threadpool, use [make_graphql_filter_with_thread_pool](fn.make_graphql_filter_with_thread_pool.html) instead.
///
/// Example:
///
/// ```
/// # extern crate juniper_warp;
/// # #[macro_use]
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
/// graphql_object! (QueryRoot: ExampleContext |&self| {
///     field say_hello(&executor) -> String {
///         let context = executor.context();
///
///         format!("good morning {}, the app state is {:?}", context.1, context.0)
///     }
/// });
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
pub fn make_graphql_filter<Query, Mutation, Context>(
    schema: juniper::RootNode<'static, Query, Mutation>,
    context_extractor: BoxedFilter<(Context,)>,
) -> BoxedFilter<(warp::http::Response<Vec<u8>>,)>
where
    Context: Send + 'static,
    Query: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
    Mutation: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
{
    let pool = CpuPool::new_num_cpus();
    make_graphql_filter_with_thread_pool(schema, context_extractor, pool)
}

type Response =
    Box<Future<Item = warp::http::Response<Vec<u8>>, Error = warp::reject::Rejection> + Send>;

/// Same as [make_graphql_filter](./fn.make_graphql_filter.html), but use the provided [CpuPool](../futures_cpupool/struct.CpuPool.html) instead.
pub fn make_graphql_filter_with_thread_pool<Query, Mutation, Context>(
    schema: juniper::RootNode<'static, Query, Mutation>,
    context_extractor: BoxedFilter<(Context,)>,
    thread_pool: futures_cpupool::CpuPool,
) -> BoxedFilter<(warp::http::Response<Vec<u8>>,)>
where
    Context: Send + 'static,
    Query: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
    Mutation: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
{
    let schema = Arc::new(schema);
    let post_schema = schema.clone();
    let pool_filter = warp::any().map(move || thread_pool.clone());

    let handle_post_request = move |context: Context,
                                    request: juniper::http::GraphQLRequest,
                                    pool: CpuPool|
          -> Response {
        let schema = post_schema.clone();
        Box::new(
            pool.spawn_fn(move || Ok(serde_json::to_vec(&request.execute(&schema, &context))?))
                .then(|result| ::futures::future::done(Ok(build_response(result))))
                .map_err(|_: failure::Error| warp::reject::server_error()),
        )
    };

    let post_filter = warp::post2()
        .and(context_extractor.clone())
        .and(warp::body::json())
        .and(pool_filter.clone())
        .and_then(handle_post_request);

    let handle_get_request = move |context: Context,
                                   mut request: std::collections::HashMap<String, String>,
                                   pool: CpuPool|
          -> Response {
        let schema = schema.clone();
        Box::new(
            pool.spawn_fn(move || {
                let graphql_request = juniper::http::GraphQLRequest::new(
                    request.remove("query").ok_or_else(|| {
                        format_err!("Missing GraphQL query string in query parameters")
                    })?,
                    request.get("operation_name").map(|s| s.to_owned()),
                    request
                        .remove("variables")
                        .and_then(|vs| serde_json::from_str(&vs).ok()),
                );
                Ok(serde_json::to_vec(
                    &graphql_request.execute(&schema, &context),
                )?)
            }).then(|result| ::futures::future::done(Ok(build_response(result))))
                .map_err(|_: failure::Error| warp::reject::server_error()),
        )
    };

    let get_filter = warp::get2()
        .and(context_extractor.clone())
        .and(warp::filters::query::query())
        .and(pool_filter)
        .and_then(handle_get_request);

    get_filter.or(post_filter).unify().boxed()
}

fn build_response(response: Result<Vec<u8>, failure::Error>) -> warp::http::Response<Vec<u8>> {
    match response {
        Ok(body) => warp::http::Response::builder()
            .header("content-type", "application/json")
            .body(body)
            .expect("response is valid"),
        Err(_) => warp::http::Response::builder()
            .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
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
/// # use juniper_warp::graphiql_handler;
/// #
/// # fn main() {
/// let graphiql_route = warp::path("graphiql").and(graphiql_handler("/graphql"));
/// # }
/// ```
pub fn graphiql_handler(
    graphql_endpoint_url: &'static str,
) -> warp::filters::BoxedFilter<(warp::http::Response<String>,)> {
    warp::any()
        .map(move || graphiql_response(graphql_endpoint_url))
        .boxed()
}

fn graphiql_response(graphql_endpoint_url: &'static str) -> warp::http::Response<String> {
    warp::http::Response::builder()
        .header("content-type", "text/html;charset=utf-8")
        .body(juniper::graphiql::graphiql_source(graphql_endpoint_url))
        .expect("response is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http;
    use warp::test::request;

    #[test]
    fn graphiql_response_does_not_panic() {
        graphiql_response("/abcd");
    }

    #[test]
    fn graphiql_endpoint_matches() {
        let filter = warp::get2()
            .and(warp::path("graphiql"))
            .and(graphiql_handler("/graphql"));
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
            .and(graphiql_handler("/dogs-api/graphql"));
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
    fn graphql_handler_works_json_post() {
        use juniper::tests::model::Database;
        use juniper::{EmptyMutation, RootNode};

        type Schema = juniper::RootNode<'static, Database, EmptyMutation<Database>>;

        let schema: Schema = RootNode::new(Database::new(), EmptyMutation::<Database>::new());

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
}

#[cfg(test)]
mod tests_http_harness {
    use super::*;
    use juniper::http::tests::{run_http_test_suite, HTTPIntegration, TestResponse};
    use juniper::tests::model::Database;
    use juniper::EmptyMutation;
    use juniper::RootNode;
    use warp;
    use warp::Filter;

    type Schema = juniper::RootNode<'static, Database, EmptyMutation<Database>>;

    fn warp_server() -> warp::filters::BoxedFilter<(warp::http::Response<Vec<u8>>,)> {
        let schema: Schema = RootNode::new(Database::new(), EmptyMutation::<Database>::new());

        let state = warp::any().map(move || Database::new());
        let filter = warp::filters::path::index().and(make_graphql_filter(schema, state.boxed()));

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
                .expect("warp filter failed");
            test_response_from_http_response(response)
        }

        fn post(&self, url: &str, body: &str) -> TestResponse {
            let response = warp::test::request()
                .method("POST")
                .header("content-type", "application/json")
                .path(url)
                .body(body)
                .filter(&self.filter)
                .expect("warp filter failed");
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
