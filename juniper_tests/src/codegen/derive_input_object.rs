#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
use juniper::{self, FromInputValue, GraphQLType, ToInputValue};

#[derive(GraphQLInputObject, Debug, PartialEq)]
#[graphql(name = "MyInput", description = "input descr")]
struct Input {
    regular_field: String,
    #[graphql(name = "haha", default = "33", description = "haha descr")]
    c: i32,
}

#[test]
fn test_derived_input_object() {
    assert_eq!(Input::name(&()), Some("MyInput"));

    // Validate meta info.
    let mut registry = juniper::Registry::new(HashMap::new());
    let meta = Input::meta(&(), &mut registry);
    assert_eq!(meta.name(), Some("MyInput"));
    assert_eq!(meta.description(), Some(&"input descr".to_string()));

    let obj = Input {
        regular_field: "a".to_string(),
        c: 33,
    };
    let restored: Input = FromInputValue::from_input_value(&obj.to()).unwrap();
    assert_eq!(obj, restored);
}
