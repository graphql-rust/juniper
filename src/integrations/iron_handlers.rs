//! Optional handlers for the Iron framework. Requires the `iron-handlers` feature enabled.
use iron::prelude::*;
use iron::middleware::Handler;
use iron::mime::Mime;
use iron::status;
use iron::method;
use urlencoded::{UrlEncodedQuery, UrlDecodingError};

use std::io::Read;
use std::error::Error;
use std::fmt;

use serde_json;
use serde_json::error::Error as SerdeError;

use ::{InputValue, GraphQLType, RootNode};
use ::http;

/// Handler that executes GraphQL queries in the given schema
///
/// The handler responds to GET requests and POST requests only. In GET
/// requests, the query should be supplied in the `query` URL parameter, e.g.
/// `http://localhost:3000/graphql?query={hero{name}}`.
///
/// POST requests support both queries and variables. POST a JSON document to
/// this endpoint containing the field `"query"` and optionally `"variables"`.
/// The variables should be a JSON object containing the variable to value
/// mapping.
pub struct GraphQLHandler<'a, CtxFactory, Query, Mutation, CtxT>
    where CtxFactory: Fn(&mut Request) -> CtxT + Send + Sync + 'static,
          CtxT: 'static,
          Query: GraphQLType<Context=CtxT> + Send + Sync + 'static,
          Mutation: GraphQLType<Context=CtxT> + Send + Sync + 'static,
{
    context_factory: CtxFactory,
    root_node: RootNode<'a, Query, Mutation>,
}

/// Handler that renders GraphiQL - a graphical query editor interface
pub struct GraphiQLHandler {
    graphql_url: String,
}


fn get_single_value<T>(mut values: Vec<T>) -> IronResult<T> {
    if values.len() == 1 {
        Ok(values.remove(0))
    }
    else {
        Err(GraphQLIronError::InvalidData("Duplicate URL query parameter").into())
    }
}

fn parse_url_param(params: Option<Vec<String>>) -> IronResult<Option<String>> {
    if let Some(values) = params {
        get_single_value(values).map(Some)
    }
    else {
        Ok(None)
    }
}

fn parse_variable_param(params: Option<Vec<String>>) -> IronResult<Option<InputValue>> {
    if let Some(values) = params {
        Ok(serde_json::from_str::<InputValue>(get_single_value(values)?.as_ref())
            .map(Some)
            .map_err(GraphQLIronError::Serde)?)
    }
    else {
        Ok(None)
    }
}


impl<'a, CtxFactory, Query, Mutation, CtxT>
    GraphQLHandler<'a, CtxFactory, Query, Mutation, CtxT>
    where CtxFactory: Fn(&mut Request) -> CtxT + Send + Sync + 'static,
          CtxT: 'static,
          Query: GraphQLType<Context=CtxT> + Send + Sync + 'static,
          Mutation: GraphQLType<Context=CtxT> + Send + Sync + 'static,
{
    /// Build a new GraphQL handler
    ///
    /// The context factory will receive the Iron request object and is
    /// expected to construct a context object for the given schema. This can
    /// be used to construct e.g. database connections or similar data that
    /// the schema needs to execute the query.
    pub fn new(context_factory: CtxFactory, query: Query, mutation: Mutation) -> Self {
        GraphQLHandler {
            context_factory: context_factory,
            root_node: RootNode::new(query, mutation),
        }
    }


    fn handle_get(&self, req: &mut Request) -> IronResult<http::GraphQLRequest> {
        let url_query_string = req.get_mut::<UrlEncodedQuery>()
            .map_err(|e| GraphQLIronError::Url(e))?;
    
        let input_query = parse_url_param(url_query_string.remove("query"))?
            .ok_or_else(|| GraphQLIronError::InvalidData("No query provided"))?;
        let operation_name = parse_url_param(url_query_string.remove("operationName"))?;
        let variables = parse_variable_param(url_query_string.remove("variables"))?;

        Ok(http::GraphQLRequest::new(input_query, operation_name, variables))
    }

    fn handle_post(&self, req: &mut Request) -> IronResult<http::GraphQLRequest> {
        let mut request_payload = String::new();
        itry!(req.body.read_to_string(&mut request_payload));
        
        Ok(serde_json::from_str::<http::GraphQLRequest>(request_payload.as_str())
            .map_err(|err| GraphQLIronError::Serde(err))?)
    }

    fn execute(&self, context: &CtxT, request: http::GraphQLRequest) -> IronResult<Response> {
        let response = request.execute(
            &self.root_node,
            context,
        );
        let content_type = "application/json".parse::<Mime>().unwrap();
        let json = serde_json::to_string_pretty(&response).unwrap();
        let status = if response.is_ok() { status::Ok } else { status::BadRequest };
        Ok(Response::with((content_type, status, json)))
    }
}

