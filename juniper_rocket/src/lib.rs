#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate juniper;
extern crate serde_json;
extern crate rocket;

use std::io::{Cursor, Read};
use std::error::Error;

use rocket::Request;
use rocket::request::{FormItems, FromForm};
use rocket::data::{FromData, Outcome as FromDataOutcome};
use rocket::response::{content, Responder, Response};
use rocket::http::{ContentType, Status};
use rocket::Data;
use rocket::Outcome::{Failure, Forward, Success};

use juniper::InputValue;
use juniper::http;

use juniper::GraphQLType;
use juniper::RootNode;

/// Simple wrapper around an incoming GraphQL request
///
/// See the `http` module for more information. This type can be constructed
/// automatically from both GET and POST routes by implementing the `FromForm`
/// and `FromData` traits.
pub struct GraphQLRequest(http::GraphQLRequest);

/// Simple wrapper around the result of executing a GraphQL query
pub struct GraphQLResponse(Status, String);

/// Generate an HTML page containing GraphiQL
pub fn graphiql_source(graphql_endpoint_url: &str) -> content::Html<String> {
    content::Html(juniper::graphiql::graphiql_source(graphql_endpoint_url))
}

impl GraphQLRequest {
    /// Execute an incoming GraphQL query
    pub fn execute<CtxT, QueryT, MutationT>(
        &self,
        root_node: &RootNode<QueryT, MutationT>,
        context: &CtxT,
    ) -> GraphQLResponse
    where
        QueryT: GraphQLType<Context = CtxT>,
        MutationT: GraphQLType<Context = CtxT>,
    {
        let response = self.0.execute(root_node, context);
        let status = if response.is_ok() {
            Status::Ok
        } else {
            Status::BadRequest
        };
        let json = serde_json::to_string_pretty(&response).unwrap();

        GraphQLResponse(status, json)
    }
}

impl<'f> FromForm<'f> for GraphQLRequest {
    type Error = String;

    fn from_form(form_items: &mut FormItems<'f>, strict: bool) -> Result<Self, String> {
        let mut query = None;
        let mut operation_name = None;
        let mut variables = None;

        for (key, value) in form_items {
            match key.as_str() {
                "query" => if query.is_some() {
                    return Err("Query parameter must not occur more than once".to_owned());
                } else {
                    query = Some(value.as_str().to_string());
                },
                "operation_name" => if operation_name.is_some() {
                    return Err(
                        "Operation name parameter must not occur more than once".to_owned(),
                    );
                } else {
                    operation_name = Some(value.as_str().to_string());
                },
                "variables" => if variables.is_some() {
                    return Err(
                        "Variables parameter must not occur more than once".to_owned(),
                    );
                } else {
                    variables = Some(serde_json::from_str::<InputValue>(value.as_str())
                        .map_err(|err| err.description().to_owned())?);
                },
                _ => if strict {
                    return Err(format!("Prohibited extra field '{}'", key).to_owned());
                },
            }
        }

        if let Some(query) = query {
            Ok(GraphQLRequest(
                http::GraphQLRequest::new(query, operation_name, variables),
            ))
        } else {
            Err("Query parameter missing".to_owned())
        }
    }
}

impl FromData for GraphQLRequest {
    type Error = String;

    fn from_data(request: &Request, data: Data) -> FromDataOutcome<Self, Self::Error> {
        if !request.content_type().map_or(false, |ct| ct.is_json()) {
            return Forward(data);
        }

        let mut body = String::new();
        if let Err(e) = data.open().read_to_string(&mut body) {
            return Failure((Status::InternalServerError, format!("{:?}", e)));
        }

        match serde_json::from_str(&body) {
            Ok(value) => Success(GraphQLRequest(value)),
            Err(failure) => return Failure((Status::BadRequest, format!("{}", failure))),
        }
    }
}

impl<'r> Responder<'r> for GraphQLResponse {
    fn respond_to(self, _: &Request) -> Result<Response<'r>, Status> {
        let GraphQLResponse(status, body) = self;

        Ok(
            Response::build()
                .header(ContentType::new("application", "json"))
                .status(status)
                .sized_body(Cursor::new(body))
                .finalize(),
        )
    }
}

#[cfg(test)]
mod tests {

    use rocket;
    use rocket::Rocket;
    use rocket::http::{ContentType, Method};
    use rocket::State;

    use juniper::RootNode;
    use juniper::tests::model::Database;
    use juniper::http::tests as http_tests;
    use juniper::EmptyMutation;

    type Schema = RootNode<'static, Database, EmptyMutation<Database>>;


    #[get("/?<request>")]
    fn get_graphql_handler(
        context: State<Database>,
        request: super::GraphQLRequest,
        schema: State<Schema>,
    ) -> super::GraphQLResponse {
        request.execute(&schema, &context)
    }

    #[post("/", data = "<request>")]
    fn post_graphql_handler(
        context: State<Database>,
        request: super::GraphQLRequest,
        schema: State<Schema>,
    ) -> super::GraphQLResponse {
        request.execute(&schema, &context)
    }

    struct TestRocketIntegration {
        rocket: Rocket,
    }

    /*

    impl http_tests::HTTPIntegration for TestRocketIntegration
    {
        fn get(&self, url: &str) -> http_tests::TestResponse {
            make_test_response(&self.rocket, MockRequest::new(
                Method::Get,
                url))
        }

        fn post(&self, url: &str, body: &str) -> http_tests::TestResponse {
            make_test_response(
                &self.rocket,
                MockRequest::new(
                    Method::Post,
                    url,
                ).header(ContentType::JSON).body(body))
        }
    }

    #[test]
    fn test_rocket_integration() {
        let integration = TestRocketIntegration {
            rocket: make_rocket(),
        };

        http_tests::run_http_test_suite(&integration);
    }

    fn make_rocket() -> Rocket {
        rocket::ignite()
            .manage(Database::new())
            .manage(Schema::new(Database::new(), EmptyMutation::<Database>::new()))
            .mount("/", routes![post_graphql_handler, get_graphql_handler])
    }

    fn make_test_response<'r>(rocket: &'r Rocket, mut request: MockRequest<'r>) -> http_tests::TestResponse {
        let mut response = request.dispatch_with(&rocket);
        let status_code = response.status().code as i32;
        let content_type = response.header_values("content-type").collect::<Vec<_>>().into_iter().next()
            .expect("No content type header from handler").to_owned();
        let body = response.body().expect("No body returned from GraphQL handler").into_string();

        http_tests::TestResponse {
            status_code: status_code,
            body: body,
            content_type: content_type,
        }
    }

    */
}
