//! Optional handlers for the Iron framework. Requires the `iron-handlers` feature enabled.
use iron::prelude::*;
use iron::middleware::Handler;
use iron::mime::Mime;
use iron::status;
use iron::method;
use urlencoded::{UrlEncodedQuery, UrlDecodingError};

use std::io::Read;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::error::Error;
use std::fmt;
use std::boxed::Box;

use serde_json;
use serde_json::error::Error as SerdeError;

use ::{InputValue, GraphQLType, RootNode, execute};
use super::serde::{WrappedGraphQLResult, GraphQLQuery};

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


/// Get queries are allowed to repeat the same key more than once.
fn check_for_repeat_keys(params: &Vec<String>) -> Result<(), IronError> {
    if params.len() > 1 {
        let error = IronError::new(
            Box::new(GraphQlIronError::IO(IoError::new(ErrorKind::InvalidData,
                                                        "Was able parse a query string \
                                                        but a duplicate uri key was \
                                                        found."))),
                    (status::BadRequest, "Duplicate uri key was found."));
        Err(error)
    }
    else {
        Ok(())
    }
}

fn parse_url_param(param: Option<Vec<String>>) -> Result<Option<String>, IronError> {
    if let Some(values) = param {
            check_for_repeat_keys(&values)?;
            Ok(Some(values[0].to_owned()))
    }
    else {
        Ok(None)
    }
}

fn parse_variable_param(param: Option<Vec<String>>) -> Result<Option<InputValue>, IronError> {
    if let Some(values) = param {
        check_for_repeat_keys(&values)?;
        match serde_json::from_str::<InputValue>(values[0].as_ref()) {
            Ok(input_values) => {
                Ok(Some(input_values))
            }
            Err(err) => {
                Err(IronError::new(
                    Box::new(GraphQlIronError::Serde(err)),
                    (status::BadRequest, "No JSON object was decoded.")))
            }
        }
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


    fn handle_get(&self, req: &mut Request) -> IronResult<GraphQLQuery> {
         match req.get_mut::<UrlEncodedQuery>() {
            Ok(ref mut query_string) => {
                let input_query = parse_url_param(query_string.remove("query").to_owned())?;
                if let Some(query) = input_query {
                    let operation_name =
                        parse_url_param(query_string.remove("operationName"))?;
                    let input_variables =
                        parse_variable_param(query_string.remove("variables"))?;
                        Ok(GraphQLQuery::new(query,operation_name,input_variables))
                } else {
                    Err(IronError::new(
                        Box::new(GraphQlIronError::IO(IoError::new(ErrorKind::InvalidData,
                                                                    "No query key was found in \
                                                                    the Get request."))),
                                (status::BadRequest, "No query was provided.")))
                }
            }
            Err(err) => {
                Err(IronError::new(
                    Box::new(GraphQlIronError::Url(err)),
                            (status::BadRequest, "No JSON object was decoded.")))
            }
        }
    }

    fn handle_post(&self, req: &mut Request) -> IronResult<GraphQLQuery> {
        let mut request_payload = String::new();
        itry!(req.body.read_to_string(&mut request_payload));
        let graphql_query = serde_json::from_str::<GraphQLQuery>(request_payload.as_str()).map_err(|err|{
            IronError::new(
                Box::new(GraphQlIronError::Serde(err)),
                (status::BadRequest, "No JSON object was decoded."))
        });
        graphql_query
    }

    fn respond(&self, req: &mut Request, graphql: GraphQLQuery) -> IronResult<Response> {
       let context = (self.context_factory)(req);
       let variables = graphql.variables();
       let result = execute(graphql.query(),
                            graphql.operation_name(),
                            &self.root_node,
                            &variables,
                            &context);
      let content_type = "application/json".parse::<Mime>().unwrap();
      if result.is_ok() {
          let response = WrappedGraphQLResult::new(result);
          let json = serde_json::to_string_pretty(&response).unwrap();
          Ok(Response::with((content_type, status::Ok, json)))
      } else {
          let response = WrappedGraphQLResult::new(result);
          let json = serde_json::to_string_pretty(&response).unwrap();
          Ok(Response::with((content_type, status::BadRequest, json)))
      }
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
        match req.method {
            method::Get =>  {
                let graphql_query = self.handle_get(&mut req)?;
                self.respond(&mut req, graphql_query)
            }
            method::Post => {
                let graphql_query = self.handle_post(&mut req)?;
                self.respond(&mut req, graphql_query)
            },
            _ => Ok(Response::with((status::MethodNotAllowed)))
        }
    }
}

impl Handler for GraphiQLHandler {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let content_type = "text/html".parse::<Mime>().unwrap();

        let stylesheet_source = r#"
        <style>
            html, body, #app {
                height: 100%;
                margin: 0;
                overflow: hidden;
                width: 100%;
            }
        </style>
        "#;
        let fetcher_source = r#"
        <script>
            function graphQLFetcher(params) {
                return fetch(GRAPHQL_URL, {
                    method: 'post',
                    headers: {
                        'Accept': 'application/json',
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify(params)
                }).then(function (response) {
                    return response.text();
                }).then(function (body) {
                    try {
                        return JSON.parse(body);
                    } catch (error) {
                        return body;
                    }
                });
            }
            ReactDOM.render(
                React.createElement(GraphiQL, {
                    fetcher: graphQLFetcher,
                }),
                document.querySelector('#app'));
        </script>
        "#;

        let source = format!(r#"
<!DOCTYPE html>
<html>
    <head>
        <title>GraphQL</title>
        {stylesheet_source}
        <link rel="stylesheet" type="text/css" href="//cdnjs.cloudflare.com/ajax/libs/graphiql/0.8.1/graphiql.css">
    </head>
    <body>
        <div id="app"></div>

        <script src="//cdnjs.cloudflare.com/ajax/libs/fetch/2.0.1/fetch.js"></script>
        <script src="//cdnjs.cloudflare.com/ajax/libs/react/15.4.1/react.js"></script>
        <script src="//cdnjs.cloudflare.com/ajax/libs/react/15.4.1/react-dom.js"></script>
        <script src="//cdnjs.cloudflare.com/ajax/libs/graphiql/0.8.1/graphiql.js"></script>
        <script>var GRAPHQL_URL = '{graphql_url}';</script>
        {fetcher_source}
    </body>
</html>
"#,
            graphql_url = self.graphql_url,
            stylesheet_source = stylesheet_source,
            fetcher_source = fetcher_source);

        Ok(Response::with((content_type, status::Ok, source)))
    }
}

