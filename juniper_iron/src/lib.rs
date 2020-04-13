/*!

# juniper_iron

This repository contains the [Iron][Iron] web framework integration for
[Juniper][Juniper], a [GraphQL][GraphQL] implementation for Rust.

For documentation, including guides and examples, check out [Juniper][Juniper].

A basic usage example can also be found in the [Api documentation][documentation].

## Links

* [Juniper][Juniper]
* [Api Reference][documentation]
* [Iron framework][Iron]

## Integrating with Iron


For example, continuing from the schema created above and using Iron to expose
the schema on an HTTP endpoint supporting both GET and POST requests:

```rust,no_run
extern crate iron;
# extern crate juniper;
# extern crate juniper_iron;
# use std::collections::HashMap;

use iron::prelude::*;
use juniper_iron::GraphQLHandler;
use juniper::{Context, EmptyMutation, EmptySubscription};

# use juniper::FieldResult;
#
# struct User { id: String, name: String, friend_ids: Vec<String>  }
# struct QueryRoot;
# struct Database { users: HashMap<String, User> }
#
# #[juniper::graphql_object( Context = Database )]
# impl User {
#     fn id(&self) -> FieldResult<&String> {
#         Ok(&self.id)
#     }
#
#     fn name(&self) -> FieldResult<&String> {
#         Ok(&self.name)
#     }
#
#     fn friends(context: &Database) -> FieldResult<Vec<&User>> {
#         Ok(self.friend_ids.iter()
#             .filter_map(|id| executor.context().users.get(id))
#             .collect())
#     }
# }
#
# #[juniper::graphql_object( Context = Database )]
# impl QueryRoot {
#     fn user(context: &Database, id: String) -> FieldResult<Option<&User>> {
#         Ok(executor.context().users.get(&id))
#     }
# }

// This function is executed for every request. Here, we would realistically
// provide a database connection or similar. For this example, we'll be
// creating the database from scratch.
fn context_factory(_: &mut Request) -> IronResult<Database> {
    Ok(Database {
        users: vec![
            ( "1000".to_owned(), User {
                id: "1000".to_owned(), name: "Robin".to_owned(),
                friend_ids: vec!["1001".to_owned()] } ),
            ( "1001".to_owned(), User {
                id: "1001".to_owned(), name: "Max".to_owned(),
                friend_ids: vec!["1000".to_owned()] } ),
        ].into_iter().collect()
    })
}

impl Context for Database {}

fn main() {
    // GraphQLHandler takes a context factory function, the root object,
    // and the mutation object. If we don't have any mutations to expose, we
    // can use the empty tuple () to indicate absence.
    let graphql_endpoint = GraphQLHandler::new(
        context_factory,
        QueryRoot,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    // Start serving the schema at the root on port 8080.
    Iron::new(graphql_endpoint).http("localhost:8080").unwrap();
}

```

See the the [`GraphQLHandler`][3] documentation for more information on what request methods are
supported.

[3]: ./struct.GraphQLHandler.html
[Iron]: https://github.com/iron/iron
[Juniper]: https://github.com/graphql-rust/juniper
[GraphQL]: http://graphql.org
[documentation]: https://docs.rs/juniper_iron

*/

#![doc(html_root_url = "https://docs.rs/juniper_iron/0.3.0")]

#[cfg(test)]
extern crate iron_test;
#[cfg(test)]
extern crate url;

use iron::{itry, method, middleware::Handler, mime::Mime, prelude::*, status};
use urlencoded::{UrlDecodingError, UrlEncodedQuery};

use std::{error::Error, fmt, io::Read};

use serde_json::error::Error as SerdeError;

use juniper::{
    http, http::GraphQLBatchRequest, DefaultScalarValue, GraphQLType, InputValue, RootNode,
    ScalarValue,
};

/// Handler that executes `GraphQL` queries in the given schema
///
/// The handler responds to GET requests and POST requests only. In GET
/// requests, the query should be supplied in the `query` URL parameter, e.g.
/// `http://localhost:3000/graphql?query={hero{name}}`.
///
/// POST requests support both queries and variables. POST a JSON document to
/// this endpoint containing the field `"query"` and optionally `"variables"`.
/// The variables should be a JSON object containing the variable to value
/// mapping.
pub struct GraphQLHandler<
    'a,
    CtxFactory,
    Query,
    Mutation,
    Subscription,
    CtxT,
    S = DefaultScalarValue,
> where
    S: ScalarValue,
    CtxFactory: Fn(&mut Request) -> IronResult<CtxT> + Send + Sync + 'static,
    CtxT: 'static,
    Query: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    Mutation: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
    Subscription: GraphQLType<S, Context = CtxT> + Send + Sync + 'static,
{
    context_factory: CtxFactory,
    root_node: RootNode<'a, Query, Mutation, Subscription, S>,
}

/// Handler that renders `GraphiQL` - a graphical query editor interface
pub struct GraphiQLHandler {
    graphql_url: String,
    subscription_url: Option<String>,
}

