use fnv::FnvHashMap;
use juniper::{
    graphql_input_value, graphql_object, DefaultScalarValue, FromInputValue, GraphQLObject,
    GraphQLScalar, GraphQLType, InputValue, Registry, ToInputValue,
};

#[derive(GraphQLScalar, Debug, Eq, PartialEq)]
struct UserId(String);

#[derive(GraphQLScalar, Debug, Eq, PartialEq)]
#[graphql(name = "MyUserId", description = "custom description...")]
struct CustomUserId(String);

/// The doc comment...
#[derive(GraphQLScalar, Debug, Eq, PartialEq)]
#[graphql(specified_by_url = "https://tools.ietf.org/html/rfc4122")]
struct IdWithDocComment(i32);

#[derive(GraphQLObject)]
struct User {
    id: UserId,
    id_custom: CustomUserId,
}

struct User2;

#[graphql_object]
impl User2 {
    fn id(&self) -> UserId {
        UserId("id".to_string())
    }
}

#[test]
fn test_scalar_value_simple() {
    assert_eq!(
        <UserId as GraphQLType<DefaultScalarValue>>::name(&()),
        Some("UserId")
    );

    let mut registry: Registry = Registry::new(FnvHashMap::default());
    let meta = UserId::meta(&(), &mut registry);
    assert_eq!(meta.name(), Some("UserId"));
    assert_eq!(meta.description(), None);

    let input: InputValue = serde_json::from_value(serde_json::json!("userId1")).unwrap();
    let output: UserId = FromInputValue::from_input_value(&input).unwrap();
    assert_eq!(output, UserId("userId1".into()),);

    let id = UserId("111".into());
    let output = ToInputValue::<DefaultScalarValue>::to_input_value(&id);
    assert_eq!(output, graphql_input_value!("111"));
}

#[test]
fn test_scalar_value_custom() {
    assert_eq!(
        <CustomUserId as GraphQLType<DefaultScalarValue>>::name(&()),
        Some("MyUserId")
    );

    let mut registry: Registry = Registry::new(FnvHashMap::default());
    let meta = CustomUserId::meta(&(), &mut registry);
    assert_eq!(meta.name(), Some("MyUserId"));
    assert_eq!(meta.description(), Some("custom description..."));
    assert_eq!(meta.specified_by_url(), None);

    let input: InputValue = serde_json::from_value(serde_json::json!("userId1")).unwrap();
    let output: CustomUserId = FromInputValue::from_input_value(&input).unwrap();
    assert_eq!(output, CustomUserId("userId1".into()),);

    let id = CustomUserId("111".into());
    let output = ToInputValue::<DefaultScalarValue>::to_input_value(&id);
    assert_eq!(output, graphql_input_value!("111"));
}

#[test]
fn test_scalar_value_doc_comment() {
    let mut registry: Registry = Registry::new(FnvHashMap::default());
    let meta = IdWithDocComment::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some("The doc comment..."));
    assert_eq!(
        meta.specified_by_url(),
        Some("https://tools.ietf.org/html/rfc4122"),
    );
}
