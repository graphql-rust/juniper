#![doc = include_str!("../README.md")]

use std::{borrow::Cow, io::Cursor};

use rocket::{
    data::{self, FromData, ToByteUnit},
    form::{error::ErrorKind, DataField, Error, Errors, FromForm, Options, ValueField},
    http::{ContentType, Status},
    outcome::Outcome,
    response::{self, content::RawHtml, Responder, Response},
    Data, Request,
};

use juniper::{
    http::{self, GraphQLBatchRequest},
    DefaultScalarValue, FieldError, GraphQLSubscriptionType, GraphQLType, GraphQLTypeAsync,
    InputValue, RootNode, ScalarValue,
};

/// Simple wrapper around an incoming GraphQL request.
///
/// See the [`http`] module for more information. This type can be constructed automatically from
/// both GET and POST routes, as implements [`FromForm`] and [`FromData`] traits.
///
/// # Example
///
/// ```rust
/// use juniper::{
///     tests::fixtures::starwars::schema::{Database, Query},
///     EmptyMutation, EmptySubscription, RootNode,
/// };
/// use rocket::{routes, State};
///
/// type Schema = RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;
///
/// // GET request accepts query parameters like these:
/// // ?query=<urlencoded-graphql-query-string>
/// // &operationName=<optional-name>
/// // &variables=<optional-json-encoded-variables>
/// // See details here: https://graphql.org/learn/serving-over-http#get-request
/// #[rocket::get("/graphql?<request..>")]
/// async fn get_graphql_handler(
///     db: &State<Database>,
///     request: juniper_rocket::GraphQLRequest,
///     schema: &State<Schema>,
/// ) -> juniper_rocket::GraphQLResponse {
///     request.execute(schema, db).await
/// }
///
/// #[rocket::post("/graphql", data = "<request>")]
/// async fn post_graphql_handler(
///     db: &State<Database>,
///     request: juniper_rocket::GraphQLRequest,
///     schema: &State<Schema>,
/// ) -> juniper_rocket::GraphQLResponse {
///     request.execute(schema, db).await
/// }
///
/// let rocket = rocket::build()
///     .manage(Database::new())
///     .manage(Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()))
///     .mount("/", routes![get_graphql_handler, post_graphql_handler]);
/// ```
#[derive(Debug, PartialEq)]
pub struct GraphQLRequest<S = DefaultScalarValue>(GraphQLBatchRequest<S>)
where
    S: ScalarValue;

impl<S: ScalarValue> AsRef<GraphQLBatchRequest<S>> for GraphQLRequest<S> {
    fn as_ref(&self) -> &GraphQLBatchRequest<S> {
        &self.0
    }
}

impl<S: ScalarValue> AsMut<GraphQLBatchRequest<S>> for GraphQLRequest<S> {
    fn as_mut(&mut self) -> &mut GraphQLBatchRequest<S> {
        &mut self.0
    }
}

/// Simple wrapper around the result of executing a GraphQL query
pub struct GraphQLResponse(pub Status, pub String);

/// Generates a [`RawHtml`] page containing [GraphiQL].
///
/// This does not handle routing, so you can mount it on any endpoint.
///
/// # Example
///
/// ```rust
/// use rocket::{response::content::RawHtml, routes};
///
/// #[rocket::get("/graphiql")]
/// fn graphiql() -> RawHtml<String> {
///     juniper_rocket::graphiql_source("/graphql", "/subscriptions")
/// }
///
/// let rocket = rocket::build().mount("/", routes![graphiql]);
/// ```
///
/// [GraphiQL]: https://github.com/graphql/graphiql
pub fn graphiql_source<'a>(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: impl Into<Option<&'a str>>,
) -> RawHtml<String> {
    RawHtml(http::graphiql::graphiql_source(
        graphql_endpoint_url,
        subscriptions_endpoint_url.into(),
    ))
}

