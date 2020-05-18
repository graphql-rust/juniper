//! Utilities for building HTTP endpoints in a library-agnostic manner

pub mod graphiql;
pub mod playground;

use serde::{
    de,
    ser::{self, SerializeMap},
    Deserialize, Serialize,
};

use crate::{
    ast::InputValue,
    executor::{ExecutionError, ValuesStream},
    value::{DefaultScalarValue, ScalarValue},
    FieldError, GraphQLError, GraphQLSubscriptionType, GraphQLType, RootNode, Value, Variables,
};

/// The expected structure of the decoded JSON document for either POST or GET requests.
///
/// For POST, you can use Serde to deserialize the incoming JSON data directly
/// into this struct - it derives Deserialize for exactly this reason.
///
/// For GET, you will need to parse the query string and extract "query",
/// "operationName", and "variables" manually.
#[derive(Deserialize, Clone, Serialize, PartialEq, Debug)]
pub struct GraphQLRequest<S = DefaultScalarValue>
where
    S: ScalarValue,
{
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    #[serde(bound(deserialize = "InputValue<S>: Deserialize<'de> + Serialize"))]
    variables: Option<InputValue<S>>,
}

impl<S> GraphQLRequest<S>
where
    S: ScalarValue,
{
    /// Returns the `operation_name` associated with this request.
    pub fn operation_name(&self) -> Option<&str> {
        self.operation_name.as_ref().map(|oper_name| &**oper_name)
    }

    fn variables(&self) -> Variables<S> {
        self.variables
            .as_ref()
            .and_then(|iv| {
                iv.to_object_value().map(|o| {
                    o.into_iter()
                        .map(|(k, v)| (k.to_owned(), v.clone()))
                        .collect()
                })
            })
            .unwrap_or_default()
    }

    /// Construct a new GraphQL request from parts
    pub fn new(
        query: String,
        operation_name: Option<String>,
        variables: Option<InputValue<S>>,
    ) -> Self {
        GraphQLRequest {
            query,
            operation_name,
            variables,
        }
    }

    /// Execute a GraphQL request using the specified schema and context
    ///
    /// This is a simple wrapper around the `execute` function exposed at the
    /// top level of this crate.
    pub async fn execute<'a, CtxT, QueryT, MutationT, SubscriptionT>(
        &'a self,
        root_node: &'a RootNode<'a, QueryT, MutationT, SubscriptionT, S>,
        context: &'a CtxT,
    ) -> GraphQLResponse<'a, S>
    where
        S: ScalarValue,
        QueryT: crate::GraphQLType<S, Context = CtxT>,
        QueryT::TypeInfo: Send + Sync,
        MutationT: crate::GraphQLType<S, Context = CtxT>,
        MutationT::TypeInfo: Send + Sync,
        SubscriptionT: GraphQLType<S, Context = CtxT>,
        SubscriptionT::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
    {
        let op = self.operation_name();
        let vars = &self.variables();
        let res = crate::execute(&self.query, op, root_node, vars, context).await;
        GraphQLResponse(res)
    }
}

/// Resolve a GraphQL subscription into `Value<ValuesStream<S>` using the
/// specified schema and context.
/// This is a wrapper around the `resolve_into_stream` function exposed at the top
/// level of this crate.
pub async fn resolve_into_stream<'req, 'rn, 'ctx, 'a, CtxT, QueryT, MutationT, SubscriptionT, S>(
    req: &'req GraphQLRequest<S>,
    root_node: &'rn RootNode<'a, QueryT, MutationT, SubscriptionT, S>,
    context: &'ctx CtxT,
) -> Result<(Value<ValuesStream<'a, S>>, Vec<ExecutionError<S>>), GraphQLError<'a>>
where
    'req: 'a,
    'rn: 'a,
    'ctx: 'a,
    S: ScalarValue,
    QueryT: GraphQLType<S, Context = CtxT>,
    QueryT::TypeInfo: Send + Sync,
    MutationT: GraphQLType<S, Context = CtxT>,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT>,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Send + Sync,
{
    let op = req.operation_name();
    let vars = req.variables();

    crate::resolve_into_stream(&req.query, op, root_node, &vars, context).await
}

/// Simple wrapper around the result from executing a GraphQL query
///
/// This struct implements Serialize, so you can simply serialize this
/// to JSON and send it over the wire. Use the `is_ok` method to determine
/// whether to send a 200 or 400 HTTP status code.
#[derive(Debug)]
pub struct GraphQLResponse<'a, S = DefaultScalarValue>(
    Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError<'a>>,
);

