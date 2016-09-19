use std::collections::HashMap;

use executor::FieldResult;
use value::Value;
use schema::model::RootNode;

enum Sample {
    One,
    Two,
}

struct Root {}

graphql_enum!(Sample as "SampleEnum" {
    Sample::One => "ONE",
    Sample::Two => "TWO",
});

graphql_object!(Root: () as "Root" |&self| {
    field sample_enum() -> FieldResult<Sample> {
        Ok(Sample::One)
    }
});

#[test]
fn enum_introspection() {
    let doc = r#"
    {
        __type(name: "SampleEnum") {
            enumValues {
                name
                description
                isDeprecated
                deprecationReason
            }
        }
    }
    "#;
    let schema = RootNode::new(Root {}, ());

    let (result, errs) = ::execute(doc, None, &schema, &HashMap::new(), &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let values = result
        .as_object_value().expect("Result is not an object")
        .get("__type").expect("__type field missing")
        .as_object_value().expect("__type field not an object value")
        .get("enumValues").expect("enumValues field missing")
        .as_list_value().expect("enumValues not a list");

    assert_eq!(values.len(), 2);

    assert!(values.contains(&Value::object(vec![
        ("name", Value::string("ONE")),
        ("description", Value::null()),
        ("isDeprecated", Value::boolean(false)),
        ("deprecationReason", Value::null()),
    ].into_iter().collect())));

    assert!(values.contains(&Value::object(vec![
        ("name", Value::string("TWO")),
        ("description", Value::null()),
        ("isDeprecated", Value::boolean(false)),
        ("deprecationReason", Value::null()),
    ].into_iter().collect())));
}
