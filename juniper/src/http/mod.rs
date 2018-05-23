//! Utilities for building HTTP endpoints in a library-agnostic manner

pub mod graphiql;

use serde::ser;
use serde::ser::SerializeMap;

use ast::InputValue;
use executor::ExecutionError;
use {GraphQLError, GraphQLType, RootNode, Value, Variables};

/// The result of executing a query
pub trait ExecutionResponse: ser::Serialize {
    /// Was the request successful or not?
    fn is_ok(&self) -> bool;
}

/// A request that can be executed
pub trait Executable {
    /// The response that is produced by executing this request
    type Response: ExecutionResponse;

    /// Executes this request against the provided schema
    fn execute<CtxT, QueryT, MutationT>(
        self,
        root_node: &RootNode<QueryT, MutationT>,
        context: &CtxT,
    ) -> Self::Response
    where
        QueryT: GraphQLType<Context = CtxT>,
        MutationT: GraphQLType<Context = CtxT>;
}

/// The expected structure of the decoded JSON document for either POST or GET requests.
///
/// For POST, you can use Serde to deserialize the incoming JSON data directly
/// into this struct - it derives Deserialize for exactly this reason.
///
/// For GET, you will need to parse the query string and extract "query",
/// "operationName", and "variables" manually.
#[derive(Deserialize, Clone, Serialize, PartialEq, Debug)]
pub struct GraphQLRequest {
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    variables: Option<InputValue>,
}

impl GraphQLRequest {
    fn operation_name(&self) -> Option<&str> {
        self.operation_name.as_ref().map(|oper_name| &**oper_name)
    }

    fn variables(&self) -> Variables {
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
        variables: Option<InputValue>,
    ) -> GraphQLRequest {
        GraphQLRequest {
            query: query,
            operation_name: operation_name,
            variables: variables,
        }
    }
}

impl<'a> Executable for &'a GraphQLRequest {
    type Response = GraphQLResponse<'a>;

    /// Execute a GraphQL request using the specified schema and context
    ///
    /// This is a simple wrapper around the `execute` function exposed at the
    /// top level of this crate.
    fn execute<CtxT, QueryT, MutationT>(
        self,
        root_node: &RootNode<QueryT, MutationT>,
        context: &CtxT,
    ) -> Self::Response
    where
        QueryT: GraphQLType<Context = CtxT>,
        MutationT: GraphQLType<Context = CtxT>,
    {
        GraphQLResponse(::execute(
            &self.query,
            self.operation_name(),
            root_node,
            &self.variables(),
            context,
        ))
    }
}

/// Wraps the GraphQLRequest allowing for an array of requests to be handled in one request.
#[derive(Deserialize, Clone, Serialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum GraphQLBatchRequest {
    /// A single GraphQLRequest
    Single(GraphQLRequest),
    /// Multiple GraphQLRequests to be handled at once
    Batch(Vec<GraphQLRequest>),
}

impl<'a> Executable for &'a GraphQLBatchRequest {
    type Response = GraphQLBatchResponse<'a>;

    /// Execute all contained GraphQLRequests
    fn execute<CtxT, QueryT, MutationT>(
        self,
        root_node: &RootNode<QueryT, MutationT>,
        context: &CtxT,
    ) -> Self::Response
    where
        QueryT: GraphQLType<Context = CtxT>,
        MutationT: GraphQLType<Context = CtxT>,
    {
        match self {
            &GraphQLBatchRequest::Single(ref request) =>
                GraphQLBatchResponse::Single(
                    request.execute(root_node, context)
                ),
            &GraphQLBatchRequest::Batch(ref requests) =>
                GraphQLBatchResponse::Batch(
                    requests
                        .into_iter()
                        .map(|req| req.execute(root_node, context))
                        .collect()
                ),
        }
    }
}

/// Simple wrapper around the result from executing a GraphQL query
///
/// This struct implements Serialize, so you can simply serialize this
/// to JSON and send it over the wire. Use the `is_ok` method to determine
/// whether to send a 200 or 400 HTTP status code.
pub struct GraphQLResponse<'a>(Result<(Value, Vec<ExecutionError>), GraphQLError<'a>>);

impl<'a> ExecutionResponse for GraphQLResponse<'a> {
    /// Was the request successful or not?
    ///
    /// Note that there still might be errors in the response even though it's
    /// considered OK. This is by design in GraphQL.
    fn is_ok(&self) -> bool {
        self.0.is_ok()
    }
}

