use std::collections::HashMap;

use executor::FieldResult;
use value::Value;
use schema::model::RootNode;

enum Sample {
    One,
    Two,
}

struct Scalar(i64);

struct Interface {}

struct Root {}

graphql_scalar!(Scalar as "SampleScalar" {
    resolve(&self) -> Value {
        Value::int(self.0)
    }

    from_input_value(v: &InputValue) -> Option<Scalar> {
        v.as_int_value().map(|i| Scalar(i))
    }
});

graphql_enum!(Sample as "SampleEnum" {
    Sample::One => "ONE",
    Sample::Two => "TWO",
});

graphql_interface!(Interface: () as "SampleInterface" |&self| {
    description: "A sample interface"
    
    field sample_enum() -> FieldResult<Sample> as "A sample field in the interface" {
        Ok(Sample::One)
    }

    instance_resolvers: |&_| [
        Some(Root {}),
    ]
});

graphql_object!(Root: () as "Root" |&self| {
    interfaces: [Interface]

    field sample_enum() -> FieldResult<Sample> {
        Ok(Sample::One)
    }

    field sample_scalar() -> FieldResult<Scalar> {
        Ok(Scalar(123))
    }
});

#[test]
fn test_execution() {
    let doc = r#"
    {
        sampleEnum
        sampleScalar
    }
    "#;
    let schema = RootNode::new(Root {}, ());

    let (result, errs) = ::execute(doc, None, &schema, &HashMap::new(), &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    assert_eq!(result, Value::object(vec![
        ("sampleEnum", Value::string("ONE")),
        ("sampleScalar", Value::int(123)),
    ].into_iter().collect()));
}

#[test]
fn enum_introspection() {
    let doc = r#"
    {
        __type(name: "SampleEnum") {
            name
            kind
            description
            enumValues {
                name
                description
                isDeprecated
                deprecationReason
            }
            interfaces { name }
            possibleTypes { name }
            inputFields { name }
            ofType { name }
        }
    }
    "#;
    let schema = RootNode::new(Root {}, ());

    let (result, errs) = ::execute(doc, None, &schema, &HashMap::new(), &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value().expect("Result is not an object")
        .get("__type").expect("__type field missing")
        .as_object_value().expect("__type field not an object value");

    assert_eq!(type_info.get("name"), Some(&Value::string("SampleEnum")));
    assert_eq!(type_info.get("kind"), Some(&Value::string("ENUM")));
    assert_eq!(type_info.get("description"), Some(&Value::null())); 
    assert_eq!(type_info.get("interfaces"), Some(&Value::null()));
    assert_eq!(type_info.get("possibleTypes"), Some(&Value::null()));
    assert_eq!(type_info.get("inputFields"), Some(&Value::null()));
    assert_eq!(type_info.get("ofType"), Some(&Value::null()));

    let values = type_info
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

#[test]
fn interface_introspection() {
    let doc = r#"
    {
        __type(name: "SampleInterface") {
            name
            kind
            description
            possibleTypes {
                name
            }
            fields {
                name
                description
                args {
                    name
                }
                type {
                    name
                    kind
                    ofType {
                        name
                        kind
                    }
                }
                isDeprecated
                deprecationReason
            }
            interfaces { name }
            enumValues { name }
            inputFields { name }
            ofType { name }
        }
    }
    "#;
    let schema = RootNode::new(Root {}, ());

    let (result, errs) = ::execute(doc, None, &schema, &HashMap::new(), &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value().expect("Result is not an object")
        .get("__type").expect("__type field missing")
        .as_object_value().expect("__type field not an object value");

    assert_eq!(type_info.get("name"), Some(&Value::string("SampleInterface")));
    assert_eq!(type_info.get("kind"), Some(&Value::string("INTERFACE")));
    assert_eq!(type_info.get("description"), Some(&Value::string("A sample interface")));
    assert_eq!(type_info.get("interfaces"), Some(&Value::null()));
    assert_eq!(type_info.get("enumValues"), Some(&Value::null()));
    assert_eq!(type_info.get("inputFields"), Some(&Value::null()));
    assert_eq!(type_info.get("ofType"), Some(&Value::null()));

    let possible_types = type_info
        .get("possibleTypes").expect("possibleTypes field missing")
        .as_list_value().expect("possibleTypes not a list");

    assert_eq!(possible_types.len(), 1);

    assert!(possible_types.contains(&Value::object(vec![
        ("name", Value::string("Root")),
    ].into_iter().collect())));

    let fields = type_info
        .get("fields").expect("fields field missing")
        .as_list_value().expect("fields field not an object value");

    assert_eq!(fields.len(), 2);

    assert!(fields.contains(&Value::object(vec![
        ("name", Value::string("sampleEnum")),
        ("description", Value::string("A sample field in the interface")),
        ("args", Value::list(vec![])),
        ("type", Value::object(vec![
            ("name", Value::null()),
            ("kind", Value::string("NON_NULL")),
            ("ofType", Value::object(vec![
                ("name", Value::string("SampleEnum")),
                ("kind", Value::string("ENUM")),
            ].into_iter().collect())),
        ].into_iter().collect())),
        ("isDeprecated", Value::boolean(false)),
        ("deprecationReason", Value::null()),
    ].into_iter().collect())));
}

#[test]
fn scalar_introspection() {
    let doc = r#"
    {
        __type(name: "SampleScalar") {
            name
            kind
            description
            fields { name }
            interfaces { name }
            possibleTypes { name }
            enumValues { name }
            inputFields { name }
            ofType { name }
        }
    }
    "#;
    let schema = RootNode::new(Root {}, ());

    let (result, errs) = ::execute(doc, None, &schema, &HashMap::new(), &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value().expect("Result is not an object")
        .get("__type").expect("__type field missing");

    assert_eq!(type_info, &Value::object(vec![
        ("name", Value::string("SampleScalar")),
        ("kind", Value::string("SCALAR")),
        ("description", Value::null()),
        ("fields", Value::null()),
        ("interfaces", Value::null()),
        ("possibleTypes", Value::null()),
        ("enumValues", Value::null()),
        ("inputFields", Value::null()),
        ("ofType", Value::null()),
    ].into_iter().collect()));
}
