#[cfg(test)]
use fnv::FnvHashMap;

#[cfg(test)]
use juniper::{self, FromInputValue, GraphQLType, InputValue};

#[derive(GraphQLInputObject, Debug, PartialEq)]
#[graphql(name = "MyInput", description = "input descr")]
struct Input {
    regular_field: String,
    #[graphql(name = "haha", default = "33", description = "haha descr")] c: i32,

    #[graphql(default)] other: Option<bool>,
}

#[test]
fn test_derived_input_object() {
    assert_eq!(Input::name(&()), Some("MyInput"));

    // Validate meta info.
    let mut registry = juniper::Registry::new(FnvHashMap::default());
    let meta = Input::meta(&(), &mut registry);
    assert_eq!(meta.name(), Some("MyInput"));
    assert_eq!(meta.description(), Some(&"input descr".to_string()));

    // Test default value injection.

    let input_no_defaults: InputValue = ::serde_json::from_value(json!({
        "regularField": "a",
    })).unwrap();

    let output_no_defaults: Input = FromInputValue::from_input_value(&input_no_defaults).unwrap();
    assert_eq!(
        output_no_defaults,
        Input {
            regular_field: "a".into(),
            c: 33,
            other: None,
        }
    );

    // Test with all values supplied.

    let input: InputValue = ::serde_json::from_value(json!({
        "regularField": "a",
        "haha": 55,
        "other": true,
    })).unwrap();

    let output: Input = FromInputValue::from_input_value(&input).unwrap();
    assert_eq!(
        output,
        Input {
            regular_field: "a".into(),
            c: 55,
            other: Some(true),
        }
    );
}
