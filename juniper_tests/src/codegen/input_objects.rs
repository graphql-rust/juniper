use juniper::{self, InputValue, ToInputValue, GraphQLType, FromInputValue};

#[derive(GraphQLInputObject, Debug, PartialEq)]
#[graphql(name="MyInput")]
struct Input {
  regular_field: String,
  #[graphql(name="haha", default="33")]
  c: i32,
}

#[test]
fn test_derived_input_object() {
  assert_eq!(Input::name(), Some("MyInput"));

  let obj = Input {
    regular_field: "a".to_string(),
    c: 33,
  };
  let restored: Input = FromInputValue::from(&obj.to()).unwrap();
  assert_eq!(obj, restored);
}