impl<'a, S> GraphQLResponse<'a, S>
where
    S: ScalarValue,
{
    /// Constructs new `GraphQLResponse` using the given result
    pub fn from_result(r: Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError<'a>>) -> Self {
        Self(r)
    }

    /// Constructs an error response outside of the normal execution flow
    pub fn error(error: FieldError<S>) -> Self {
        GraphQLResponse(Ok((Value::null(), vec![ExecutionError::at_origin(error)])))
    }

    /// Was the request successful or not?
    ///
    /// Note that there still might be errors in the response even though it's
    /// considered OK. This is by design in GraphQL.
    pub fn is_ok(&self) -> bool {
        self.0.is_ok()
    }
}

impl<'a, T> Serialize for GraphQLResponse<'a, T>
where
    T: Serialize + ScalarValue,
    Value<T>: Serialize,
    ExecutionError<T>: Serialize,
    GraphQLError<'a>: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self.0 {
            Ok((ref res, ref err)) => {
                let mut map = serializer.serialize_map(None)?;

                map.serialize_key("data")?;
                map.serialize_value(res)?;

                if !err.is_empty() {
                    map.serialize_key("errors")?;
                    map.serialize_value(err)?;
                }

                map.end()
            }
            Err(ref err) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_key("errors")?;
                map.serialize_value(err)?;
                map.end()
            }
        }
    }
}

/// Simple wrapper around GraphQLRequest to allow the handling of Batch requests.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
#[serde(bound = "InputValue<S>: Deserialize<'de>")]
pub enum GraphQLBatchRequest<S = DefaultScalarValue>
where
    S: ScalarValue,
{
    /// A single operation request.
    Single(GraphQLRequest<S>),

    /// A batch operation request.
    ///
    /// Empty batch is considered as invalid value, so cannot be deserialized.
    #[serde(deserialize_with = "deserialize_non_empty_vec")]
    Batch(Vec<GraphQLRequest<S>>),
}

fn deserialize_non_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: de::Deserializer<'de>,
    T: Deserialize<'de>,
{
    use de::Error as _;

    let v = Vec::<T>::deserialize(deserializer)?;
    if v.is_empty() {
        Err(D::Error::invalid_length(0, &"a positive integer"))
    } else {
        Ok(v)
    }
}

impl<S> GraphQLBatchRequest<S>
where
    S: ScalarValue,
{
    /// Executes a GraphQL request using the specified schema and context
    ///
    /// This is a simple wrapper around the `execute` function exposed in
    /// GraphQLRequest
    pub async fn execute<'a, CtxT, QueryT, MutationT, SubscriptionT>(
        &'a self,
        root_node: &'a crate::RootNode<'a, QueryT, MutationT, SubscriptionT, S>,
        context: &'a CtxT,
    ) -> GraphQLBatchResponse<'a, S>
    where
        QueryT: crate::GraphQLType<S, Context = CtxT>,
        QueryT::TypeInfo: Send + Sync,
        MutationT: crate::GraphQLType<S, Context = CtxT>,
        MutationT::TypeInfo: Send + Sync,
        SubscriptionT: crate::GraphQLSubscriptionType<S, Context = CtxT>,
        SubscriptionT::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
    {
        match *self {
            Self::Single(ref req) => {
                let resp = req.execute(root_node, context).await;
                GraphQLBatchResponse::Single(resp)
            }
            Self::Batch(ref reqs) => {
                let resps = futures::future::join_all(
                    reqs.iter().map(|req| req.execute(root_node, context)),
                )
                .await;
                GraphQLBatchResponse::Batch(resps)
            }
        }
    }

    /// The operation names of the request.
    pub fn operation_names(&self) -> Vec<Option<&str>> {
        match self {
            Self::Single(req) => vec![req.operation_name()],
            Self::Batch(reqs) => reqs.iter().map(|req| req.operation_name()).collect(),
        }
    }
}