impl<'a> ser::Serialize for GraphQLResponse<'a> {
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

/// Wraps the GraphQLResponse so that multiple responses can be returned for a batched request.
pub enum GraphQLBatchResponse<'a> {
    /// A single GraphQLResponse
    Single(GraphQLResponse<'a>),
    /// Multiple GraphQLResponses that were handled at once
    Batch(Vec<GraphQLResponse<'a>>),
}

impl<'a> ExecutionResponse for GraphQLBatchResponse<'a> {
    /// Was the request successful or not?
    ///
    /// Requires that all the batched responses are ok.
    fn is_ok(&self) -> bool {
        match self {
            &GraphQLBatchResponse::Single(ref response) => response.is_ok(),
            &GraphQLBatchResponse::Batch(ref batch) => batch.iter().fold(true, |ok, res| ok && res.is_ok()),
        }
    }
}

impl<'a> ser::Serialize for GraphQLBatchResponse<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            &GraphQLBatchResponse::Single(ref response) => response.serialize(serializer),
            &GraphQLBatchResponse::Batch(ref batch) => batch.serialize(serializer),
        }
    }
}

#[cfg(any(test, feature = "expose-test-schema"))]
#[allow(missing_docs)]
pub mod tests {
    use serde_json;
    use serde_json::Value as Json;

    /// Normalized response content we expect to get back from
    /// the http framework integration we are testing.
    pub struct TestResponse {
        pub status_code: i32,
        pub body: Option<String>,
        pub content_type: String,
    }

    /// Normalized way to make requests to the http framework
    /// integration we are testing.
    pub trait HTTPIntegration {
        fn get(&self, url: &str) -> TestResponse;
        fn post(&self, url: &str, body: &str) -> TestResponse;
    }

    #[allow(missing_docs)]
    pub fn run_http_test_suite<T: HTTPIntegration>(integration: &T) {
        println!("Running HTTP Test suite for integration");

        println!("  - test_simple_get");
        test_simple_get(integration);

        println!("  - test_encoded_get");
        test_encoded_get(integration);

        println!("  - test_get_with_variables");
        test_get_with_variables(integration);

        println!("  - test_simple_post");
        test_simple_post(integration);

        println!("  - test_batched_post");
        test_batched_post(integration);
    }

    fn unwrap_json_response(response: &TestResponse) -> Json {
        serde_json::from_str::<Json>(
            response
                .body
                .as_ref()
                .expect("No data returned from request"),
        ).expect("Could not parse JSON object")
    }

    fn test_simple_get<T: HTTPIntegration>(integration: &T) {
        let response = integration.get("/?query={hero{name}}");

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type.as_str(), "application/json");

        assert_eq!(
            unwrap_json_response(&response),
            serde_json::from_str::<Json>(r#"{"data": {"hero": {"name": "R2-D2"}}}"#)
                .expect("Invalid JSON constant in test")
        );
    }

    fn test_encoded_get<T: HTTPIntegration>(integration: &T) {
        let response = integration.get(
            "/?query=query%20{%20%20%20human(id:%20\"1000\")%20{%20%20%20%20%20id,%20%20%20%20%20name,%20%20%20%20%20appearsIn,%20%20%20%20%20homePlanet%20%20%20}%20}");

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
            ).expect("Invalid JSON constant in test")
        );
    }

    fn test_get_with_variables<T: HTTPIntegration>(integration: &T) {
        let response = integration.get(
            "/?query=query($id:%20String!)%20{%20%20%20human(id:%20$id)%20{%20%20%20%20%20id,%20%20%20%20%20name,%20%20%20%20%20appearsIn,%20%20%20%20%20homePlanet%20%20%20}%20}&variables={%20%20%20\"id\":%20%20\"1000\"%20}");

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
            ).expect("Invalid JSON constant in test")
        );
    }

    fn test_simple_post<T: HTTPIntegration>(integration: &T) {
        let response = integration.post("/", r#"{"query": "{hero{name}}"}"#);

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type, "application/json");

        assert_eq!(
            unwrap_json_response(&response),
            serde_json::from_str::<Json>(r#"{"data": {"hero": {"name": "R2-D2"}}}"#)
                .expect("Invalid JSON constant in test")
        );
    }

    fn test_batched_post<T: HTTPIntegration>(integration: &T) {
        let response = integration.post("/", r#"[{"query": "{hero{name}}"}, {"query": "{hero{name}}"}]"#);

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type, "application/json");

        assert_eq!(
            unwrap_json_response(&response),
            serde_json::from_str::<Json>(r#"[{"data": {"hero": {"name": "R2-D2"}}}, {"data": {"hero": {"name": "R2-D2"}}}]"#)
                .expect("Invalid JSON constant in test")
        );
    }
}