/// A general error allowing the developer to see the underlying issue.
#[derive(Debug)]
pub enum GraphQlIronError {
    ///Captures any errors that were caused by Serde.
    Serde(SerdeError),
    /// Captures any error related the IO.
    IO(IoError),
    /// Captures any error related to Url Decoding,
    Url(UrlDecodingError)
}

impl fmt::Display for GraphQlIronError {
    fn fmt(&self, mut f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GraphQlIronError::Serde(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQlIronError::IO(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQlIronError::Url(ref err) => fmt::Display::fmt(err, &mut f),
        }
    }
}

impl Error for GraphQlIronError {
    fn description(&self) -> &str {
       match *self {
           GraphQlIronError::Serde(ref err) => {
               err.description()
           },
           GraphQlIronError::IO(ref err) => {
               err.description()
           }
           GraphQlIronError::Url(ref err) => {
               err.description()
           }
       }
   }

   fn cause(&self) -> Option<&Error> {
       match *self {
           GraphQlIronError::Serde(ref err) => {
               err.cause()
           }
           GraphQlIronError::IO(ref err) => {
               err.cause()
           }
           GraphQlIronError::Url(ref err) => {
               err.cause()
           }
       }
   }
}


#[cfg(test)]
mod tests {
    use serde_json::Value as Json;
    use serde_json;
    use iron::prelude::*;
    use iron::status;
    use iron::headers;
    use iron_test::{request, response};
    use iron::{Handler, Headers};

    use ::tests::model::Database;
    use types::scalars::EmptyMutation;

    use super::GraphQLHandler;

    fn context_factory(_: &mut Request) -> Database {
        Database::new()
    }

