/*!

# juniper_rocket_async

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
[documentation]: https://docs.rs/juniper_rocket_async
[example]: https://github.com/graphql-rust/juniper_rocket_async/blob/master/examples/rocket_server.rs

*/

#![doc(html_root_url = "https://docs.rs/juniper_rocket_async/0.2.0")]

use std::{borrow::Cow, io::Cursor};

use rocket::{
    data::{self, FromData, ToByteUnit},
    form::{error::ErrorKind, DataField, Error, Errors, FromForm, Options, ValueField},
    http::{ContentType, Status},
    outcome::Outcome::{Failure, Forward, Success},
    response::{self, content, Responder, Response},
    Data, Request,
};

use juniper::{
    http::{self, GraphQLBatchRequest},
    DefaultScalarValue, FieldError, GraphQLSubscriptionType, GraphQLType, GraphQLTypeAsync,
    InputValue, RootNode, ScalarValue,
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
    subscriptions_endpoint_url: Option<&str>,
) -> content::Html<String> {
    content::Html(juniper::http::graphiql::graphiql_source(
        graphql_endpoint_url,
        subscriptions_endpoint_url,
    ))
}

/// Generate an HTML page containing GraphQL Playground
pub fn playground_source(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: Option<&str>,
) -> content::Html<String> {
    content::Html(juniper::http::playground::playground_source(
        graphql_endpoint_url,
        subscriptions_endpoint_url,
    ))
}

impl<S> GraphQLRequest<S>
where
    S: ScalarValue,
{
    /// Synchronously execute an incoming GraphQL query.
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

    /// Asynchronously execute an incoming GraphQL query.
    pub async fn execute<CtxT, QueryT, MutationT, SubscriptionT>(
        &self,
        root_node: &RootNode<'_, QueryT, MutationT, SubscriptionT, S>,
        context: &CtxT,
    ) -> GraphQLResponse
    where
        QueryT: GraphQLTypeAsync<S, Context = CtxT>,
        QueryT::TypeInfo: Sync,
        MutationT: GraphQLTypeAsync<S, Context = CtxT>,
        MutationT::TypeInfo: Sync,
        SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT>,
        SubscriptionT::TypeInfo: Sync,
        CtxT: Sync,
        S: Send + Sync,
    {
        let response = self.0.execute(root_node, context).await;
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
    /// # use rocket::http::CookieJar;
    /// # use rocket::form::Form;
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
    ///     cookies: &CookieJar,
    ///     context: &State<Database>,
    ///     request: juniper_rocket_async::GraphQLRequest,
    ///     schema: &State<Schema>,
    /// ) -> juniper_rocket_async::GraphQLResponse {
    ///     if cookies.get("user_id").is_none() {
    ///         let err = FieldError::new("User is not logged in", Value::null());
    ///         return juniper_rocket_async::GraphQLResponse::error(err);
    ///     }
    ///
    ///     request.execute_sync(&*schema, &*context)
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

pub struct GraphQLContext<'f, S: ScalarValue> {
    opts: Options,
    query: Option<String>,
    operation_name: Option<String>,
    variables: Option<InputValue<S>>,
    errors: Errors<'f>,
}

impl<'f, S: ScalarValue> GraphQLContext<'f, S> {
    fn query(&mut self, value: String) {
        if self.query.is_some() {
            let error = Error::from(ErrorKind::Duplicate).with_name("query");

            self.errors.push(error)
        } else {
            println!("{}", value);
            self.query = Some(value);
        }
    }

    fn operation_name(&mut self, value: String) {
        if self.operation_name.is_some() {
            let error = Error::from(ErrorKind::Duplicate).with_name("operation_name");

            self.errors.push(error)
        } else {
            self.operation_name = Some(value);
        }
    }

    fn variables(&mut self, value: String) {
        if self.variables.is_some() {
            let error = Error::from(ErrorKind::Duplicate).with_name("variables");

            self.errors.push(error)
        } else {
            let parse_result = serde_json::from_str::<InputValue<S>>(&value);

            match parse_result {
                Ok(variables) => self.variables = Some(variables),
                Err(e) => {
                    let error = Error::from(ErrorKind::Validation(Cow::Owned(e.to_string())))
                        .with_name("variables");

                    self.errors.push(error);
                }
            }
        }
    }
}

#[rocket::async_trait]
impl<'f, S> FromForm<'f> for GraphQLRequest<S>
where
    S: ScalarValue + Send,
{
    type Context = GraphQLContext<'f, S>;

    fn init(opts: Options) -> Self::Context {
        GraphQLContext {
            opts,
            query: None,
            operation_name: None,
            variables: None,
            errors: Errors::new(),
        }
    }

    fn push_value(ctx: &mut Self::Context, field: ValueField<'f>) {
        match field.name.key().map(|key| key.as_str()) {
            Some("query") => ctx.query(field.value.to_owned()),
            Some("operation_name") => ctx.operation_name(field.value.to_owned()),
            Some("variables") => ctx.variables(field.value.to_owned()),
            Some(key) => {
                if ctx.opts.strict {
                    let error = Error::from(ErrorKind::Unknown).with_name(key);

                    ctx.errors.push(error)
                }
            }
            None => {
                if ctx.opts.strict {
                    let error = Error::from(ErrorKind::Unexpected);

                    ctx.errors.push(error)
                }
            }
        }
    }

    async fn push_data(ctx: &mut Self::Context, field: DataField<'f, '_>) {
        if ctx.opts.strict {
            let error = Error::from(ErrorKind::Unexpected).with_name(field.name);

            ctx.errors.push(error)
        }
    }

    fn finalize(mut ctx: Self::Context) -> rocket::form::Result<'f, Self> {
        if ctx.query.is_none() {
            let error = Error::from(ErrorKind::Missing).with_name("query");

            ctx.errors.push(error)
        }

        match ctx.errors.is_empty() {
            true => Ok(GraphQLRequest(GraphQLBatchRequest::Single(
                http::GraphQLRequest::new(ctx.query.unwrap(), ctx.operation_name, ctx.variables),
            ))),
            false => Err(ctx.errors),
        }
    }
}

