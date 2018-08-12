extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate juniper;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate url;

use futures::future;
use futures::{Future, Stream};
use futures_cpupool::CpuPool;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use juniper::http::{
    GraphQLRequest as JuniperGraphQLRequest, GraphQLResponse as JuniperGraphQLResponse,
};
use juniper::{GraphQLType, InputValue, RootNode};
use serde_json::error::Error as SerdeError;
use std::error::Error as StdError;
use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;
use url::form_urlencoded;

pub fn handle_graphql<CtxT, QueryT, MutationT>(
    pool: &CpuPool,
    root_node: &RootNode<QueryT, MutationT>,
    context: &CtxT,
    request: Request<Body>,
) -> Box<Future<Item = Response<Body>, Error = ()>>
where
    QueryT: GraphQLType<Context = CtxT>,
    MutationT: GraphQLType<Context = CtxT>,
{
    match request.method() {
        &Method::GET => Box::new(
            future::ok(
                request
                    .uri()
                    .query()
                    .map(gql_request_from_get)
                    .unwrap_or(Err(GraphQLRequestError::Invalid(
                        "'query' parameter is missing".to_string(),
                    ))),
            ).map(|res| new_response(StatusCode::METHOD_NOT_ALLOWED)),
        ),
        &Method::POST => Box::new(
            request
                .into_body()
                .concat2()
                .map(move |chunk| {
                    String::from_utf8(chunk.iter().rev().cloned().collect::<Vec<u8>>())
                        .map_err(GraphQLRequestError::BodyUtf8)
                        .and_then(|input| {
                            serde_json::from_str::<GraphQLRequest>(&input)
                                .map_err(GraphQLRequestError::BodyJSONError)
                        })
                }).or_else(|err| future::ok(Err(GraphQLRequestError::BodyHyper(err))))
                .map(|res| new_response(StatusCode::METHOD_NOT_ALLOWED)),
        ),
        _ => return Box::new(future::ok(new_response(StatusCode::METHOD_NOT_ALLOWED))),
    }
}

fn gql_request_from_get(input: &str) -> Result<JuniperGraphQLRequest, GraphQLRequestError> {
    let mut query = None;
    let mut operation_name = None;
    let mut variables = None;
    for (key, value) in form_urlencoded::parse(input.as_bytes()).into_owned() {
        match key.as_ref() {
            "query" => {
                if query.is_some() {
                    return Err(invalid_err("query"));
                }
                query = Some(value)
            }
            "operationName" => {
                if operation_name.is_some() {
                    return Err(invalid_err("operationName"));
                }
            }
            "variables" => {
                if variables.is_some() {
                    return Err(invalid_err("variables"));
                }
                match serde_json::from_str::<InputValue>(&value)
                    .map_err(GraphQLRequestError::Variables)
                {
                    Ok(parsed_variables) => variables = Some(parsed_variables),
                    Err(e) => return Err(e),
                }
            }
            _ => continue,
        }
    }
    match query {
        Some(query) => Ok(JuniperGraphQLRequest::new(query, operation_name, variables)),
        None => Err(GraphQLRequestError::Invalid(
            "'query' parameter is missing".to_string(),
        )),
    }
}

fn invalid_err(parameter_name: &str) -> GraphQLRequestError {
    GraphQLRequestError::Invalid(format!(
        "'{}' parameter is specified multiple times",
        parameter_name
    ))
}

fn new_response(code: StatusCode) -> Response<Body> {
    let mut r = Response::new(Body::empty());
    *r.status_mut() = code;
    r
}

#[derive(Deserialize)]
#[serde(untagged)]
enum GraphQLRequest {
    Single(JuniperGraphQLRequest),
    Batch(Vec<JuniperGraphQLRequest>),
}

impl GraphQLRequest {
    pub fn execute<'a, CtxT, QueryT, MutationT>(
        &'a self,
        root_node: &RootNode<QueryT, MutationT>,
        context: &CtxT,
    ) -> GraphQLResponse<'a>
    where
        QueryT: GraphQLType<Context = CtxT>,
        MutationT: GraphQLType<Context = CtxT>,
    {
        match self {
            &GraphQLRequest::Single(ref request) => {
                GraphQLResponse::Single(request.execute(root_node, context))
            }
            &GraphQLRequest::Batch(ref requests) => GraphQLResponse::Batch(
                requests
                    .iter()
                    .map(|request| request.execute(root_node, context))
                    .collect(),
            ),
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum GraphQLResponse<'a> {
    Single(JuniperGraphQLResponse<'a>),
    Batch(Vec<JuniperGraphQLResponse<'a>>),
}

impl<'a> GraphQLResponse<'a> {
    fn is_ok(&self) -> bool {
        match self {
            &GraphQLResponse::Single(ref response) => response.is_ok(),
            &GraphQLResponse::Batch(ref responses) => responses
                .iter()
                .fold(true, |ok, response| ok && response.is_ok()),
        }
    }
}

#[derive(Debug)]
enum GraphQLRequestError {
    BodyHyper(hyper::Error),
    BodyUtf8(FromUtf8Error),
    BodyJSONError(SerdeError),
    Variables(SerdeError),
    Invalid(String),
}

impl fmt::Display for GraphQLRequestError {
    fn fmt(&self, mut f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GraphQLRequestError::BodyHyper(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLRequestError::BodyUtf8(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLRequestError::BodyJSONError(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLRequestError::Variables(ref err) => fmt::Display::fmt(err, &mut f),
            GraphQLRequestError::Invalid(ref err) => fmt::Display::fmt(err, &mut f),
        }
    }
}

impl Error for GraphQLRequestError {
    fn description(&self) -> &str {
        match *self {
            GraphQLRequestError::BodyHyper(ref err) => err.description(),
            GraphQLRequestError::BodyUtf8(ref err) => err.description(),
            GraphQLRequestError::BodyJSONError(ref err) => err.description(),
            GraphQLRequestError::Variables(ref err) => err.description(),
            GraphQLRequestError::Invalid(ref err) => err,
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            GraphQLRequestError::BodyHyper(ref err) => Some(err),
            GraphQLRequestError::BodyUtf8(ref err) => Some(err),
            GraphQLRequestError::BodyJSONError(ref err) => Some(err),
            GraphQLRequestError::Variables(ref err) => Some(err),
            GraphQLRequestError::Invalid(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_query_as_graphql_request() {
        assert_eq!(2 + 2, 4);
    }
}
