use std::io::{Cursor, Read};

use serde_json;

use rocket::Request;
use rocket::data::{FromData, Outcome};
use rocket::response::{Responder, Response, content};
use rocket::http::{ContentType, Status};
use rocket::Data;
use rocket::Outcome::{Forward, Failure, Success};

use ::http;

use types::base::GraphQLType;
use schema::model::RootNode;

pub struct GraphQLResponse(Status, String);
pub struct GraphQLRequest(http::GraphQLRequest);

pub fn graphiql_source(graphql_endpoint_url: &str) -> content::HTML<String> {
    content::HTML(::graphiql::graphiql_source(graphql_endpoint_url))
}

impl GraphQLRequest {
    pub fn execute<CtxT, QueryT, MutationT>(
        &self,
        root_node: &RootNode<QueryT, MutationT>,
        context: &CtxT,
    )
        -> GraphQLResponse
        where QueryT: GraphQLType<Context=CtxT>,
            MutationT: GraphQLType<Context=CtxT>,
    {
        let response = self.0.execute(root_node, context);
        let status = if response.is_ok() { Status::Ok } else { Status::BadRequest };
        let json = serde_json::to_string_pretty(&response).unwrap();

        GraphQLResponse(status, json)
    }
}

impl FromData for GraphQLRequest {
    type Error = String;

    fn from_data(request: &Request, data: Data) -> Outcome<Self, String> {
        if !request.content_type().map_or(false, |ct| ct.is_json()) {
            return Forward(data);
        }

        let mut body = String::new();
        if let Err(e) = data.open().read_to_string(&mut body) {
            return Failure((Status::InternalServerError, format!("{:?}", e)));
        }

        match serde_json::from_str(&body) {
            Ok(value) => Success(GraphQLRequest(value)),
            Err(failure) => return Failure(
                (Status::BadRequest, format!("{}", failure)),
            ),
        }
    }
}

impl<'r> Responder<'r> for GraphQLResponse {
    fn respond(self) -> Result<Response<'r>, Status> {
        let GraphQLResponse(status, body) = self;

        Ok(Response::build()
            .header(ContentType::new("application", "json"))
            .status(status)
            .sized_body(Cursor::new(body))
            .finalize())
    }
}
