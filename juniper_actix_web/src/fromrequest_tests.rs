use super::*;
use actix_web::{http::header, test::TestRequest};

use url::form_urlencoded::Serializer as GetSerializer;

fn make_get(uri: &str) -> Result<GraphQLRequest, Error> {
    let (req, mut payload) = TestRequest::get().uri(uri).to_http_parts();
    GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait()
}

fn make_post(payload: &str, content_type: &str) -> Result<GraphQLRequest, Error> {
    let (req, mut payload) = TestRequest::post()
        .set_payload(payload)
        .header(header::CONTENT_TYPE, content_type)
        .to_http_parts();
    GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait()
}

fn check_error_get(uri: &str, error: &str) {
    let result = make_get(uri);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), error);
}

fn check_error_post(payload: &str, content_type: &str, error: &str) {
    let result = make_post(payload, content_type);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), error);
}

fn check_success_get(uri: &str, expected: GraphQLRequest) {
    let (req, mut payload) = TestRequest::get().uri(uri).to_http_parts();
    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);
}

fn check_success_post(payload: &str, content_type: &str, expected: GraphQLRequest) {
    let result = make_post(payload, content_type);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);
}

fn create_request(
    query: &str,
    operation_name: Option<&str>,
    variables: Option<&str>,
) -> JuniperGraphQLRequest {
    JuniperGraphQLRequest::new(
        query.to_string(),
        operation_name.map(|s| s.to_string()),
        variables.map(|s| serde_json::from_str::<InputValue>(s).unwrap()),
    )
}

fn create_single_request(
    query: &str,
    operation_name: Option<&str>,
    variables: Option<&str>,
) -> GraphQLRequest {
    GraphQLRequest(GraphQLBatchRequest::Single(create_request(
        query,
        operation_name,
        variables,
    )))
}

fn create_batch_request(requests: Vec<(&str, Option<&str>, Option<&str>)>) -> GraphQLRequest {
    GraphQLRequest(GraphQLBatchRequest::Batch(
        requests
            .into_iter()
            .map(|(query, operation_name, variables)| {
                create_request(query, operation_name, variables)
            })
            .collect(),
    ))
}

// Get tests
const URI_PREFIX: &'static str = "http://test.com";
const URI_PREFIX_Q: &'static str = "http://test.com?";

#[test]
fn test_empty_get() {
    check_error_get(
        &format!("{}", URI_PREFIX),
        "Query deserialize error: missing field `query`",
    );
}

#[test]
fn test_no_query() {
    let variables = r#"{
        "qux": "quux"
    }"#;
    let uri = GetSerializer::new(URI_PREFIX_Q.to_string())
        .append_pair("operationName", "foo")
        .append_pair("variables", variables)
        .finish();
    check_error_get(&uri, "Query deserialize error: missing field `query`");
}

#[test]
fn test_normal_get() {
    let query = "{ foo { bar, baz } }";
    let operation_name = "rust";
    let uri = &format!(
        "{}?query={}&operationName={}",
        URI_PREFIX, query, operation_name
    );

    let expected = create_single_request(query, Some(operation_name), None);
    check_success_get(uri, expected);
}

#[test]
fn test_get_all_fields() {
    let query = "query a($qux: Qux) { foo(qux: $qux) { bar } } query b { foo { baz } }";
    let operation_name = "b";
    let variables = r#"{
        "qux": "quux"
    }"#;

    let uri = GetSerializer::new(URI_PREFIX_Q.to_string())
        .append_pair("query", query)
        .append_pair("operationName", operation_name)
        .append_pair("variables", variables)
        .finish();
    let expected = create_single_request(query, Some(operation_name), Some(variables));
    check_success_get(&uri, expected);
}

#[test]
fn test_get_extra_fields() {
    let query = "{ foo { bar, baz } }";
    let operation_name = "rust";

    let uri = GetSerializer::new(URI_PREFIX_Q.to_string())
        .append_pair("query", query)
        .append_pair("operationName", operation_name)
        .append_pair("foo", "bar")
        .finish();

    check_error_get(&uri, "Query deserialize error: unknown field `foo`, expected one of `query`, `operationName`, `variables`");
}

#[test]
fn test_get_duplicate_query() {
    let query = "{ foo { bar, baz } }";
    let operation_name = "rust";

    let uri = GetSerializer::new(URI_PREFIX_Q.to_string())
        .append_pair("query", query)
        .append_pair("operationName", operation_name)
        .append_pair("query", "bar")
        .finish();

    check_error_get(&uri, "Query deserialize error: duplicate field `query`");
}

#[test]
fn test_get_duplicate_operation_name() {
    let query = "{ foo { bar, baz } }";

    let uri = GetSerializer::new(URI_PREFIX_Q.to_string())
        .append_pair("query", query)
        .append_pair("operationName", "qux")
        .append_pair("operationName", "quux")
        .finish();

    check_error_get(
        &uri,
        "Query deserialize error: duplicate field `operationName`",
    );
}

// Post tests
#[test]
fn test_invalid_post() {
    check_error_post(
        "NOT JSON",
        "application/json",
        "Json deserialize error: expected value at line 1 column 1",
    );
}

#[test]
fn test_empty_post_single() {
    check_error_post(
        "{}",
        "application/json",
        "Json deserialize error: data did not match any variant of untagged enum GraphQLBatchRequest"
    );
}

#[test]
fn test_empty_post_batch() {
    check_error_post(
        "[]",
        "application/json",
        "Json deserialize error: data did not match any variant of untagged enum GraphQLBatchRequest"
    );
}

#[test]
fn test_post_single() {
    let query = "{ foo { bar }}";
    let payload = &format!(
        r#"{{
                "query": "{}"
            }}"#,
        query
    );

    let expected = create_single_request(query, None, None);
    check_success_post(payload, "application/json", expected);
}

#[test]
fn test_post_batch() {
    let query1 = "{ foo { bar } }";
    let query2 = "{ foo { bar } }";

    let payload = &format!(
        r#"[
        {{ "query": "{}" }},
        {{ "query": "{}" }}
    ]"#,
        query1, query2
    );

    let expected = create_batch_request(vec![(query1, None, None), (query2, None, None)]);
    check_success_post(payload, "application/json", expected);
}

#[test]
fn test_post_duplicate_field() {
    let payload = r#"{
                "query": "foo",
                "query": "bar"
            }"#;

    check_error_post(payload, "application/json", "Json deserialize error: data did not match any variant of untagged enum GraphQLBatchRequest");
}

#[test]
fn test_post_variables() {
    let query = "quux";
    let variables = r#"{"meep": "morp"}"#;
    let payload = &format!(
        r#"{{
                "query": "{}",
                "variables": {}
            }}"#,
        query, variables
    );

    let expected = create_single_request(query, None, Some(variables));
    check_success_post(payload, "application/json", expected);
}

#[test]
fn test_post_graphql() {
    let query = "{ meep { morp } }";
    let expected = create_single_request(query, None, None);
    check_success_post(query, "application/graphql", expected);
}
