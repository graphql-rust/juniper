mod enums;
mod input_object;

// This asserts that the input objects defined public actually became public
#[allow(unused_imports)]
use self::input_object::{NamedPublic, NamedPublicWithDescription};

use executor::Variables;
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use value::Value;

#[derive(GraphQLEnum)]
#[graphql(name = "SampleEnum", _internal)]
enum Sample {
    One,
    Two,
}

struct Scalar(i32);

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

graphql_interface!(Interface: () as "SampleInterface" |&self| {
    description: "A sample interface"

    field sample_enum() -> Sample as "A sample field in the interface" {
        Sample::One
    }

    instance_resolvers: |&_| {
        Root => Some(Root {}),
    }
});

graphql_object!(Root: () |&self| {
    description: "The root query object in the schema"

    interfaces: [Interface]

    field sample_enum() -> Sample {
        Sample::One
    }

    field sample_scalar(
        first: i32 as "The first number",
        second = 123: i32 as "The second number"
    ) -> Scalar as "A sample scalar field on the object" {
        Scalar(first + second)
    }
});

#[test]
fn test_execution() {
    let doc = r#"
    {
        sampleEnum
        first: sampleScalar(first: 0)
        second: sampleScalar(first: 10 second: 20)
    }
    "#;
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) =
        ::execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    assert_eq!(
        result,
        Value::object(
            vec![
                ("sampleEnum", Value::string("ONE")),
                ("first", Value::int(123)),
                ("second", Value::int(30)),
            ].into_iter()
                .collect()
        )
    );
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
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) =
        ::execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    assert_eq!(
        type_info.get_field_value("name"),
        Some(&Value::string("SampleEnum"))
    );
    assert_eq!(
        type_info.get_field_value("kind"),
        Some(&Value::string("ENUM"))
    );
    assert_eq!(
        type_info.get_field_value("description"),
        Some(&Value::null())
    );
    assert_eq!(
        type_info.get_field_value("interfaces"),
        Some(&Value::null())
    );
    assert_eq!(
        type_info.get_field_value("possibleTypes"),
        Some(&Value::null())
    );
    assert_eq!(
        type_info.get_field_value("inputFields"),
        Some(&Value::null())
    );
    assert_eq!(type_info.get_field_value("ofType"), Some(&Value::null()));

    let values = type_info
        .get_field_value("enumValues")
        .expect("enumValues field missing")
        .as_list_value()
        .expect("enumValues not a list");

    assert_eq!(values.len(), 2);

    assert!(
        values.contains(&Value::object(
            vec![
                ("name", Value::string("ONE")),
                ("description", Value::null()),
                ("isDeprecated", Value::boolean(false)),
                ("deprecationReason", Value::null()),
            ].into_iter()
                .collect(),
        ))
    );

    assert!(
        values.contains(&Value::object(
            vec![
                ("name", Value::string("TWO")),
                ("description", Value::null()),
                ("isDeprecated", Value::boolean(false)),
                ("deprecationReason", Value::null()),
            ].into_iter()
                .collect(),
        ))
    );
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
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) =
        ::execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    assert_eq!(
        type_info.get_field_value("name"),
        Some(&Value::string("SampleInterface"))
    );
    assert_eq!(
        type_info.get_field_value("kind"),
        Some(&Value::string("INTERFACE"))
    );
    assert_eq!(
        type_info.get_field_value("description"),
        Some(&Value::string("A sample interface"))
    );
    assert_eq!(
        type_info.get_field_value("interfaces"),
        Some(&Value::null())
    );
    assert_eq!(
        type_info.get_field_value("enumValues"),
        Some(&Value::null())
    );
    assert_eq!(
        type_info.get_field_value("inputFields"),
        Some(&Value::null())
    );
    assert_eq!(type_info.get_field_value("ofType"), Some(&Value::null()));

    let possible_types = type_info
        .get_field_value("possibleTypes")
        .expect("possibleTypes field missing")
        .as_list_value()
        .expect("possibleTypes not a list");

    assert_eq!(possible_types.len(), 1);

    assert!(possible_types.contains(&Value::object(
        vec![("name", Value::string("Root"))].into_iter().collect()
    )));

    let fields = type_info
        .get_field_value("fields")
        .expect("fields field missing")
        .as_list_value()
        .expect("fields field not an object value");

    assert_eq!(fields.len(), 1);

    assert!(
        fields.contains(&Value::object(
            vec![
                ("name", Value::string("sampleEnum")),
                (
                    "description",
                    Value::string("A sample field in the interface"),
                ),
                ("args", Value::list(vec![])),
                (
                    "type",
                    Value::object(
                        vec![
                            ("name", Value::null()),
                            ("kind", Value::string("NON_NULL")),
                            (
                                "ofType",
                                Value::object(
                                    vec![
                                        ("name", Value::string("SampleEnum")),
                                        ("kind", Value::string("ENUM")),
                                    ].into_iter()
                                        .collect(),
                                ),
                            ),
                        ].into_iter()
                            .collect(),
                    ),
                ),
                ("isDeprecated", Value::boolean(false)),
                ("deprecationReason", Value::null()),
            ].into_iter()
                .collect(),
        ))
    );
}

