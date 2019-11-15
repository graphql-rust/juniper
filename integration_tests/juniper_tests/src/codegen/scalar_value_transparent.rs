use fnv::FnvHashMap;
use juniper::{DefaultScalarValue, FromInputValue, GraphQLType, InputValue, ToInputValue};

#[derive(juniper::GraphQLScalarValue, PartialEq, Eq, Debug)]
#[graphql(transparent)]
struct UserId(String);

#[derive(juniper::GraphQLScalarValue, PartialEq, Eq, Debug)]
#[graphql(transparent, name = "MyUserId", description = "custom description...")]
struct CustomUserId(String);

/// The doc comment...
#[derive(juniper::GraphQLScalarValue, PartialEq, Eq, Debug)]
#[graphql(transparent)]
struct IdWithDocComment(i32);

#[derive(juniper::GraphQLObject)]
struct User {
    id: UserId,
    id_custom: CustomUserId,
}

struct User2;

#[juniper::object]
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

    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = UserId::meta(&(), &mut registry);
    assert_eq!(meta.name(), Some("UserId"));
    assert_eq!(meta.description(), None);

    let input: InputValue = serde_json::from_value(serde_json::json!("userId1")).unwrap();
    let output: UserId = FromInputValue::from_input_value(&input).unwrap();
    assert_eq!(output, UserId("userId1".into()),);

    let id = UserId("111".into());
    let output = ToInputValue::<DefaultScalarValue>::to_input_value(&id);
    assert_eq!(output, InputValue::scalar("111"),);
}

#[test]
fn test_scalar_value_custom() {
    assert_eq!(
        <CustomUserId as GraphQLType<DefaultScalarValue>>::name(&()),
        Some("MyUserId")
    );

    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = CustomUserId::meta(&(), &mut registry);
    assert_eq!(meta.name(), Some("MyUserId"));
    assert_eq!(
        meta.description(),
        Some(&"custom description...".to_string())
    );

    let input: InputValue = serde_json::from_value(serde_json::json!("userId1")).unwrap();
    let output: CustomUserId = FromInputValue::from_input_value(&input).unwrap();
    assert_eq!(output, CustomUserId("userId1".into()),);

    let id = CustomUserId("111".into());
    let output = ToInputValue::<DefaultScalarValue>::to_input_value(&id);
    assert_eq!(output, InputValue::scalar("111"),);
}

#[test]
fn test_scalar_value_doc_comment() {
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = IdWithDocComment::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some(&"The doc comment...".to_string()));
}
