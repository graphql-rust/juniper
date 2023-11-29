#![deny(unused_variables)]

use crate::{
    ast::{FromInputValue, InputValue},
    graphql_input_value, graphql_object, graphql_value, graphql_vars,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    value::{DefaultScalarValue, Object, Value},
    GraphQLInputObject,
};

struct Root;

#[derive(GraphQLInputObject, Debug)]
struct DefaultName {
    field_one: String,
    field_two: String,
}

#[derive(GraphQLInputObject, Debug)]
struct NoTrailingComma {
    field_one: String,
    field_two: String,
}

#[derive(GraphQLInputObject, Debug)]
struct Derive {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(name = "ANamedInputObject")]
struct Named {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(description = "Description for the input object")]
struct Description {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
pub struct Public {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(description = "Description for the input object")]
pub struct PublicWithDescription {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(
    name = "APublicNamedInputObjectWithDescription",
    description = "Description for the input object"
)]
pub struct NamedPublicWithDescription {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(name = "APublicNamedInputObject")]
pub struct NamedPublic {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
struct FieldDescription {
    #[graphql(description = "The first field")]
    field_one: String,
    #[graphql(description = "The second field")]
    field_two: String,
}

#[derive(GraphQLInputObject, Debug)]
struct FieldWithDefaults {
    #[graphql(default = 123)]
    field_one: i32,
    #[graphql(default = 456, description = "The second field")]
    field_two: i32,
}

#[graphql_object]
impl Root {
    fn test_field(
        a1: DefaultName,
        a2: NoTrailingComma,
        a3: Derive,
        a4: Named,
        a5: Description,
        a6: FieldDescription,
        a7: Public,
        a8: PublicWithDescription,
        a9: NamedPublicWithDescription,
        a10: NamedPublic,
        a11: FieldWithDefaults,
    ) -> i32 {
        let _ = a1;
        let _ = a2;
        let _ = a3;
        let _ = a4;
        let _ = a5;
        let _ = a6;
        let _ = a7;
        let _ = a8;
        let _ = a9;
        let _ = a10;
        let _ = a11;
        0
    }
}

async fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn(&Object<DefaultScalarValue>, &Vec<Value<DefaultScalarValue>>),
{
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

    let fields = type_info
        .get_field_value("inputFields")
        .expect("inputFields field missing")
        .as_list_value()
        .expect("inputFields not a list");

    f(type_info, fields);
}

#[tokio::test]
async fn default_name_introspection() {
    let doc = r#"{
        __type(name: "DefaultName") {
            name
            description
            inputFields {
                name
                description
                type {
                    ofType {
                        name
                    }
                }
                defaultValue
            }
        }
    }"#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("DefaultName")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&graphql_value!({
            "name": "fieldOne",
            "description": null,
            "type": {
                "ofType": {"name": "String"},
            },
            "defaultValue": null,
        })));
        assert!(fields.contains(&graphql_value!({
            "name": "fieldTwo",
            "description": null,
            "type": {
                "ofType": {"name": "String"},
            },
            "defaultValue": null,
        })));
    })
    .await;
}

#[test]
fn default_name_input_value() {
    let iv: InputValue = graphql_input_value!({
        "fieldOne": "number one",
        "fieldTwo": "number two",
    });

    let dv = DefaultName::from_input_value(&iv);

    assert!(dv.is_ok(), "error: {}", dv.unwrap_err().message());

    let dv = dv.unwrap();

    assert_eq!(dv.field_one, "number one");
    assert_eq!(dv.field_two, "number two");
}

#[tokio::test]
async fn no_trailing_comma_introspection() {
    let doc = r#"{
        __type(name: "NoTrailingComma") {
            name
            description
            inputFields {
                name
                description
                type {
                    ofType {
                        name
                    }
                }
                defaultValue
            }
        }
    }"#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("NoTrailingComma")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&graphql_value!({
            "name": "fieldOne",
            "description": null,
            "type": {
                "ofType": {"name": "String"},
            },
            "defaultValue": null,
        })));
        assert!(fields.contains(&graphql_value!({
            "name": "fieldTwo",
            "description": null,
            "type": {
                "ofType": {"name": "String"},
            },
            "defaultValue": null,
        })));
    })
    .await;
}

#[tokio::test]
async fn derive_introspection() {
    let doc = r#"{
        __type(name: "Derive") {
            name
            description
            inputFields {
                name
                description
                type {
                    ofType {
                        name
                    }
                }
                defaultValue
            }
        }
    }"#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("Derive")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(fields.len(), 1);
        assert!(fields.contains(&graphql_value!({
            "name": "fieldOne",
            "description": null,
            "type": {
                "ofType": {"name": "String"},
            },
            "defaultValue": null,
        })));
    })
    .await;
}

#[test]
fn derive_derived() {
    assert_eq!(
        format!(
            "{:?}",
            Derive {
                field_one: "test".into(),
            },
        ),
        "Derive { field_one: \"test\" }"
    );
}

#[tokio::test]
async fn named_introspection() {
    let doc = r#"{
        __type(name: "ANamedInputObject") {
            name
            description
            inputFields {
                name
                description
                type {
                    ofType {
                        name
                    }
                }
                defaultValue
            }
        }
    }"#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("ANamedInputObject"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null))
        );

        assert_eq!(fields.len(), 1);
        assert!(fields.contains(&graphql_value!({
            "name": "fieldOne",
            "description": null,
            "type": {
                "ofType": {"name": "String"},
            },
            "defaultValue": null,
        })));
    })
    .await;
}

#[tokio::test]
async fn description_introspection() {
    let doc = r#"{
        __type(name: "Description") {
            name
            description
            inputFields {
                name
                description
                type {
                    ofType {
                        name
                    }
                }
                defaultValue
            }
        }
    }"#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("Description")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!("Description for the input object")),
        );

        assert_eq!(fields.len(), 1);
        assert!(fields.contains(&graphql_value!({
            "name": "fieldOne",
            "description": null,
            "type": {
                "ofType": {"name": "String"},
            },
            "defaultValue": null,
        })));
    })
    .await;
}

#[tokio::test]
async fn field_description_introspection() {
    let doc = r#"{
        __type(name: "FieldDescription") {
            name
            description
            inputFields {
                name
                description
                type {
                    ofType {
                        name
                    }
                }
                defaultValue
            }
        }
    }"#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("FieldDescription")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&graphql_value!({
            "name": "fieldOne",
            "description": "The first field",
            "type": {
                "ofType": {"name": "String"},
            },
            "defaultValue": null,
        })));
        assert!(fields.contains(&graphql_value!({
            "name": "fieldTwo",
            "description": "The second field",
            "type": {
                "ofType": {"name": "String"},
            },
            "defaultValue": null,
        })));
    })
    .await;
}

#[tokio::test]
async fn field_with_defaults_introspection() {
    let doc = r#"{
        __type(name: "FieldWithDefaults") {
            name
            inputFields {
                name
                type {
                    name
                    ofType {
                        name
                    }
                }
                defaultValue
            }
        }
    }"#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("FieldWithDefaults")),
        );

        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&graphql_value!({
            "name": "fieldOne",
            "type": {"name": null, "ofType": {"name": "Int"}},
            "defaultValue": "123",
        })));
        assert!(fields.contains(&graphql_value!({
            "name": "fieldTwo",
            "type": {"name": null, "ofType": {"name": "Int"}},
            "defaultValue": "456",
        })));
    })
    .await;
}
