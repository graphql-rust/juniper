#![deny(unused_variables)]

use juniper_codegen::GraphQLInputObjectInternal as GraphQLInputObject;

use crate::{
    ast::{FromInputValue, InputValue},
    executor::Variables,
    schema::model::RootNode,
    types::scalars::EmptyMutation,
    value::{DefaultScalarValue, Object, Value},
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
    #[graphql(default = "123")]
    field_one: i32,
    #[graphql(default = "456", description = "The second field")]
    field_two: i32,
}

#[crate::graphql_object_internal]
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
    F: Fn(&Object<DefaultScalarValue>, &Vec<Value<DefaultScalarValue>>) -> (),
{
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) = crate::execute(doc, None, &schema, &Variables::new(), &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:#?}", result);

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
    let doc = r#"
    {
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
    }
    "#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::scalar("DefaultName"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::null())
        );

        assert_eq!(fields.len(), 2);

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldOne")),
                ("description", Value::null()),
                (
                    "type",
                    Value::object(
                        vec![(
                            "ofType",
                            Value::object(
                                vec![("name", Value::scalar("String"))]
                                    .into_iter()
                                    .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("defaultValue", Value::null()),
            ]
            .into_iter()
            .collect(),
        )));

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldTwo")),
                ("description", Value::null()),
                (
                    "type",
                    Value::object(
                        vec![(
                            "ofType",
                            Value::object(
                                vec![("name", Value::scalar("String"))]
                                    .into_iter()
                                    .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("defaultValue", Value::null()),
            ]
            .into_iter()
            .collect(),
        )));
    })
    .await;
}

#[test]
fn default_name_input_value() {
    let iv: InputValue<DefaultScalarValue> = InputValue::object(
        vec![
            ("fieldOne", InputValue::scalar("number one")),
            ("fieldTwo", InputValue::scalar("number two")),
        ]
        .into_iter()
        .collect(),
    );

    let dv: Option<DefaultName> = FromInputValue::from_input_value(&iv);

    assert!(dv.is_some());

    let dv = dv.unwrap();

    assert_eq!(dv.field_one, "number one");
    assert_eq!(dv.field_two, "number two");
}

#[tokio::test]
async fn no_trailing_comma_introspection() {
    let doc = r#"
    {
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
    }
    "#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::scalar("NoTrailingComma"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::null())
        );

        assert_eq!(fields.len(), 2);

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldOne")),
                ("description", Value::null()),
                (
                    "type",
                    Value::object(
                        vec![(
                            "ofType",
                            Value::object(
                                vec![("name", Value::scalar("String"))]
                                    .into_iter()
                                    .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("defaultValue", Value::null()),
            ]
            .into_iter()
            .collect(),
        )));

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldTwo")),
                ("description", Value::null()),
                (
                    "type",
                    Value::object(
                        vec![(
                            "ofType",
                            Value::object(
                                vec![("name", Value::scalar("String"))]
                                    .into_iter()
                                    .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("defaultValue", Value::null()),
            ]
            .into_iter()
            .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn derive_introspection() {
    let doc = r#"
    {
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
    }
    "#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::scalar("Derive"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::null())
        );

        assert_eq!(fields.len(), 1);

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldOne")),
                ("description", Value::null()),
                (
                    "type",
                    Value::object(
                        vec![(
                            "ofType",
                            Value::object(
                                vec![("name", Value::scalar("String"))]
                                    .into_iter()
                                    .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("defaultValue", Value::null()),
            ]
            .into_iter()
            .collect(),
        )));
    })
    .await;
}

#[test]
fn derive_derived() {
    assert_eq!(
        format!(
            "{:?}",
            Derive {
                field_one: "test".to_owned(),
            }
        ),
        "Derive { field_one: \"test\" }"
    );
}

#[tokio::test]
async fn named_introspection() {
    let doc = r#"
    {
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
    }
    "#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::scalar("ANamedInputObject"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::null())
        );

        assert_eq!(fields.len(), 1);

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldOne")),
                ("description", Value::null()),
                (
                    "type",
                    Value::object(
                        vec![(
                            "ofType",
                            Value::object(
                                vec![("name", Value::scalar("String"))]
                                    .into_iter()
                                    .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("defaultValue", Value::null()),
            ]
            .into_iter()
            .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn description_introspection() {
    let doc = r#"
    {
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
    }
    "#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::scalar("Description"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::scalar("Description for the input object"))
        );

        assert_eq!(fields.len(), 1);

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldOne")),
                ("description", Value::null()),
                (
                    "type",
                    Value::object(
                        vec![(
                            "ofType",
                            Value::object(
                                vec![("name", Value::scalar("String"))]
                                    .into_iter()
                                    .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("defaultValue", Value::null()),
            ]
            .into_iter()
            .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn field_description_introspection() {
    let doc = r#"
    {
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
    }
    "#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::scalar("FieldDescription"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::null())
        );

        assert_eq!(fields.len(), 2);

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldOne")),
                ("description", Value::scalar("The first field")),
                (
                    "type",
                    Value::object(
                        vec![(
                            "ofType",
                            Value::object(
                                vec![("name", Value::scalar("String"))]
                                    .into_iter()
                                    .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("defaultValue", Value::null()),
            ]
            .into_iter()
            .collect(),
        )));

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldTwo")),
                ("description", Value::scalar("The second field")),
                (
                    "type",
                    Value::object(
                        vec![(
                            "ofType",
                            Value::object(
                                vec![("name", Value::scalar("String"))]
                                    .into_iter()
                                    .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("defaultValue", Value::null()),
            ]
            .into_iter()
            .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn field_with_defaults_introspection() {
    let doc = r#"
    {
        __type(name: "FieldWithDefaults") {
            name
            inputFields {
                name
                type {
                    name
                }
                defaultValue
            }
        }
    }
    "#;

    run_type_info_query(doc, |type_info, fields| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::scalar("FieldWithDefaults"))
        );

        assert_eq!(fields.len(), 2);

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldOne")),
                (
                    "type",
                    Value::object(vec![("name", Value::scalar("Int"))].into_iter().collect()),
                ),
                ("defaultValue", Value::scalar("123")),
            ]
            .into_iter()
            .collect(),
        )));

        assert!(fields.contains(&Value::object(
            vec![
                ("name", Value::scalar("fieldTwo")),
                (
                    "type",
                    Value::object(vec![("name", Value::scalar("Int"))].into_iter().collect()),
                ),
                ("defaultValue", Value::scalar("456")),
            ]
            .into_iter()
            .collect(),
        )));
    })
    .await;
}
