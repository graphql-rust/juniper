/*!

# juniper_rocket

This repository contains the [Rocket][Rocket] web server integration for
[Juniper][Juniper], a [GraphQL][GraphQL] implementation for Rust.

## Documentation

For documentation, including guides and examples, check out [Juniper][Juniper].

A basic usage example can also be found in the [Api documentation][documentation].

## Examples

Check [examples/rocket_server.rs][example] for example code of a working Rocket
server with GraphQL handlers.

## Links

* [Juniper][Juniper]
* [Api Reference][documentation]
* [Rocket][Rocket]

## License

This project is under the BSD-2 license.

Check the LICENSE file for details.

[Rocket]: https://rocket.rs
[Juniper]: https://github.com/graphql-rust/juniper
[GraphQL]: http://graphql.org
[documentation]: https://docs.rs/juniper_rocket
[example]: https://github.com/graphql-rust/juniper_rocket/blob/master/examples/rocket_server.rs

*/

#![doc(html_root_url = "https://docs.rs/juniper_rocket/0.2.0")]
#![feature(decl_macro, proc_macro_hygiene)]

use std::io::{Cursor, Read};

use juniper::{
    http::{self, GraphQLBatchRequest},
    DefaultScalarValue, FieldError, GraphQLType, InputValue, RootNode, ScalarValue,
};
use rocket::{
    data::{FromDataSimple, Outcome as FromDataOutcome},
    http::{ContentType, RawStr, Status},
    request::{FormItems, FromForm, FromFormValue},
    response::{content, Responder, Response},
    Data,
    Outcome::{Forward, Success},
    Request,
};

/// Simple wrapper around an incoming GraphQL request
///
/// See the `http` module for more information. This type can be constructed
/// automatically from both GET and POST routes by implementing the `FromForm`
/// and `FromData` traits.
#[derive(Debug, PartialEq)]
pub struct GraphQLRequest<S = DefaultScalarValue>(GraphQLBatchRequest<S>)
where
    S: ScalarValue;

/// Simple wrapper around the result of executing a GraphQL query
pub struct GraphQLResponse(pub Status, pub String);

/// Generate an HTML page containing GraphiQL
pub fn graphiql_source(
    graphql_endpoint_url: &str,
    subscriptions_endpoint: Option<&str>,
) -> content::Html<String> {
    content::Html(juniper::http::graphiql::graphiql_source(
        graphql_endpoint_url,
        subscriptions_endpoint,
    ))
}

/// Generate an HTML page containing GraphQL Playground
pub fn playground_source(
    graphql_endpoint_url: &str,
    subscriptions_endpoint: Option<&str>,
) -> content::Html<String> {
    content::Html(juniper::http::playground::playground_source(
        graphql_endpoint_url,
        subscriptions_endpoint,
    ))
}

impl<S> GraphQLRequest<S>
where
    S: ScalarValue,
{
    /// Execute an incoming GraphQL query
    pub fn execute_sync<CtxT, QueryT, MutationT, SubscriptionT>(
        &self,
        root_node: &RootNode<QueryT, MutationT, SubscriptionT, S>,
        context: &CtxT,
    ) -> GraphQLResponse
    where
        QueryT: GraphQLType<S, Context = CtxT>,
        MutationT: GraphQLType<S, Context = CtxT>,
        SubscriptionT: GraphQLType<S, Context = CtxT>,
    {
        let response = self.0.execute_sync(root_node, context);
        let status = if response.is_ok() {
            Status::Ok
        } else {
            Status::BadRequest
        };
        let json = serde_json::to_string(&response).unwrap();

        GraphQLResponse(status, json)
    }

    /// Returns the operation names associated with this request.
    ///
    /// For batch requests there will be multiple names.
    pub fn operation_names(&self) -> Vec<Option<&str>> {
        self.0.operation_names()
    }
}

