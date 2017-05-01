//! Optional handlers for the Iron framework. Requires the `iron-handlers` feature enabled.

use iron::prelude::*;
use iron::middleware::Handler;
use iron::mime::Mime;
use iron::status;
use iron::method;
use iron::url::Url;

use std::io::Read;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::boxed::Box;

use serde_json;
use serde_json::Value as Json;
use serde_json::error::Error as SerdeError;

use ::{InputValue, GraphQLType, RootNode, Variables, execute as execute_query};

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


    fn handle_get(&self, req: &mut Request) -> IronResult<Response> {
        let url: Url = req.url.clone().into();

        let mut query = None;
        let variables = Variables::new();

        for (k, v) in url.query_pairs() {
            if k == "query" {
                query = Some(v.into_owned());
            }
        }

        let query = iexpect!(query);

        self.execute(req, &query, &variables)
    }

    fn handle_post(&self, req: &mut Request) -> IronResult<Response> {
        let mut request_payload = String::new();
        itry!(req.body.read_to_string(&mut request_payload));
        let json_data =
            match serde_json::from_str::<Json>(&*request_payload) {
                Ok(json) => json,
                Err(err) => {
                    let error = IronError::new(
                        Box::new(GraphQlIronError::Serde(err)),
                        (status::BadRequest, "No JSON object was decoded."));
                    return Err(error)
                }
            };
        match json_data {
            Json::Object(json_obj) => {
                let mut query = None;
                let mut variables = Variables::new();
                for (k, v) in json_obj {
                    if k == "query" {
                        query = v.as_str().map(|query| query.to_owned());
                    }
                    else if k == "variables" {
                        variables = InputValue::from_json(v).to_object_value()
                            .map(|o| o.into_iter().map(|(k, v)| (k.to_owned(), v.clone())).collect())
                            .unwrap_or_default();
                    }
                }
                let query = iexpect!(query);
                self.execute(req, &query, &variables)
            }
            _ => {
                let error = IronError::new(
                    Box::new(GraphQlIronError::IO(IoError::new(ErrorKind::InvalidData,
                                                                "Was able parse a JSON item but it\
                                                                 was not an object as expected."))),
                            (status::BadRequest, "No JSON object was decoded."));
                Err(error)
            }
        }
    }

    fn execute(&self, req: &mut Request, query: &str, variables: &Variables) -> IronResult<Response> {
        let context = (self.context_factory)(req);
        let result = execute_query(query, None, &self.root_node, variables, &context);
        let content_type = "application/json".parse::<Mime>().unwrap();
        let mut map = HashMap::new();

        match result {
            Ok((result, errors)) => {
                let response_data = serde_json::to_value(result)
                    .expect("Failed to convert response data to JSON.");
                map.insert("data".to_owned(), response_data);

                if !errors.is_empty() {
                    let response_data = serde_json::to_value(errors)
                        .expect("Failed to convert the errors to JSON.");
                    map.insert("errors".to_owned(), response_data);
                }
                let data = serde_json::to_value(map).expect("Failed to convert response to JSON");
                let json = serde_json::to_string_pretty(&data).expect("Failed to convert response to JSON.");
                Ok(Response::with((content_type, status::Ok, json)))
            }
            Err(err) => {
                let response_data = serde_json::to_value(err)
                    .expect("Failed to convert error data to JSON.");
                map.insert("errors".to_owned(), response_data);
                let data = serde_json::to_value(map).expect("Failed to convert response to JSON");
                let json = serde_json::to_string_pretty(&data)
                    .expect("Failed to convert response to JSON");
                Ok(Response::with((content_type, status::BadRequest, json.to_string())))
            }
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
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match req.method {
            method::Get => self.handle_get(req),
            method::Post => self.handle_post(req),
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
pub enum GraphQlIronError {
    ///Captures any errors that were caused by Serde.
    Serde(SerdeError),
    /// Captures any error related the IO.
    IO(IoError)
}

impl fmt::Display for GraphQlIronError {
    fn fmt(&self, mut f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GraphQlIronError::Serde(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQlIronError::IO(ref err) => fmt::Display::fmt(err, &mut f),
        }
    }
}

impl fmt::Debug for GraphQlIronError {
    fn fmt(&self, mut f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GraphQlIronError::Serde(ref err) => fmt::Debug::fmt(err, &mut f),
            GraphQlIronError::IO(ref err) => fmt::Debug::fmt(err, &mut f),
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