const BODY_LIMIT: u64 = 1024 * 100;

#[rocket::async_trait]
impl<'r, S> FromData<'r> for GraphQLRequest<S>
where
    S: ScalarValue,
{
    type Error = String;

    async fn from_data(
        req: &'r Request<'_>,
        data: Data<'r>,
    ) -> data::Outcome<'r, Self, Self::Error> {
        use rocket::tokio::io::AsyncReadExt as _;

        let content_type = req
            .content_type()
            .map(|ct| (ct.top().as_str(), ct.sub().as_str()));
        let is_json = match content_type {
            Some(("application", "json")) => true,
            Some(("application", "graphql")) => false,
            _ => return Box::pin(async move { Forward(data) }).await,
        };

        Box::pin(async move {
            let mut body = String::new();
            let mut reader = data.open(BODY_LIMIT.bytes());
            if let Err(e) = reader.read_to_string(&mut body).await {
                return Failure((Status::InternalServerError, format!("{:?}", e)));
            }

            Success(GraphQLRequest(if is_json {
                match serde_json::from_str(&body) {
                    Ok(req) => req,
                    Err(e) => return Failure((Status::BadRequest, format!("{}", e))),
                }
            } else {
                GraphQLBatchRequest::Single(http::GraphQLRequest::new(body, None, None))
            }))
        })
        .await
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for GraphQLResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'o> {
        let GraphQLResponse(status, body) = self;

        Response::build()
            .header(ContentType::new("application", "json"))
            .status(status)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}

#[cfg(test)]
mod fromform_tests {
    use super::*;
    use juniper::InputValue;
    use rocket::{
        form::{error::ErrorKind, Error, Form, Strict},
        http::RawStr,
    };
    use std::borrow::Cow;

    fn check_error(input: &str, expected_errors: Vec<Error>, strict: bool) {
        let errors = if strict {
            let result = Form::<Strict<GraphQLRequest>>::parse_encoded(RawStr::new(input));
            assert!(result.is_err());
            result.unwrap_err()
        } else {
            let result = Form::<GraphQLRequest>::parse_encoded(RawStr::new(input));
            assert!(result.is_err());
            result.unwrap_err()
        };
        assert_eq!(errors.len(), expected_errors.len());
        for (error, expected) in errors.iter().zip(&expected_errors) {
            match (&error.kind, &expected.kind) {
                (ErrorKind::Unknown, ErrorKind::Unknown) => (),
                (kind_a, kind_b) => assert_eq!(kind_a, kind_b),
            };
            assert_eq!(error.name, expected.name);
            assert_eq!(error.value, expected.value);
            assert_eq!(error.entity, expected.entity);
        }
    }

    #[test]
    fn test_empty_form() {
        check_error(
            "",
            vec![Error::from(ErrorKind::Missing).with_name("query")],
            false,
        );
    }

    #[test]
    fn test_no_query() {
        check_error(
            "operation_name=foo&variables={}",
            vec![Error::from(ErrorKind::Missing).with_name("query")],
            false,
        );
    }

    #[test]
    fn test_strict() {
        check_error(
            "query=test&foo=bar",
            vec![Error::from(ErrorKind::Unknown).with_name("foo")],
            true,
        );
    }

    #[test]
    fn test_duplicate_query() {
        check_error(
            "query=foo&query=bar",
            vec![Error::from(ErrorKind::Duplicate).with_name("query")],
            false,
        );
    }

    #[test]
    fn test_duplicate_operation_name() {
        check_error(
            "query=test&operation_name=op1&operation_name=op2",
            vec![Error::from(ErrorKind::Duplicate).with_name("operation_name")],
            false,
        );
    }

