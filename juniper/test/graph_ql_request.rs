extern crate juniper;

#[test]
fn pub_operation_name() {
    use juniper::http::GraphQLRequest;

    let request = GraphQLRequest::new("query".to_string(), Some("name".to_string()), None);

    assert_eq!(request.operation_name(), Some("name"));
}