/// Generates a [`RawHtml`] page containing [GraphQL Playground].
///
/// This does not handle routing, so you can mount it on any endpoint.
///
/// # Example
///
/// ```rust
/// use rocket::{response::content::RawHtml, routes};
///
/// #[rocket::get("/playground")]
/// fn playground() -> RawHtml<String> {
///     juniper_rocket::playground_source("/graphql", "/subscriptions")
/// }
///
/// let rocket = rocket::build().mount("/", routes![playground]);
/// ```
///
/// [GraphQL Playground]: https://github.com/prisma/graphql-playground
pub fn playground_source<'a>(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: impl Into<Option<&'a str>>,
) -> RawHtml<String> {
    RawHtml(http::playground::playground_source(
        graphql_endpoint_url,
        subscriptions_endpoint_url.into(),
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
    /// Constructs an error response outside of the normal execution flow.
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
    ///     request: juniper_rocket::GraphQLRequest,
    ///     schema: &State<Schema>,
    /// ) -> juniper_rocket::GraphQLResponse {
    ///     if cookies.get("user_id").is_none() {
    ///         let err = FieldError::new("User is not logged in", Value::null());
    ///         return juniper_rocket::GraphQLResponse::error(err);
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
            self.query = Some(value);
        }
    }

    fn operation_name(&mut self, value: String) {
        if self.operation_name.is_some() {
            let error = Error::from(ErrorKind::Duplicate).with_name("operationName");

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
            Some("query") => ctx.query(field.value.into()),
            Some("operation_name" | "operationName") => ctx.operation_name(field.value.into()),
            Some("variables") => ctx.variables(field.value.into()),
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
            _ => return Outcome::Forward((data, Status::UnsupportedMediaType)),
        };

        Box::pin(async move {
            let limit = req
                .limits()
                .get("graphql")
                .unwrap_or_else(|| BODY_LIMIT.bytes());
            let mut reader = data.open(limit);
            let mut body = String::new();
            if let Err(e) = reader.read_to_string(&mut body).await {
                return Outcome::Error((Status::InternalServerError, format!("{e:?}")));
            }

            Outcome::Success(GraphQLRequest(if is_json {
                match serde_json::from_str(&body) {
                    Ok(req) => req,
                    Err(e) => return Outcome::Error((Status::BadRequest, e.to_string())),
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
    use std::borrow::Cow;

    use juniper::{http, InputValue};
    use rocket::{
        form::{error::ErrorKind, Error, Errors, Form, Strict},
        http::RawStr,
    };

    use super::GraphQLRequest;

    fn check_error(input: &str, expected_errors: Vec<Error>, strict: bool) {
        let errors = if strict {
            let res = Form::<Strict<GraphQLRequest>>::parse_encoded(RawStr::new(input));

            assert!(res.is_err(), "result: {:#?}", res.unwrap());

            res.unwrap_err()
        } else {
            let res = Form::<GraphQLRequest>::parse_encoded(RawStr::new(input));

            assert!(res.is_err(), "result: {:#?}", res.unwrap());

            res.unwrap_err()
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
    fn empty_form() {
        check_error(
            "",
            vec![Error::from(ErrorKind::Missing).with_name("query")],
            false,
        );
    }

    #[test]
    fn no_query() {
        check_error(
            "operation_name=foo&variables={}",
            vec![Error::from(ErrorKind::Missing).with_name("query")],
            false,
        );
    }

    #[test]
    fn strict() {
        check_error(
            "query=test&foo=bar",
            vec![Error::from(ErrorKind::Unknown).with_name("foo")],
            true,
        );
    }

    #[test]
    fn duplicate_query() {
        check_error(
            "query=foo&query=bar",
            vec![Error::from(ErrorKind::Duplicate).with_name("query")],
            false,
        );
    }

    #[test]
    fn duplicate_operation_name() {
        check_error(
            "query=test&operationName=op1&operationName=op2",
            vec![Error::from(ErrorKind::Duplicate).with_name("operationName")],
            false,
        );
    }

    #[test]
    fn duplicate_variables() {
        check_error(
            "query=test&variables={}&variables={}",
            vec![Error::from(ErrorKind::Duplicate).with_name("variables")],
            false,
        );
    }

    #[test]
    fn variables_invalid_json() {
        check_error(
            "query=test&variables=NOT_JSON",
            vec![Error::from(ErrorKind::Validation(Cow::Owned(
                "expected value at line 1 column 1".into(),
            )))
            .with_name("variables")],
            false,
        );
    }

    #[test]
    fn variables_valid_json() {
        let result: Result<GraphQLRequest, Errors> =
            Form::parse_encoded(RawStr::new(r#"query=test&variables={"foo":"bar"}"#));

        assert!(result.is_ok());

        let variables = ::serde_json::from_str::<InputValue>(r#"{"foo":"bar"}"#).unwrap();
        let expected = GraphQLRequest(http::GraphQLBatchRequest::Single(
            http::GraphQLRequest::new("test".into(), None, Some(variables)),
        ));

        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn variables_encoded_json() {
        let result: Result<GraphQLRequest, Errors> = Form::parse_encoded(RawStr::new(
            r#"query=test&variables={"foo":"x%20y%26%3F+z"}"#,
        ));
        let variables = ::serde_json::from_str::<InputValue>(r#"{"foo":"x y&? z"}"#).unwrap();
        let expected = GraphQLRequest(http::GraphQLBatchRequest::Single(
            http::GraphQLRequest::new("test".into(), None, Some(variables)),
        ));

        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn url_decode() {
        let result: Result<GraphQLRequest, Errors> = Form::parse_encoded(RawStr::new(
            "query=%25foo%20bar+baz%26%3F&operationName=test",
        ));

        assert!(result.is_ok());

        let expected = GraphQLRequest(http::GraphQLBatchRequest::Single(
            http::GraphQLRequest::new("%foo bar baz&?".into(), Some("test".into()), None),
        ));

        assert_eq!(result.unwrap(), expected);
    }
}