/// Simple wrapper around the result (GraphQLResponse) from executing a GraphQLBatchRequest
///
/// This struct implements Serialize, so you can simply serialize this
/// to JSON and send it over the wire. use the `is_ok` to determine
/// wheter to send a 200 or 400 HTTP status code.
#[derive(Serialize)]
#[serde(untagged)]
pub enum GraphQLBatchResponse<'a, S = DefaultScalarValue>
where
    S: ScalarValue,
{
    /// Result of a single operation in a GraphQL request.
    Single(GraphQLResponse<'a, S>),
    /// Result of a batch operation in a GraphQL request.
    Batch(Vec<GraphQLResponse<'a, S>>),
}

impl<'a, S> GraphQLBatchResponse<'a, S>
where
    S: ScalarValue,
{
    /// Returns if all the GraphQLResponse in this operation are ok,
    /// you can use it to determine wheter to send a 200 or 400 HTTP status code.
    pub fn is_ok(&self) -> bool {
        match self {
            Self::Single(resp) => resp.is_ok(),
            Self::Batch(resps) => resps.iter().all(GraphQLResponse::is_ok),
        }
    }
}

#[cfg(any(test, feature = "expose-test-schema"))]
#[allow(missing_docs)]
pub mod tests {
    use crate::BoxFuture;
    use serde_json::{self, Value as Json};

    /// Normalized response content we expect to get back from
    /// the http framework integration we are testing.
    #[derive(Debug)]
    pub struct TestResponse {
        pub status_code: i32,
        pub body: Option<String>,
        pub content_type: String,
    }

    /// Normalized way to make requests to the HTTP framework integration we are testing.
    pub trait HttpIntegration {
        /// Sends GET HTTP request to this integration with the provided `url` parameters string,
        /// and returns response returned by this integration.
        fn get<'me, 'url, 'fut>(&'me self, url: &'url str) -> BoxFuture<'fut, TestResponse>
        where
            'me: 'fut,
            'url: 'fut;

        /// Sends POST HTTP request to this integration with the provided JSON-encoded `body`, and
        /// returns response returned by this integration.
        fn post_json<'me, 'url, 'body, 'fut>(
            &'me self,
            url: &'url str,
            body: &'body str,
        ) -> BoxFuture<'fut, TestResponse>
        where
            'me: 'fut,
            'url: 'fut,
            'body: 'fut;
        /// Sends POST HTTP request to this integration with the provided raw GraphQL query as
        /// `body`, and returns response returned by this integration.
        fn post_graphql<'me, 'url, 'body, 'fut>(
            &'me self,
            url: &'url str,
            body: &'body str,
        ) -> BoxFuture<'fut, TestResponse>
        where
            'me: 'fut,
            'url: 'fut,
            'body: 'fut;
    }

    /// Runs the fixed test suite for a HTTP implementation.
    pub async fn run_http_test_suite<T: HttpIntegration>(integration: &T) {
        println!("Running HTTP Test suite for integration");

        println!("  - test_simple_get");
        test_simple_get(integration).await;

        println!("  - test_encoded_get");
        test_encoded_get(integration).await;

        println!("  - test_get_with_variables");
        test_get_with_variables(integration).await;

        println!("  - test_simple_post");
        test_simple_post(integration).await;

        println!("  - test_batched_post");
        test_batched_post(integration).await;

        println!("  - test_empty_batched_post");
        test_empty_batched_post(integration).await;

        println!("  - test_invalid_json");
        test_invalid_json(integration).await;

        println!("  - test_invalid_field");
        test_invalid_field(integration).await;

        println!("  - test_duplicate_keys");
        test_duplicate_keys(integration).await;

        println!("  - test_graphql_post");
        test_graphql_post(integration).await;

        println!("  - test_invalid_graphql_post");
        test_invalid_graphql_post(integration).await;
    }

    fn unwrap_json_response(response: &TestResponse) -> Json {
        serde_json::from_str::<Json>(
            response
                .body
                .as_ref()
                .expect("No data returned from request"),
        )
        .expect("Could not parse JSON object")
    }

    async fn test_simple_get<T: HttpIntegration>(integration: &T) {
        // {hero{name}}
        let response = integration.get("/?query=%7Bhero%7Bname%7D%7D").await;

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type.as_str(), "application/json");

        assert_eq!(
            unwrap_json_response(&response),
            serde_json::from_str::<Json>(r#"{"data": {"hero": {"name": "R2-D2"}}}"#)
                .expect("Invalid JSON constant in test")
        );
    }

    async fn test_encoded_get<T: HttpIntegration>(integration: &T) {
        // query { human(id: "1000") { id, name, appearsIn, homePlanet } }
        let response = integration.get(
            "/?query=query%20%7B%20human(id%3A%20%221000%22)%20%7B%20id%2C%20name%2C%20appearsIn%2C%20homePlanet%20%7D%20%7D").await;

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type.as_str(), "application/json");

        assert_eq!(
            unwrap_json_response(&response),
            serde_json::from_str::<Json>(
                r#"{
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
                    }"#
            )
            .expect("Invalid JSON constant in test")
        );
    }

    async fn test_get_with_variables<T: HttpIntegration>(integration: &T) {
        // query($id: String!) { human(id: $id) { id, name, appearsIn, homePlanet } }
        // with variables = { "id": "1000" }
        let response = integration.get(
            "/?query=query(%24id%3A%20String!)%20%7B%20human(id%3A%20%24id)%20%7B%20id%2C%20name%2C%20appearsIn%2C%20homePlanet%20%7D%20%7D&variables=%7B%20%22id%22%3A%20%221000%22%20%7D").await;

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type, "application/json");

        assert_eq!(
            unwrap_json_response(&response),
            serde_json::from_str::<Json>(
                r#"{
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
                    }"#
            )
            .expect("Invalid JSON constant in test")
        );
    }

    async fn test_simple_post<T: HttpIntegration>(integration: &T) {
        let response = integration
            .post_json("/", r#"{"query": "{hero{name}}"}"#)
            .await;

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type, "application/json");

        assert_eq!(
            unwrap_json_response(&response),
            serde_json::from_str::<Json>(r#"{"data": {"hero": {"name": "R2-D2"}}}"#)
                .expect("Invalid JSON constant in test"),
        );
    }

    async fn test_batched_post<T: HttpIntegration>(integration: &T) {
        let response = integration
            .post_json(
                "/",
                r#"[{"query": "{hero{name}}"}, {"query": "{hero{name}}"}]"#,
            )
            .await;

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type, "application/json");

        assert_eq!(
            unwrap_json_response(&response),
            serde_json::from_str::<Json>(
                r#"[{"data": {"hero": {"name": "R2-D2"}}}, {"data": {"hero": {"name": "R2-D2"}}}]"#,
            )
            .expect("Invalid JSON constant in test"),
        );
    }

    async fn test_empty_batched_post<T: HttpIntegration>(integration: &T) {
        let response = integration.post_json("/", "[]").await;
        assert_eq!(response.status_code, 400);
    }

    async fn test_invalid_json<T: HttpIntegration>(integration: &T) {
        let response = integration.get("/?query=blah").await;
        assert_eq!(response.status_code, 400);
        let response = integration.post_json("/", r#"blah"#).await;
        assert_eq!(response.status_code, 400);
    }

    async fn test_invalid_field<T: HttpIntegration>(integration: &T) {
        // {hero{blah}}
        let response = integration.get("/?query=%7Bhero%7Bblah%7D%7D").await;
        assert_eq!(response.status_code, 400);
        let response = integration
            .post_json("/", r#"{"query": "{hero{blah}}"}"#)
            .await;
        assert_eq!(response.status_code, 400);
    }

    async fn test_duplicate_keys<T: HttpIntegration>(integration: &T) {
        // {hero{name}}
        let response = integration.get("/?query=%7B%22query%22%3A%20%22%7Bhero%7Bname%7D%7D%22%2C%20%22query%22%3A%20%22%7Bhero%7Bname%7D%7D%22%7D").await;
        assert_eq!(response.status_code, 400);
        let response = integration
            .post_json("/", r#"{"query": "{hero{name}}", "query": "{hero{name}}"}"#)
            .await;
        assert_eq!(response.status_code, 400);
    }

    async fn test_graphql_post<T: HttpIntegration>(integration: &T) {
        let resp = integration.post_graphql("/", r#"{hero{name}}"#).await;

        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.content_type, "application/json");

        assert_eq!(
            unwrap_json_response(&resp),
            serde_json::from_str::<Json>(r#"{"data": {"hero": {"name": "R2-D2"}}}"#)
                .expect("Invalid JSON constant in test"),
        );
    }

    async fn test_invalid_graphql_post<T: HttpIntegration>(integration: &T) {
        let resp = integration.post_graphql("/", r#"{hero{name}"#).await;

        assert_eq!(resp.status_code, 400);
    }
}