/// Handler that renders `GraphQL Playground` - a graphical query editor interface
pub struct PlaygroundHandler {
    graphql_url: String,
    subscription_url: Option<String>,
}

fn get_single_value<T>(mut values: Vec<T>) -> IronResult<T> {
    if values.len() == 1 {
        Ok(values.remove(0))
    } else {
        Err(GraphQLIronError::InvalidData("Duplicate URL query parameter").into())
    }
}

fn parse_url_param(params: Option<Vec<String>>) -> IronResult<Option<String>> {
    if let Some(values) = params {
        get_single_value(values).map(Some)
    } else {
        Ok(None)
    }
}

fn parse_variable_param<S>(params: Option<Vec<String>>) -> IronResult<Option<InputValue<S>>>
where
    S: ScalarValue,
{
    if let Some(values) = params {
        Ok(
            serde_json::from_str::<InputValue<S>>(get_single_value(values)?.as_ref())
                .map(Some)
                .map_err(GraphQLIronError::Serde)?,
        )
    } else {
        Ok(None)
    }
}

impl<'a, CtxFactory, Query, Mutation, Subscription, CtxT, S>
    GraphQLHandler<'a, CtxFactory, Query, Mutation, Subscription, CtxT, S>
where
    S: ScalarValue + Send + Sync + 'static,
    CtxFactory: Fn(&mut Request) -> IronResult<CtxT> + Send + Sync + 'static,
    CtxT: Send + Sync + 'static,
    Query: GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    Mutation: GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    Subscription: GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
{
    /// Build a new GraphQL handler
    ///
    /// The context factory will receive the Iron request object and is
    /// expected to construct a context object for the given schema. This can
    /// be used to construct e.g. database connections or similar data that
    /// the schema needs to execute the query.
    pub fn new(
        context_factory: CtxFactory,
        query: Query,
        mutation: Mutation,
        subscription: Subscription,
    ) -> Self {
        GraphQLHandler {
            context_factory,
            root_node: RootNode::new(query, mutation, subscription),
        }
    }

    fn handle_get(&self, req: &mut Request) -> IronResult<GraphQLBatchRequest<S>> {
        let url_query_string = req
            .get_mut::<UrlEncodedQuery>()
            .map_err(GraphQLIronError::Url)?;

        let input_query = parse_url_param(url_query_string.remove("query"))?
            .ok_or_else(|| GraphQLIronError::InvalidData("No query provided"))?;
        let operation_name = parse_url_param(url_query_string.remove("operationName"))?;
        let variables = parse_variable_param(url_query_string.remove("variables"))?;

        Ok(GraphQLBatchRequest::Single(http::GraphQLRequest::new(
            input_query,
            operation_name,
            variables,
        )))
    }

    fn handle_post(&self, req: &mut Request) -> IronResult<GraphQLBatchRequest<S>> {
        let mut request_payload = String::new();
        itry!(req.body.read_to_string(&mut request_payload));

        Ok(
            serde_json::from_str::<GraphQLBatchRequest<S>>(request_payload.as_str())
                .map_err(GraphQLIronError::Serde)?,
        )
    }

    fn execute_sync(
        &self,
        context: &CtxT,
        request: GraphQLBatchRequest<S>,
    ) -> IronResult<Response> {
        let response = request.execute_sync(&self.root_node, context);
        let content_type = "application/json".parse::<Mime>().unwrap();
        let json = serde_json::to_string_pretty(&response).unwrap();
        let status = if response.is_ok() {
            status::Ok
        } else {
            status::BadRequest
        };
        Ok(Response::with((content_type, status, json)))
    }
}

impl GraphiQLHandler {
    /// Build a new GraphiQL handler targeting the specified URL.
    ///
    /// The provided URL should point to the URL of the attached `GraphQLHandler`. It can be
    /// relative, so a common value could be `"/graphql"`.
    pub fn new(graphql_url: &str, subscription_url: Option<&str>) -> GraphiQLHandler {
        GraphiQLHandler {
            graphql_url: graphql_url.to_owned(),
            subscription_url: subscription_url.map(|s| s.to_owned()),
        }
    }
}

impl PlaygroundHandler {
    /// Build a new GraphQL Playground handler targeting the specified URL.
    ///
    /// The provided URL should point to the URL of the attached `GraphQLHandler`. It can be
    /// relative, so a common value could be `"/graphql"`.
    pub fn new(graphql_url: &str, subscription_url: Option<&str>) -> PlaygroundHandler {
        PlaygroundHandler {
            graphql_url: graphql_url.to_owned(),
            subscription_url: subscription_url.map(|s| s.to_owned()),
        }
    }
}

impl<'a, CtxFactory, Query, Mutation, Subscription, CtxT, S> Handler
    for GraphQLHandler<'a, CtxFactory, Query, Mutation, Subscription, CtxT, S>
