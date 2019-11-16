use super::*;
use actix_web::{http::header, test::TestRequest};

fn req_is_single(req: &GraphQLRequest) -> bool {
    if let GraphQLRequest(GraphQLBatchRequest::Single(_)) = req {
        true
    } else {
        false
    }
}

// Get tests
const URI_PREFIX: &'static str = "http://test.com";

#[test]
fn test_empty_get() {
    let (req, mut payload) = TestRequest::get()
        .uri(&format!("{}", URI_PREFIX))
        .to_http_parts();
    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Query deserialize error: missing field `query`"
    );
}

#[test]
fn test_no_query() {
    let (req, mut payload) = TestRequest::get()
        .uri("http://example.com?operationName=foo&variables={}")
        .to_http_parts();

    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Query deserialize error: missing field `query`"
    );
}

#[test]
fn test_normal_get() {
    let query = "{foo{bar,baz}}";
    let (req, mut payload) = TestRequest::get()
        .uri(&format!(
            "{}?query={}&operationName=rust",
            URI_PREFIX, query
        ))
        .to_http_parts();

    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_ok());
    assert!(req_is_single(&result.unwrap()));
}

#[test]
fn test_get_all_fields() {
    use url::form_urlencoded;
    let query = "query a($qux: Qux) { foo(qux: $qux) { bar } } query b { foo { baz } }";
    let operation_name = "b";
    let variables = r#"{
        "qux": "quux"
    }"#;

    let encoded: String = form_urlencoded::Serializer::new(String::new())
        .append_pair("query", query)
        .append_pair("operationName", operation_name)
        .append_pair("variables", variables)
        .finish();

    let (req, mut payload) = TestRequest::get()
        .uri(&format!("{}?{}", URI_PREFIX, encoded))
        .to_http_parts();
    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_ok());
    assert!(req_is_single(&result.unwrap()));
}

#[test]
fn test_get_extra_fields() {
    let query = "{foo{bar,baz}}";
    let (req, mut payload) = TestRequest::get()
        .uri(&format!(
            "{}?query={}&operationName=rust&foo=bar",
            URI_PREFIX, query
        ))
        .to_http_parts();
    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .starts_with("Query deserialize error: unknown field `foo`"))
}

#[test]
fn test_get_duplicate_query() {
    let query = "{foo{bar,baz}}";
    let (req, mut payload) = TestRequest::get()
        .uri(&format!(
            "{}?query={}&operationName=rust&query=bar",
            URI_PREFIX, query
        ))
        .to_http_parts();
    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Query deserialize error: duplicate field `query`"
    );
}

#[test]
fn test_get_duplicate_operation_name() {
    let query = "{foo{bar,baz}}";
    let (req, mut payload) = TestRequest::get()
        .uri(&format!(
            "{}?query={}&operationName=rust&operationName=bar",
            URI_PREFIX, query
        ))
        .to_http_parts();

    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Query deserialize error: duplicate field `operationName`"
    );
}

// Post tests
#[test]
fn test_empty_post_single() {
    let (req, mut payload) = TestRequest::post()
        .set_payload("{}")
        .header(header::CONTENT_TYPE, "application/json")
        .to_http_parts();

    let result: Result<GraphQLRequest, _> =
        GraphQLRequest::from_request(&req, &mut payload).wait();
    assert!(result.is_err());
}

#[test]
fn test_empty_post_batch() {
    let (req, mut payload) = TestRequest::post()
        .set_payload("[]")
        .header(header::CONTENT_TYPE, "application/json")
        .to_http_parts();
    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_err());
}

#[test]
fn test_post_single() {
    let (req, mut payload) = TestRequest::post()
        .set_payload(
            r#"{
                "query": "{foo { bar }}"
            }"#,
        )
        .header(header::CONTENT_TYPE, "application/json")
        .to_http_parts();

    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_ok());
    assert!(req_is_single(&result.unwrap()));
}

#[test]
fn test_post_batch() {
    let (req, mut payload) = TestRequest::post()
        .set_payload(
            r#"[
                { "query": "{ foo { bar } }" },
                { "query": "{ baz { qux } }" }
            ]"#,
        )
        .header(header::CONTENT_TYPE, "application/json")
        .to_http_parts();

    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_ok());
    assert!(!req_is_single(&result.unwrap()));
}

#[test]
fn test_post_duplicate_field() {
    let (req, mut payload) = TestRequest::post()
        .set_payload(
            r#"{
                "query": "foo",
                "query": "bar"
            }"#,
        )
        .header(header::CONTENT_TYPE, "application/json")
        .to_http_parts();

    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_err());
}

#[test]
fn test_post_variables() {
    let (req, mut payload) = TestRequest::post()
        .set_payload(
            r#"{
                "query": "meep",
                "variables": {"foo": "bar"}
            }"#,
        )
        .header(header::CONTENT_TYPE, "application/json")
        .to_http_parts();
    let result = GraphQLRequest::<DefaultScalarValue>::from_request(&req, &mut payload).wait();
    assert!(result.is_ok());
    assert!(req_is_single(&result.unwrap()));
}