#[test]
fn object_introspection() {
    let doc = r#"
    {
        __type(name: "Root") {
            name
            kind
            description
            fields {
                name
                description
                args {
                    name
                    description
                    type {
                        name
                        kind
                        ofType {
                            name
                            kind
                            ofType {
                                name
                            }
                        }
                    }
                    defaultValue
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
            possibleTypes { name }
            interfaces { name }
            enumValues { name }
            inputFields { name }
            ofType { name }
        }
    }
    "#;
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) =
        ::execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    assert_eq!(
        type_info.get_field_value("name"),
        Some(&Value::string("Root"))
    );
    assert_eq!(
        type_info.get_field_value("kind"),
        Some(&Value::string("OBJECT"))
    );
    assert_eq!(
        type_info.get_field_value("description"),
        Some(&Value::string("The root query object in the schema"))
    );
    assert_eq!(
        type_info.get_field_value("interfaces"),
        Some(&Value::list(vec![Value::object(
            vec![("name", Value::string("SampleInterface"))]
                .into_iter()
                .collect(),
        )]))
    );
    assert_eq!(
        type_info.get_field_value("enumValues"),
        Some(&Value::null())
    );
    assert_eq!(
        type_info.get_field_value("inputFields"),
        Some(&Value::null())
    );
    assert_eq!(type_info.get_field_value("ofType"), Some(&Value::null()));
    assert_eq!(
        type_info.get_field_value("possibleTypes"),
        Some(&Value::null())
    );

    let fields = type_info
        .get_field_value("fields")
        .expect("fields field missing")
        .as_list_value()
        .expect("fields field not an object value");

    assert_eq!(fields.len(), 2);

    println!("Fields: {:#?}", fields);

    assert!(
        fields.contains(&Value::object(
            vec![
                ("name", Value::string("sampleEnum")),
                ("description", Value::null()),
                ("args", Value::list(vec![])),
                (
                    "type",
                    Value::object(
                        vec![
                            ("name", Value::null()),
                            ("kind", Value::string("NON_NULL")),
                            (
                                "ofType",
                                Value::object(
                                    vec![
                                        ("name", Value::string("SampleEnum")),
                                        ("kind", Value::string("ENUM")),
                                    ].into_iter()
                                        .collect(),
                                ),
                            ),
                        ].into_iter()
                            .collect(),
                    ),
                ),
                ("isDeprecated", Value::boolean(false)),
                ("deprecationReason", Value::null()),
            ].into_iter()
                .collect(),
        ))
    );

    assert!(
        fields.contains(&Value::object(
            vec![
                ("name", Value::string("sampleScalar")),
                (
                    "description",
                    Value::string("A sample scalar field on the object"),
                ),
                (
                    "args",
                    Value::list(vec![
                        Value::object(
                            vec![
                                ("name", Value::string("first")),
                                ("description", Value::string("The first number")),
                                (
                                    "type",
                                    Value::object(
                                        vec![
                                            ("name", Value::null()),
                                            ("kind", Value::string("NON_NULL")),
                                            (
                                                "ofType",
                                                Value::object(
                                                    vec![
                                                        ("name", Value::string("Int")),
                                                        ("kind", Value::string("SCALAR")),
                                                        ("ofType", Value::null()),
                                                    ].into_iter()
                                                        .collect(),
                                                ),
                                            ),
                                        ].into_iter()
                                            .collect(),
                                    ),
                                ),
                                ("defaultValue", Value::null()),
                            ].into_iter()
                                .collect(),
                        ),
                        Value::object(
                            vec![
                                ("name", Value::string("second")),
                                ("description", Value::string("The second number")),
                                (
                                    "type",
                                    Value::object(
                                        vec![
                                            ("name", Value::string("Int")),
                                            ("kind", Value::string("SCALAR")),
                                            ("ofType", Value::null()),
                                        ].into_iter()
                                            .collect(),
                                    ),
                                ),
                                ("defaultValue", Value::string("123")),
                            ].into_iter()
                                .collect(),
                        ),
                    ]),
                ),
                (
                    "type",
                    Value::object(
                        vec![
                            ("name", Value::null()),
                            ("kind", Value::string("NON_NULL")),
                            (
                                "ofType",
                                Value::object(
                                    vec![
                                        ("name", Value::string("SampleScalar")),
                                        ("kind", Value::string("SCALAR")),
                                    ].into_iter()
                                        .collect(),
                                ),
                            ),
                        ].into_iter()
                            .collect(),
                    ),
                ),
                ("isDeprecated", Value::boolean(false)),
                ("deprecationReason", Value::null()),
            ].into_iter()
                .collect(),
        ))
    );
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
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) =
        ::execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing");

    assert_eq!(
        type_info,
        &Value::object(
            vec![
                ("name", Value::string("SampleScalar")),
                ("kind", Value::string("SCALAR")),
                ("description", Value::null()),
                ("fields", Value::null()),
                ("interfaces", Value::null()),
                ("possibleTypes", Value::null()),
                ("enumValues", Value::null()),
                ("inputFields", Value::null()),
                ("ofType", Value::null()),
            ].into_iter()
                .collect()
        )
    );
}
