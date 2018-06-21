use ast::{FromInputValue, InputValue};
use executor::Variables;
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use value::{Value, Object};

struct Root;

#[derive(GraphQLInputObject)]
#[graphql(_internal)]
struct DefaultName {
    field_one: String,
    field_two: String,
}

#[derive(GraphQLInputObject)]
#[graphql(_internal)]
struct NoTrailingComma {
    field_one: String,
    field_two: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(_internal)]
struct Derive {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(name = "ANamedInputObject", _internal)]
struct Named {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(description = "Description for the input object", _internal)]
struct Description {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(_internal)]
pub struct Public {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(description = "Description for the input object", _internal)]
pub struct PublicWithDescription {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(
    name = "APublicNamedInputObjectWithDescription",
    description = "Description for the input object",
    _internal
)]
pub struct NamedPublicWithDescription {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(name = "APublicNamedInputObject", _internal)]
pub struct NamedPublic {
    field_one: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(_internal)]
struct FieldDescription {
    #[graphql(description = "The first field")]
    field_one: String,
    #[graphql(description = "The second field")]
    field_two: String,
}

#[derive(GraphQLInputObject, Debug)]
#[graphql(_internal)]
struct FieldWithDefaults {
    #[graphql(default = "123")]
    field_one: i32,
    #[graphql(default = "456", description = "The second field")]
    field_two: i32,
}

graphql_object!(Root: () |&self| {
    field test_field(
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
        0
    }
});

fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn(&Object, &Vec<Value>) -> (),
{
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

    let fields = type_info
        .get_field_value("inputFields")
        .expect("inputFields field missing")
        .as_list_value()
        .expect("inputFields not a list");

    f(type_info, fields);
}

#[test]
fn default_name_introspection() {
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
        assert_eq!(type_info.get_field_value("name"), Some(&Value::string("DefaultName")));
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 2);

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldOne")),
                    ("description", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![(
                                "ofType",
                                Value::object(
                                    vec![("name", Value::string("String"))]
                                        .into_iter()
                                        .collect(),
                                ),
                            )].into_iter()
                                .collect(),
                        ),
                    ),
                    ("defaultValue", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldTwo")),
                    ("description", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![(
                                "ofType",
                                Value::object(
                                    vec![("name", Value::string("String"))]
                                        .into_iter()
                                        .collect(),
                                ),
                            )].into_iter()
                                .collect(),
                        ),
                    ),
                    ("defaultValue", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn default_name_input_value() {
    let iv = InputValue::object(
        vec![
            ("fieldOne", InputValue::string("number one")),
            ("fieldTwo", InputValue::string("number two")),
        ].into_iter()
            .collect(),
    );

    let dv: Option<DefaultName> = FromInputValue::from_input_value(&iv);

    assert!(dv.is_some());

    let dv = dv.unwrap();

    assert_eq!(dv.field_one, "number one");
    assert_eq!(dv.field_two, "number two");
}

#[test]
fn no_trailing_comma_introspection() {
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
            Some(&Value::string("NoTrailingComma"))
        );
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 2);

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldOne")),
                    ("description", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![(
                                "ofType",
                                Value::object(
                                    vec![("name", Value::string("String"))]
                                        .into_iter()
                                        .collect(),
                                ),
                            )].into_iter()
                                .collect(),
                        ),
                    ),
                    ("defaultValue", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldTwo")),
                    ("description", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![(
                                "ofType",
                                Value::object(
                                    vec![("name", Value::string("String"))]
                                        .into_iter()
                                        .collect(),
                                ),
                            )].into_iter()
                                .collect(),
                        ),
                    ),
                    ("defaultValue", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn derive_introspection() {
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
        assert_eq!(type_info.get_field_value("name"), Some(&Value::string("Derive")));
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 1);

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldOne")),
                    ("description", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![(
                                "ofType",
                                Value::object(
                                    vec![("name", Value::string("String"))]
                                        .into_iter()
                                        .collect(),
                                ),
                            )].into_iter()
                                .collect(),
                        ),
                    ),
                    ("defaultValue", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
    });
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

#[test]
fn named_introspection() {
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
            Some(&Value::string("ANamedInputObject"))
        );
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 1);

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldOne")),
                    ("description", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![(
                                "ofType",
                                Value::object(
                                    vec![("name", Value::string("String"))]
                                        .into_iter()
                                        .collect(),
                                ),
                            )].into_iter()
                                .collect(),
                        ),
                    ),
                    ("defaultValue", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn description_introspection() {
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
        assert_eq!(type_info.get_field_value("name"), Some(&Value::string("Description")));
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::string("Description for the input object"))
        );

        assert_eq!(fields.len(), 1);

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldOne")),
                    ("description", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![(
                                "ofType",
                                Value::object(
                                    vec![("name", Value::string("String"))]
                                        .into_iter()
                                        .collect(),
                                ),
                            )].into_iter()
                                .collect(),
                        ),
                    ),
                    ("defaultValue", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn field_description_introspection() {
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
            Some(&Value::string("FieldDescription"))
        );
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 2);

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldOne")),
                    ("description", Value::string("The first field")),
                    (
                        "type",
                        Value::object(
                            vec![(
                                "ofType",
                                Value::object(
                                    vec![("name", Value::string("String"))]
                                        .into_iter()
                                        .collect(),
                                ),
                            )].into_iter()
                                .collect(),
                        ),
                    ),
                    ("defaultValue", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldTwo")),
                    ("description", Value::string("The second field")),
                    (
                        "type",
                        Value::object(
                            vec![(
                                "ofType",
                                Value::object(
                                    vec![("name", Value::string("String"))]
                                        .into_iter()
                                        .collect(),
                                ),
                            )].into_iter()
                                .collect(),
                        ),
                    ),
                    ("defaultValue", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn field_with_defaults_introspection() {
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
            Some(&Value::string("FieldWithDefaults"))
        );

        assert_eq!(fields.len(), 2);

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldOne")),
                    (
                        "type",
                        Value::object(vec![("name", Value::string("Int"))].into_iter().collect()),
                    ),
                    ("defaultValue", Value::string("123")),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            fields.contains(&Value::object(
                vec![
                    ("name", Value::string("fieldTwo")),
                    (
                        "type",
                        Value::object(vec![("name", Value::string("Int"))].into_iter().collect()),
                    ),
                    ("defaultValue", Value::string("456")),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}
