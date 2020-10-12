#[test]
fn operation_name_is_public() {
    use juniper::http::GraphQLRequest;

    let request = GraphQLRequest::new("query".to_string(), Some("name".to_string()), None);

    assert_eq!(request.operation_name(), Some("name"));
}