impl GraphiQLHandler {
    /// Build a new GraphiQL handler targeting the specified URL.
    ///
    /// The provided URL should point to the URL of the attached `GraphQLHandler`. It can be
    /// relative, so a common value could be `"/graphql"`.
    pub fn new(graphql_url: &str) -> GraphiQLHandler {
        GraphiQLHandler {
            graphql_url: graphql_url.to_owned(),
        }
    }
}

impl<'a, CtxFactory, Query, Mutation, CtxT>
    Handler
    for GraphQLHandler<'a, CtxFactory, Query, Mutation, CtxT>
    where CtxFactory: Fn(&mut Request) -> CtxT + Send + Sync + 'static,
          CtxT: 'static,
          Query: GraphQLType<Context=CtxT> + Send + Sync + 'static,
          Mutation: GraphQLType<Context=CtxT> + Send + Sync + 'static, 'a: 'static,
{
    fn handle(&self, mut req: &mut Request) -> IronResult<Response> {
        let context = (self.context_factory)(req);

        let graphql_request = match req.method {
            method::Get => self.handle_get(&mut req)?,
            method::Post => self.handle_post(&mut req)?,
            _ => return Ok(Response::with((status::MethodNotAllowed)))
        };

        self.execute(&context, graphql_request)
    }
}

impl Handler for GraphiQLHandler {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let content_type = "text/html".parse::<Mime>().unwrap();

        Ok(Response::with((
            content_type,
            status::Ok,
            ::graphiql::graphiql_source(&self.graphql_url),
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
            GraphQLIronError::InvalidData(ref err) => fmt::Display::fmt(err, &mut f),
        }
    }
}

impl Error for GraphQLIronError {
    fn description(&self) -> &str {
       match *self {
           GraphQLIronError::Serde(ref err) => err.description(),
           GraphQLIronError::Url(ref err) => err.description(),
           GraphQLIronError::InvalidData(ref err) => err,
       }
    }

    fn cause(&self) -> Option<&Error> {
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
    use iron::prelude::*;
    use iron_test::{request, response};
    use iron::{Handler, Headers};

    use ::tests::model::Database;
    use ::http::tests as http_tests;
    use types::scalars::EmptyMutation;

    use super::GraphQLHandler;

    struct TestIronIntegration;

    impl http_tests::HTTPIntegration for TestIronIntegration
    {
        fn get(&self, url: &str) -> http_tests::TestResponse {
            make_test_response(request::get(
                &("http://localhost:3000".to_owned() + url),
                Headers::new(),
                &make_handler(),
            ))
        }

        fn post(&self, url: &str, body: &str) -> http_tests::TestResponse {
            make_test_response(request::post(
                &("http://localhost:3000".to_owned() + url),
                Headers::new(),
                body,
                &make_handler(),
            ))
        }
    }

    #[test]
    fn test_iron_integration() {
        let integration = TestIronIntegration;

        http_tests::run_http_test_suite(&integration);
    }

    fn context_factory(_: &mut Request) -> Database {
        Database::new()
    }

    fn make_test_response(response: IronResult<Response>) -> http_tests::TestResponse {
        let response = response.expect("Error response from GraphQL handler");
        let status_code = response.status.expect("No status code returned from handler").to_u16() as i32;
        let content_type = String::from_utf8(
            response.headers.get_raw("content-type")
                .expect("No content type header from handler")[0].clone())
            .expect("Content-type header invalid UTF-8");
        let body = response::extract_body_to_string(response);

        http_tests::TestResponse {
            status_code: status_code,
            body: Some(body),
            content_type: content_type,
        }
    }

    fn make_handler() -> Box<Handler> {
        Box::new(GraphQLHandler::new(
            context_factory,
            Database::new(),
            EmptyMutation::<Database>::new(),
        ))
    }
}