impl GraphQLResponse {
    /// Constructs an error response outside of the normal execution flow
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(decl_macro, proc_macro_hygiene)]
    /// #
    /// # use rocket::http::Cookies;
    /// # use rocket::request::Form;
    /// # use rocket::response::content;
    /// # use rocket::State;
    /// #
    /// # use juniper::tests::fixtures::starwars::schema::{Database, Query};
    /// # use juniper::{EmptyMutation, EmptySubscription, FieldError, RootNode, Value};
    /// #
    /// # type Schema = RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;
    /// #
    /// #[rocket::get("/graphql?<request..>")]
    /// fn get_graphql_handler(
    ///     mut cookies: Cookies,
    ///     context: State<Database>,
    ///     request: Form<juniper_rocket::GraphQLRequest>,
    ///     schema: State<Schema>,
    /// ) -> juniper_rocket::GraphQLResponse {
    ///     if cookies.get("user_id").is_none() {
    ///         let err = FieldError::new("User is not logged in", Value::null());
    ///         return juniper_rocket::GraphQLResponse::error(err);
    ///     }
    ///
    ///     request.execute_sync(&schema, &context)
    /// }
    /// ```
    pub fn error(error: FieldError) -> Self {
        let response = http::GraphQLResponse::error(error);
        let json = serde_json::to_string(&response).unwrap();
        GraphQLResponse(Status::BadRequest, json)
    }

    /// Constructs a custom response outside of the normal execution flow
    ///
    /// This is intended for highly customized integrations and should only
    /// be used as a last resort. For normal juniper use, use the response
    /// from GraphQLRequest::execute_sync(..).
    pub fn custom(status: Status, response: serde_json::Value) -> Self {
        let json = serde_json::to_string(&response).unwrap();
        GraphQLResponse(status, json)
    }
}