where
    S: ScalarValue + Sync + Send + 'static,
    CtxFactory: Fn(&mut Request) -> IronResult<CtxT> + Send + Sync + 'static,
    CtxT: Send + Sync + 'static,
    Query: GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    Mutation: GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    Subscription: GraphQLType<S, Context = CtxT, TypeInfo = ()> + Send + Sync + 'static,
    'a: 'static,
{
    fn handle(&self, mut req: &mut Request) -> IronResult<Response> {
        let context = (self.context_factory)(req)?;

        let graphql_request = match req.method {
            method::Get => self.handle_get(&mut req)?,
            method::Post => self.handle_post(&mut req)?,
            _ => return Ok(Response::with(status::MethodNotAllowed)),
        };

        self.execute_sync(&context, graphql_request)
    }
}

impl Handler for GraphiQLHandler {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let content_type = "text/html; charset=utf-8".parse::<Mime>().unwrap();

        Ok(Response::with((
            content_type,
            status::Ok,
            juniper::http::graphiql::graphiql_source(
                &self.graphql_url,
                self.subscription_url.as_deref(),
            ),
        )))
    }
}

impl Handler for PlaygroundHandler {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let content_type = "text/html; charset=utf-8".parse::<Mime>().unwrap();

        Ok(Response::with((
            content_type,
            status::Ok,
            juniper::http::playground::playground_source(
                &self.graphql_url,
                self.subscription_url.as_deref(),
            ),
        )))
    }
}

#[derive(Debug)]
enum GraphQLIronError {
    Serde(SerdeError),
    Url(UrlDecodingError),
    InvalidData(&'static str),
}

impl fmt::Display for GraphQLIronError {
    fn fmt(&self, mut f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GraphQLIronError::Serde(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLIronError::Url(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLIronError::InvalidData(err) => fmt::Display::fmt(err, &mut f),
        }
    }
}

impl Error for GraphQLIronError {
    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            GraphQLIronError::Serde(ref err) => Some(err),
            GraphQLIronError::Url(ref err) => Some(err),
            GraphQLIronError::InvalidData(_) => None,
        }
    }
}

impl From<GraphQLIronError> for IronError {
    fn from(err: GraphQLIronError) -> IronError {
        let message = format!("{}", err);
        IronError::new(err, (status::BadRequest, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron::{Handler, Headers, Url};
    use iron_test::{request, response};
    use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

    use juniper::{
        http::tests as http_tests,
        tests::{model::Database, schema::Query},
        EmptyMutation, EmptySubscription,
    };

    use super::GraphQLHandler;

    /// https://url.spec.whatwg.org/#query-state
    const QUERY_ENCODE_SET: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'#').add(b'<').add(b'>');

    // This is ugly but it works. `iron_test` just dumps the path/url in headers
    // and newer `hyper` doesn't allow unescaped "{" or "}".
    fn fixup_url(url: &str) -> String {
        let url = Url::parse(&format!("http://localhost:3000{}", url)).expect("url to parse");
        let path: String = url
            .path()
            .iter()
            .map(|x| (*x).to_string())
            .collect::<Vec<String>>()
            .join("/");
        format!(
            "http://localhost:3000{}?{}",
            path,
            utf8_percent_encode(url.query().unwrap_or(""), QUERY_ENCODE_SET)
        )
    }

    struct TestIronIntegration;

    impl http_tests::HTTPIntegration for TestIronIntegration {
        fn get(&self, url: &str) -> http_tests::TestResponse {
            let result = request::get(&fixup_url(url), Headers::new(), &make_handler());
            match result {
                Ok(response) => make_test_response(response),
                Err(e) => make_test_error_response(e),
            }
        }

        fn post(&self, url: &str, body: &str) -> http_tests::TestResponse {
            let result = request::post(&fixup_url(url), Headers::new(), body, &make_handler());
            match result {
                Ok(response) => make_test_response(response),
                Err(e) => make_test_error_response(e),
            }
        }
    }

    #[test]
    fn test_iron_integration() {
        let integration = TestIronIntegration;

        http_tests::run_http_test_suite(&integration);
    }

    fn context_factory(_: &mut Request) -> IronResult<Database> {
        Ok(Database::new())
    }

    fn make_test_error_response(_: IronError) -> http_tests::TestResponse {
        // For now all errors return the same status code.
        // `juniper_iron` users can choose to do something different if desired.
        http_tests::TestResponse {
            status_code: 400,
            body: None,
            content_type: "application/json".to_string(),
        }
    }

    fn make_test_response(response: Response) -> http_tests::TestResponse {
        let status_code = response
            .status
            .expect("No status code returned from handler")
            .to_u16() as i32;
        let content_type = String::from_utf8(
            response
                .headers
                .get_raw("content-type")
                .expect("No content type header from handler")[0]
                .clone(),
        )
        .expect("Content-type header invalid UTF-8");
        let body = response::extract_body_to_string(response);

        http_tests::TestResponse {
            status_code,
            body: Some(body),
            content_type,
        }
    }

    fn make_handler() -> Box<dyn Handler> {
        Box::new(GraphQLHandler::new(
            context_factory,
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        ))
    }
}
