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

extern crate juniper;
extern crate serde_json;
extern crate warp;

use std::sync::Arc;
use warp::{filters::BoxedFilter, Filter};

/// Make a filter for graphql endpoint.
///
/// The `schema` argument is your juniper schema.
///
/// The `context_extractor` argument should be a filter that provides the GraphQL context required by the schema.
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
///     .and(warp::post(graphql_filter));
/// # }
/// ```
pub fn make_graphql_filter<Query, Mutation, Context>(
    schema: juniper::RootNode<'static, Query, Mutation>,
    context_extractor: BoxedFilter<(Context,)>,
) -> BoxedFilter<(impl warp::Reply,)>
where
    Context: Send + Sync + 'static,
    // CtxRef: AsRef<Context> + Send + 'static,
    Query: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
    Mutation: juniper::GraphQLType<Context = Context, TypeInfo = ()> + Send + Sync + 'static,
{
    let schema = Arc::new(schema);

    let handle_request = move |context: Context,
                               request: juniper::http::GraphQLRequest|
          -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&request.execute(&schema, &context))
    };

    warp::post(
        warp::any()
            .and(context_extractor)
            .and(warp::body::json())
            .map(handle_request)
            .map(build_response),
    ).boxed()
}

fn build_response(response: Result<Vec<u8>, serde_json::Error>) -> warp::http::Response<Vec<u8>> {
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

    #[test]
    fn graphiql_response_does_not_panic() {
        graphiql_response("/abcd");
    }
}
