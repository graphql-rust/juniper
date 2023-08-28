mod enums;
mod input_object;

// This asserts that the input objects defined public actually became public
#[allow(unused_imports)]
use self::input_object::{NamedPublic, NamedPublicWithDescription};

use crate::{
    graphql_interface, graphql_object, graphql_value, graphql_vars,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    GraphQLEnum, GraphQLScalar,
};

#[derive(GraphQLEnum)]
#[graphql(name = "SampleEnum")]
enum Sample {
    One,
    Two,
}

#[derive(GraphQLScalar)]
#[graphql(name = "SampleScalar", transparent)]
struct Scalar(i32);

/// A sample interface
#[graphql_interface(name = "SampleInterface", for = Root)]
trait Interface {
    /// A sample field in the interface
    fn sample_enum(&self) -> Sample;
}

struct Root;

/// The root query object in the schema
#[graphql_object(impl = InterfaceValue)]
impl Root {
    fn sample_enum() -> Sample {
        Sample::One
    }

    /// A sample scalar field on the object
    fn sample_scalar(
        #[graphql(description = "The first number")] first: i32,
        #[graphql(description = "The second number", default = 123)] second: i32,
    ) -> Scalar {
        Scalar(first + second)
    }
}

#[tokio::test]
async fn test_execution() {
    let doc = r#"
    {
        sampleEnum
        first: sampleScalar(first: 0)
        second: sampleScalar(first: 10 second: 20)
    }
    "#;
    let schema = RootNode::new(
        Root,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = crate::execute(doc, None, &schema, &graphql_vars! {}, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {result:#?}");

    assert_eq!(
        result,
        graphql_value!({
            "sampleEnum": "ONE",
            "first": 123,
            "second": 30,
        }),
    );
}

#[tokio::test]
async fn enum_introspection() {
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
    let schema = RootNode::new(
        Root,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = crate::execute(doc, None, &schema, &graphql_vars! {}, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {result:#?}");

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    assert_eq!(
        type_info.get_field_value("name"),
        Some(&graphql_value!("SampleEnum")),
    );
    assert_eq!(
        type_info.get_field_value("kind"),
        Some(&graphql_value!("ENUM")),
    );
    assert_eq!(
        type_info.get_field_value("description"),
        Some(&graphql_value!(null)),
    );
    assert_eq!(
        type_info.get_field_value("interfaces"),
        Some(&graphql_value!(null)),
    );
    assert_eq!(
        type_info.get_field_value("possibleTypes"),
        Some(&graphql_value!(null)),
    );
    assert_eq!(
        type_info.get_field_value("inputFields"),
        Some(&graphql_value!(null)),
    );
    assert_eq!(
        type_info.get_field_value("ofType"),
        Some(&graphql_value!(null))
    );

    let values = type_info
        .get_field_value("enumValues")
        .expect("enumValues field missing")
        .as_list_value()
        .expect("enumValues not a list");

    assert_eq!(values.len(), 2);

    assert!(values.contains(&graphql_value!({
        "name": "ONE",
        "description": null,
        "isDeprecated": false,
        "deprecationReason": null,
    })));

    assert!(values.contains(&graphql_value!({
        "name": "TWO",
        "description": null,
        "isDeprecated": false,
        "deprecationReason": null,
    })));
}

#[tokio::test]
async fn interface_introspection() {
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
    let schema = RootNode::new(
        Root,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = crate::execute(doc, None, &schema, &graphql_vars! {}, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {result:#?}");

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    assert_eq!(
        type_info.get_field_value("name"),
        Some(&graphql_value!("SampleInterface")),
    );
    assert_eq!(
        type_info.get_field_value("kind"),
        Some(&graphql_value!("INTERFACE")),
    );
    assert_eq!(
        type_info.get_field_value("description"),
        Some(&graphql_value!("A sample interface")),
    );
    assert_eq!(
        type_info.get_field_value("interfaces"),
        Some(&graphql_value!([])),
    );
    assert_eq!(
        type_info.get_field_value("enumValues"),
        Some(&graphql_value!(null)),
    );
    assert_eq!(
        type_info.get_field_value("inputFields"),
        Some(&graphql_value!(null)),
    );
    assert_eq!(
        type_info.get_field_value("ofType"),
        Some(&graphql_value!(null))
    );

    let possible_types = type_info
        .get_field_value("possibleTypes")
        .expect("possibleTypes field missing")
        .as_list_value()
        .expect("possibleTypes not a list");

    assert_eq!(possible_types.len(), 1);

    assert!(possible_types.contains(&graphql_value!({"name": "Root"})));

    let fields = type_info
        .get_field_value("fields")
        .expect("fields field missing")
        .as_list_value()
        .expect("fields field not an object value");

    assert_eq!(fields.len(), 1);

    assert!(fields.contains(&graphql_value!({
        "name": "sampleEnum",
        "description": "A sample field in the interface",
        "args": [],
        "type": {
            "name": null,
            "kind": "NON_NULL",
            "ofType": {
               "name": "SampleEnum",
               "kind": "ENUM",
            },
        },
        "isDeprecated": false,
        "deprecationReason": null,
    })));
}

#[tokio::test]
async fn object_introspection() {
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
    let schema = RootNode::new(
        Root,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = crate::execute(doc, None, &schema, &graphql_vars! {}, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {result:#?}");

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    assert_eq!(
        type_info.get_field_value("name"),
        Some(&graphql_value!("Root")),
    );
    assert_eq!(
        type_info.get_field_value("kind"),
        Some(&graphql_value!("OBJECT")),
    );
    assert_eq!(
        type_info.get_field_value("description"),
        Some(&graphql_value!("The root query object in the schema")),
    );
    assert_eq!(
        type_info.get_field_value("interfaces"),
        Some(&graphql_value!([{"name": "SampleInterface"}])),
    );
    assert_eq!(
        type_info.get_field_value("enumValues"),
        Some(&graphql_value!(null)),
    );
    assert_eq!(
        type_info.get_field_value("inputFields"),
        Some(&graphql_value!(null)),
    );
    assert_eq!(
        type_info.get_field_value("ofType"),
        Some(&graphql_value!(null))
    );
    assert_eq!(
        type_info.get_field_value("possibleTypes"),
        Some(&graphql_value!(null)),
    );

    let fields = type_info
        .get_field_value("fields")
        .expect("fields field missing")
        .as_list_value()
        .expect("fields field not an object value");

    assert_eq!(fields.len(), 2);

    println!("Fields: {fields:#?}");

    assert!(fields.contains(&graphql_value!({
        "name": "sampleEnum",
        "description": null,
        "args": [],
        "type": {
            "name": null,
            "kind": "NON_NULL",
            "ofType": {
               "name": "SampleEnum",
               "kind": "ENUM",
            },
        },
        "isDeprecated": false,
        "deprecationReason": null,
    })));

    assert!(fields.contains(&graphql_value!({
        "name": "sampleScalar",
        "description": "A sample scalar field on the object",
        "args": [{
            "name": "first",
            "description": "The first number",
            "type": {
                "name": null,
                "kind": "NON_NULL",
                "ofType": {
                    "name": "Int",
                    "kind": "SCALAR",
                    "ofType": null,
                },
            },
            "defaultValue": null,
        }, {
            "name": "second",
            "description": "The second number",
            "type": {
                "name": null,
                "kind": "NON_NULL",
                "ofType": {
                    "name": "Int",
                    "kind": "SCALAR",
                    "ofType": null,
                },
            },
            "defaultValue": "123",
        }],
        "type": {
            "name": null,
            "kind": "NON_NULL",
            "ofType": {
               "name": "SampleScalar",
               "kind": "SCALAR",
            },
        },
        "isDeprecated": false,
        "deprecationReason": null,
    })));
}

#[tokio::test]
async fn scalar_introspection() {
    let doc = r#"
    {
        __type(name: "SampleScalar") {
            name
            kind
            description
            specifiedByUrl
            fields { name }
            interfaces { name }
            possibleTypes { name }
            enumValues { name }
            inputFields { name }
            ofType { name }
        }
    }
    "#;
    let schema = RootNode::new(
        Root,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = crate::execute(doc, None, &schema, &graphql_vars! {}, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {result:#?}");

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing");

    assert_eq!(
        type_info,
        &graphql_value!({
            "name": "SampleScalar",
            "kind": "SCALAR",
            "description": null,
            "specifiedByUrl": null,
            "fields": null,
            "interfaces": null,
            "possibleTypes": null,
            "enumValues": null,
            "inputFields": null,
            "ofType": null,
        }),
    );
}