    #[test]
    fn test_duplicate_variables() {
        check_error(
            "query=test&variables={}&variables={}",
            vec![Error::from(ErrorKind::Duplicate).with_name("variables")],
            false,
        );
    }

    #[test]
    fn test_variables_invalid_json() {
        check_error(
            "query=test&variables=NOT_JSON",
            vec![Error::from(ErrorKind::Validation(Cow::Owned(
                "expected value at line 1 column 1".to_owned(),
            )))
            .with_name("variables")],
            false,
        );
    }

    #[test]
    fn test_variables_valid_json() {
        let result: Result<GraphQLRequest, Errors> =
            Form::parse_encoded(RawStr::new(r#"query=test&variables={"foo":"bar"}"#));
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
        let result: Result<GraphQLRequest, Errors> = Form::parse_encoded(RawStr::new(
            r#"query=test&variables={"foo":"x%20y%26%3F+z"}"#,
        ));
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
        let result: Result<GraphQLRequest, Errors> = Form::parse_encoded(RawStr::new(
            "query=%25foo%20bar+baz%26%3F&operation_name=test",
        ));
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

    use futures;

    use juniper::{
        http::tests as http_tests,
        tests::fixtures::starwars::schema::{Database, Query},
        EmptyMutation, EmptySubscription, RootNode,
    };
    use rocket::{
        self, get,
        http::ContentType,
        local::asynchronous::{Client, LocalResponse},
        post, routes, Build, Rocket, State,
    };

    type Schema = RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;

    #[get("/?<request..>")]
    fn get_graphql_handler(
        context: &State<Database>,
        request: super::GraphQLRequest,
        schema: &State<Schema>,
    ) -> super::GraphQLResponse {
        request.execute_sync(&*schema, &*context)
    }

    #[post("/", data = "<request>")]
    fn post_graphql_handler(
        context: &State<Database>,
        request: super::GraphQLRequest,
        schema: &State<Schema>,
    ) -> super::GraphQLResponse {
        request.execute_sync(&*schema, &*context)
    }

    struct TestRocketIntegration {
        client: Client,
    }

    impl http_tests::HttpIntegration for TestRocketIntegration {
        fn get(&self, url: &str) -> http_tests::TestResponse {
            let req = self.client.get(url);
            let req = futures::executor::block_on(req.dispatch());
            futures::executor::block_on(make_test_response(req))
        }

        fn post_json(&self, url: &str, body: &str) -> http_tests::TestResponse {
            let req = self.client.post(url).header(ContentType::JSON).body(body);
            let req = futures::executor::block_on(req.dispatch());
            futures::executor::block_on(make_test_response(req))
        }

        fn post_graphql(&self, url: &str, body: &str) -> http_tests::TestResponse {
            let req = self
                .client
                .post(url)
                .header(ContentType::new("application", "graphql"))
                .body(body);
            let req = futures::executor::block_on(req.dispatch());
            futures::executor::block_on(make_test_response(req))
        }
    }

    #[rocket::async_test]
    async fn test_rocket_integration() {
        let rocket = make_rocket();
        let client = Client::untracked(rocket).await.expect("valid rocket");
        let integration = TestRocketIntegration { client };

        http_tests::run_http_test_suite(&integration);
    }

    #[rocket::async_test]
    async fn test_operation_names() {
        #[post("/", data = "<request>")]
        fn post_graphql_assert_operation_name_handler(
            context: &State<Database>,
            request: super::GraphQLRequest,
            schema: &State<Schema>,
        ) -> super::GraphQLResponse {
            assert_eq!(request.operation_names(), vec![Some("TestQuery")]);
            request.execute_sync(&*schema, &*context)
        }

        let rocket = make_rocket_without_routes()
            .mount("/", routes![post_graphql_assert_operation_name_handler]);
        let client = Client::untracked(rocket).await.expect("valid rocket");

        let resp = client
            .post("/")
            .header(ContentType::JSON)
            .body(r#"{"query": "query TestQuery {hero{name}}", "operationName": "TestQuery"}"#)
            .dispatch()
            .await;
        let resp = make_test_response(resp);

        assert_eq!(resp.await.status_code, 200);
    }

    fn make_rocket() -> Rocket<Build> {
        make_rocket_without_routes().mount("/", routes![post_graphql_handler, get_graphql_handler])
    }

    fn make_rocket_without_routes() -> Rocket<Build> {
        Rocket::build().manage(Database::new()).manage(Schema::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        ))
    }

    async fn make_test_response(response: LocalResponse<'_>) -> http_tests::TestResponse {
        let status_code = response.status().code as i32;
        let content_type = response
            .content_type()
            .expect("No content type header from handler")
            .to_string();
        let body = response
            .into_string()
            .await
            .expect("No body returned from GraphQL handler");

        http_tests::TestResponse {
            status_code,
            body: Some(body),
            content_type,
        }
    }
}
