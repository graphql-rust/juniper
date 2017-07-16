use std::collections::HashMap;

use juniper::{self, InputValue, ToInputValue, GraphQLType, FromInputValue};

#[derive(GraphQLEnum, Debug, PartialEq)]
#[graphql(name="Some", description="enum descr")]
enum SomeEnum {
  Regular,

  #[graphql(
    name="FULL",
    description="field descr",
    deprecated="depr"
  )]
  Full,
}

#[test]
fn test_derived_enum() {
  // Ensure that rename works.
  assert_eq!(SomeEnum::name(), Some("Some"));

  // Ensure validity of meta info.
  let mut registry = juniper::Registry::new(HashMap::new());
  let meta = SomeEnum::meta(&mut registry);

  // Test Regular variant.
  assert_eq!(
    SomeEnum::Regular.to(),
    InputValue::String("REGULAR".into())
  );
  assert_eq!(
    FromInputValue::from(&InputValue::String("REGULAR".into())),
    Some(SomeEnum::Regular)
  );

  // Test FULL variant.
  assert_eq!(
    SomeEnum::Full.to(),
    InputValue::String("FULL".into())
  );
  assert_eq!(
    FromInputValue::from(&InputValue::String("FULL".into())),
    Some(SomeEnum::Full)
  );
}
