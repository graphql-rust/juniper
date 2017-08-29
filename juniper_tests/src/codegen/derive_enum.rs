#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
use juniper::{self, FromInputValue, GraphQLType, InputValue, ToInputValue};

#[derive(GraphQLEnum, Debug, PartialEq)]
#[graphql(name = "Some", description = "enum descr")]
enum SomeEnum {
    Regular,

    #[graphql(name = "FULL", description = "field descr", deprecated = "depr")]
    Full,
}

#[test]
fn test_derived_enum() {
    // Ensure that rename works.
    assert_eq!(SomeEnum::name(&()), Some("Some"));

    // Ensure validity of meta info.
    let mut registry = juniper::Registry::new(HashMap::new());
    let meta = SomeEnum::meta(&(), &mut registry);

    assert_eq!(meta.name(), Some("Some"));
    assert_eq!(meta.description(), Some(&"enum descr".to_string()));

    // Test Regular variant.
    assert_eq!(SomeEnum::Regular.to_input_value(), InputValue::String("REGULAR".into()));
    assert_eq!(
        FromInputValue::from_input_value(&InputValue::String("REGULAR".into())),
        Some(SomeEnum::Regular)
    );

    // Test FULL variant.
    assert_eq!(SomeEnum::Full.to_input_value(), InputValue::String("FULL".into()));
    assert_eq!(
        FromInputValue::from_input_value(&InputValue::String("FULL".into())),
        Some(SomeEnum::Full)
    );
}