    fn make_handler() -> Box<Handler> {
        Box::new(GraphQLHandler::new(
            context_factory,
            Database::new(),
            EmptyMutation::<Database>::new(),
        ))
    }

    fn unwrap_json_response(resp: Response) -> Json {
        let result = response::extract_body_to_string(resp);

        serde_json::from_str::<Json>(&result).expect("Could not parse JSON object")
    }

    #[test]
    fn test_simple_get() {
        let response = request::get(
            "http://localhost:3000/?query={hero{name}}",
            Headers::new(),
            &make_handler())
            .expect("Unexpected IronError");

        assert_eq!(response.status, Some(status::Ok));
        assert_eq!(response.headers.get::<headers::ContentType>(),
                   Some(&headers::ContentType::json()));

        let json = unwrap_json_response(response);

        assert_eq!(
            json,
            serde_json::from_str::<Json>(r#"{"data": {"hero": {"name": "R2-D2"}}}"#)
                .expect("Invalid JSON constant in test"));
    }

    #[test]
    fn test_encoded_get() {
        let response = request::get(
            "http://localhost:3000/?query=query%20{%20%20%20human(id:%20\"1000\")%20{%20%20%20%20%20id,%20%20%20%20%20name,%20%20%20%20%20appearsIn,%20%20%20%20%20homePlanet%20%20%20}%20}",
            Headers::new(),
            &make_handler())
            .expect("Unexpected IronError");

        assert_eq!(response.status, Some(status::Ok));
        assert_eq!(response.headers.get::<headers::ContentType>(),
                   Some(&headers::ContentType::json()));

        let json = unwrap_json_response(response);

        assert_eq!(
            json,
            serde_json::from_str::<Json>(r#"{
                    "data": {
                        "human": {
                            "appearsIn": [
                                "NEW_HOPE",
                                "EMPIRE",
                                "JEDI"
                                ],
                                "homePlanet": "Tatooine",
                                "name": "Luke Skywalker",
                                "id": "1000"
                            }
                        }
                    }"#)
                .expect("Invalid JSON constant in test"));
    }

    #[test]
    fn test_get_with_variables() {
        let response = request::get(
            "http://localhost:3000/?query=query($id:%20String!)%20{%20%20%20human(id:%20$id)%20{%20%20%20%20%20id,%20%20%20%20%20name,%20%20%20%20%20appearsIn,%20%20%20%20%20homePlanet%20%20%20}%20}&variables={%20%20%20\"id\":%20%20\"1000\"%20}",
            Headers::new(),
            &make_handler())
            .expect("Unexpected IronError");

        assert_eq!(response.status, Some(status::Ok));
        assert_eq!(response.headers.get::<headers::ContentType>(),
                   Some(&headers::ContentType::json()));

        let json = unwrap_json_response(response);

        assert_eq!(
            json,
            serde_json::from_str::<Json>(r#"{
                    "data": {
                        "human": {
                            "appearsIn": [
                                "NEW_HOPE",
                                "EMPIRE",
                                "JEDI"
                                ],
                                "homePlanet": "Tatooine",
                                "name": "Luke Skywalker",
                                "id": "1000"
                            }
                        }
                    }"#)
                .expect("Invalid JSON constant in test"));
    }


    #[test]
    fn test_simple_post() {
        let response = request::post(
            "http://localhost:3000/",
            Headers::new(),
            r#"{"query": "{hero{name}}"}"#,
            &make_handler())
            .expect("Unexpected IronError");

        assert_eq!(response.status, Some(status::Ok));
        assert_eq!(response.headers.get::<headers::ContentType>(),
                   Some(&headers::ContentType::json()));

        let json = unwrap_json_response(response);

        assert_eq!(
            json,
            serde_json::from_str::<Json>(r#"{"data": {"hero": {"name": "R2-D2"}}}"#)
                .expect("Invalid JSON constant in test"));
    }

    #[test]
    fn test_unsupported_method() {
        let response = request::options(
            "http://localhost:3000/?query={hero{name}}",
            Headers::new(),
            &make_handler())
            .expect("Unexpected IronError");

        assert_eq!(response.status, Some(status::MethodNotAllowed));
    }
}