impl<'f, S> FromForm<'f> for GraphQLRequest<S>
where
    S: ScalarValue,
{
    type Error = String;

    fn from_form(form_items: &mut FormItems<'f>, strict: bool) -> Result<Self, String> {
        let mut query = None;
        let mut operation_name = None;
        let mut variables = None;

        for form_item in form_items {
            let (key, value) = form_item.key_value();
            // Note: we explicitly decode in the match arms to save work rather
            // than decoding every form item blindly.
            match key.as_str() {
                "query" => {
                    if query.is_some() {
                        return Err("Query parameter must not occur more than once".to_owned());
                    } else {
                        match value.url_decode() {
                            Ok(v) => query = Some(v),
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                }
                "operation_name" => {
                    if operation_name.is_some() {
                        return Err(
                            "Operation name parameter must not occur more than once".to_owned()
                        );
                    } else {
                        match value.url_decode() {
                            Ok(v) => operation_name = Some(v),
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                }
                "variables" => {
                    if variables.is_some() {
                        return Err("Variables parameter must not occur more than once".to_owned());
                    } else {
                        let decoded;
                        match value.url_decode() {
                            Ok(v) => decoded = v,
                            Err(e) => return Err(e.to_string()),
                        }
                        variables = Some(
                            serde_json::from_str::<InputValue<_>>(&decoded)
                                .map_err(|err| err.to_string())?,
                        );
                    }
                }
                _ => {
                    if strict {
                        return Err(format!("Prohibited extra field '{}'", key));
                    }
                }
            }
        }

        if let Some(query) = query {
            Ok(GraphQLRequest(GraphQLBatchRequest::Single(
                http::GraphQLRequest::new(query, operation_name, variables),
            )))
        } else {
            Err("Query parameter missing".to_owned())
        }
    }
}

impl<'v, S> FromFormValue<'v> for GraphQLRequest<S>
where
    S: ScalarValue,
{
    type Error = String;

    fn from_form_value(form_value: &'v RawStr) -> Result<Self, Self::Error> {
        let mut form_items = FormItems::from(form_value);

        Self::from_form(&mut form_items, true)
    }
}

impl<S> FromDataSimple for GraphQLRequest<S>
where
    S: ScalarValue,
{
    type Error = String;

    fn from_data(req: &Request, data: Data) -> FromDataOutcome<Self, Self::Error> {
        let content_type = req
            .content_type()
            .map(|ct| (ct.top().as_str(), ct.sub().as_str()));
        let is_json = match content_type {
            Some(("application", "json")) => true,
            Some(("application", "graphql")) => false,
            _ => return Forward(data),
        };

        let mut body = String::new();
        data.open()
            .read_to_string(&mut body)
            .map_err(|e| Err((Status::InternalServerError, format!("{:?}", e))))?;

        Success(GraphQLRequest(if is_json {
            serde_json::from_str(&body).map_err(|e| Err((Status::BadRequest, format!("{}", e))))?
        } else {
            GraphQLBatchRequest::Single(http::GraphQLRequest::new(body, None, None))
        }))
    }
}

impl<'r> Responder<'r> for GraphQLResponse {
    fn respond_to(self, _: &Request) -> Result<Response<'r>, Status> {
        let GraphQLResponse(status, body) = self;

        Ok(Response::build()
            .header(ContentType::new("application", "json"))
            .status(status)
            .sized_body(Cursor::new(body))
            .finalize())
    }
}

#[cfg(test)]
mod fromform_tests {
    use super::*;
    use juniper::InputValue;
    use rocket::request::{FormItems, FromForm};
    use std::str;

    fn check_error(input: &str, error: &str, strict: bool) {
        let mut items = FormItems::from(input);
        let result: Result<GraphQLRequest, _> = GraphQLRequest::from_form(&mut items, strict);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), error);
    }

    #[test]
    fn test_empty_form() {
        check_error("", "Query parameter missing", false);
    }

    #[test]
    fn test_no_query() {
        check_error(
            "operation_name=foo&variables={}",
            "Query parameter missing",
            false,
        );
    }

    #[test]
    fn test_strict() {
        check_error("query=test&foo=bar", "Prohibited extra field \'foo\'", true);
    }

    #[test]
    fn test_duplicate_query() {
        check_error(
            "query=foo&query=bar",
            "Query parameter must not occur more than once",
            false,
        );
    }

    #[test]
    fn test_duplicate_operation_name() {
        check_error(
            "query=test&operation_name=op1&operation_name=op2",
            "Operation name parameter must not occur more than once",
            false,
        );
    }

    #[test]
    fn test_duplicate_variables() {
        check_error(
            "query=test&variables={}&variables={}",
            "Variables parameter must not occur more than once",
            false,
        );
    }

    #[test]
    fn test_variables_invalid_json() {
        check_error(
            "query=test&variables=NOT_JSON",
            "expected value at line 1 column 1",
            false,
        );
    }

    #[test]
    fn test_variables_valid_json() {
        let form_string = r#"query=test&variables={"foo":"bar"}"#;
        let mut items = FormItems::from(form_string);
        let result = GraphQLRequest::from_form(&mut items, false);
        assert!(result.is_ok());
        let variables = ::serde_json::from_str::<InputValue>(r#"{"foo":"bar"}"#).unwrap();
        let expected = GraphQLRequest(GraphQLBatchRequest::Single(http::GraphQLRequest::new(
            "test".to_string(),
            None,
            Some(variables),
        )));
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_variables_encoded_json() {
        let form_string = r#"query=test&variables={"foo": "x%20y%26%3F+z"}"#;
        let mut items = FormItems::from(form_string);
        let result = GraphQLRequest::from_form(&mut items, false);
        assert!(result.is_ok());
        let variables = ::serde_json::from_str::<InputValue>(r#"{"foo":"x y&? z"}"#).unwrap();
        let expected = GraphQLRequest(GraphQLBatchRequest::Single(http::GraphQLRequest::new(
            "test".to_string(),
            None,
            Some(variables),
        )));
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_url_decode() {
        let form_string = "query=%25foo%20bar+baz%26%3F&operation_name=test";
        let mut items = FormItems::from(form_string);
        let result: Result<GraphQLRequest, _> = GraphQLRequest::from_form(&mut items, false);
        assert!(result.is_ok());
        let expected = GraphQLRequest(GraphQLBatchRequest::Single(http::GraphQLRequest::new(
            "%foo bar baz&?".to_string(),
            Some("test".to_string()),
            None,
        )));
        assert_eq!(result.unwrap(), expected);
    }
}

#[cfg(test)]
mod tests {
    use juniper::{
        http::tests as http_tests,
        tests::fixtures::starwars::schema::{Database, Query},
        EmptyMutation, EmptySubscription, RootNode,
    };
    use rocket::{
        self, get,
        http::ContentType,
        local::{Client, LocalRequest},
        post,
        request::Form,
        routes, Rocket, State,
    };

    type Schema = RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;

    #[get("/?<request..>")]
    fn get_graphql_handler(
        context: State<Database>,
        request: Form<super::GraphQLRequest>,
        schema: State<Schema>,
    ) -> super::GraphQLResponse {
        request.execute_sync(&schema, &context)
    }

    #[post("/", data = "<request>")]
    fn post_graphql_handler(
        context: State<Database>,
        request: super::GraphQLRequest,
        schema: State<Schema>,
    ) -> super::GraphQLResponse {
        request.execute_sync(&schema, &context)
    }

    struct TestRocketIntegration {
        client: Client,
    }

    impl http_tests::HttpIntegration for TestRocketIntegration {
        fn get(&self, url: &str) -> http_tests::TestResponse {
            let req = &self.client.get(url);
            make_test_response(req)
        }

        fn post_json(&self, url: &str, body: &str) -> http_tests::TestResponse {
            let req = &self.client.post(url).header(ContentType::JSON).body(body);
            make_test_response(req)
        }

        fn post_graphql(&self, url: &str, body: &str) -> http_tests::TestResponse {
            let req = &self
                .client
                .post(url)
                .header(ContentType::new("application", "graphql"))
                .body(body);
            make_test_response(req)
        }
    }

    #[test]
    fn test_rocket_integration() {
        let rocket = make_rocket();
        let client = Client::new(rocket).expect("valid rocket");
        let integration = TestRocketIntegration { client };

        http_tests::run_http_test_suite(&integration);
    }

    #[test]
    fn test_operation_names() {
        #[post("/", data = "<request>")]
        fn post_graphql_assert_operation_name_handler(
            context: State<Database>,
            request: super::GraphQLRequest,
            schema: State<Schema>,
        ) -> super::GraphQLResponse {
            assert_eq!(request.operation_names(), vec![Some("TestQuery")]);
            request.execute_sync(&schema, &context)
        }

        let rocket = make_rocket_without_routes()
            .mount("/", routes![post_graphql_assert_operation_name_handler]);
        let client = Client::new(rocket).expect("valid rocket");

        let req = client
            .post("/")
            .header(ContentType::JSON)
            .body(r#"{"query": "query TestQuery {hero{name}}", "operationName": "TestQuery"}"#);
        let resp = make_test_response(&req);

        assert_eq!(resp.status_code, 200);
    }

    fn make_rocket() -> Rocket {
        make_rocket_without_routes().mount("/", routes![post_graphql_handler, get_graphql_handler])
    }

    fn make_rocket_without_routes() -> Rocket {
        rocket::ignite().manage(Database::new()).manage(Schema::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        ))
    }

    fn make_test_response(request: &LocalRequest) -> http_tests::TestResponse {
        let mut response = request.clone().dispatch();
        let status_code = response.status().code as i32;
        let content_type = response
            .content_type()
            .expect("No content type header from handler")
            .to_string();
        let body = response
            .body()
            .expect("No body returned from GraphQL handler")
            .into_string();

        http_tests::TestResponse {
            status_code,
            body,
            content_type,
        }
    }
}
