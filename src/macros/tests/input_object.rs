use std::collections::HashMap;

use ast::{InputValue, FromInputValue};
use value::Value;
use schema::model::RootNode;

struct Root;

graphql_input_object!(
    struct DefaultName {
        field_one: String,
        field_two: String,
    }
);

graphql_input_object!(
    struct NoTrailingComma {
        field_one: String,
        field_two: String
    }
);

graphql_input_object!(
    #[derive(Debug)]
    struct Derive {
        field_one: String,
    }
);

graphql_input_object!(
    struct Named as "ANamedInputObject" {
        field_one: String,
    }
);

graphql_input_object!(
    description: "Description for the input object"

    struct Description {
        field_one: String,
    }
);

graphql_input_object!(
    struct FieldDescription {
        field_one: String as "The first field",
        field_two: String as "The second field",
    }
);

graphql_object!(Root: () |&self| {
    field test_field(
        a1: DefaultName,
        a2: NoTrailingComma,
        a3: Derive,
        a4: Named,
        a5: Description,
        a6: FieldDescription
    ) -> i64 {
        0
    }
});

fn run_type_info_query<F>(doc: &str, f: F) where F: Fn(&HashMap<String, Value>, &Vec<Value>) -> () {
    let schema = RootNode::new(Root {}, ());

    let (result, errs) = ::execute(doc, None, &schema, &HashMap::new(), &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value().expect("Result is not an object")
        .get("__type").expect("__type field missing")
        .as_object_value().expect("__type field not an object value");

    let fields = type_info
        .get("inputFields").expect("inputFields field missing")
        .as_list_value().expect("inputFields not a list");

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
        assert_eq!(type_info.get("name"), Some(&Value::string("DefaultName")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 2);

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("fieldOne")),
            ("description", Value::null()),
            ("type", Value::object(vec![
                ("ofType", Value::object(vec![
                    ("name", Value::string("String")),
                ].into_iter().collect())),
            ].into_iter().collect())),
            ("defaultValue", Value::null()),
        ].into_iter().collect())));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("fieldTwo")),
            ("description", Value::null()),
            ("type", Value::object(vec![
                ("ofType", Value::object(vec![
                    ("name", Value::string("String")),
                ].into_iter().collect())),
            ].into_iter().collect())),
            ("defaultValue", Value::null()),
        ].into_iter().collect())));
    });
}

#[test]
fn default_name_input_value() {
    let iv = InputValue::object(vec![
        ("fieldOne", InputValue::string("number one")),
        ("fieldTwo", InputValue::string("number two")),
    ].into_iter().collect());

    let dv: Option<DefaultName> = FromInputValue::from(&iv);

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
        assert_eq!(type_info.get("name"), Some(&Value::string("NoTrailingComma")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 2);

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("fieldOne")),
            ("description", Value::null()),
            ("type", Value::object(vec![
                ("ofType", Value::object(vec![
                    ("name", Value::string("String")),
                ].into_iter().collect())),
            ].into_iter().collect())),
            ("defaultValue", Value::null()),
        ].into_iter().collect())));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("fieldTwo")),
            ("description", Value::null()),
            ("type", Value::object(vec![
                ("ofType", Value::object(vec![
                    ("name", Value::string("String")),
                ].into_iter().collect())),
            ].into_iter().collect())),
            ("defaultValue", Value::null()),
        ].into_iter().collect())));
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
        assert_eq!(type_info.get("name"), Some(&Value::string("Derive")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 1);

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("fieldOne")),
            ("description", Value::null()),
            ("type", Value::object(vec![
                ("ofType", Value::object(vec![
                    ("name", Value::string("String")),
                ].into_iter().collect())),
            ].into_iter().collect())),
            ("defaultValue", Value::null()),
        ].into_iter().collect())));
    });
}

#[test]
fn derive_derived() {
    assert_eq!(
        format!("{:?}", Derive { field_one: "test".to_owned() }),
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
        assert_eq!(type_info.get("name"), Some(&Value::string("ANamedInputObject")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 1);

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("fieldOne")),
            ("description", Value::null()),
            ("type", Value::object(vec![
                ("ofType", Value::object(vec![
                    ("name", Value::string("String")),
                ].into_iter().collect())),
            ].into_iter().collect())),
            ("defaultValue", Value::null()),
        ].into_iter().collect())));
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
        assert_eq!(type_info.get("name"), Some(&Value::string("Description")));
        assert_eq!(type_info.get("description"), Some(&Value::string("Description for the input object")));

        assert_eq!(fields.len(), 1);

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("fieldOne")),
            ("description", Value::null()),
            ("type", Value::object(vec![
                ("ofType", Value::object(vec![
                    ("name", Value::string("String")),
                ].into_iter().collect())),
            ].into_iter().collect())),
            ("defaultValue", Value::null()),
        ].into_iter().collect())));
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
        assert_eq!(type_info.get("name"), Some(&Value::string("FieldDescription")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(fields.len(), 2);

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("fieldOne")),
            ("description", Value::string("The first field")),
            ("type", Value::object(vec![
                ("ofType", Value::object(vec![
                    ("name", Value::string("String")),
                ].into_iter().collect())),
            ].into_iter().collect())),
            ("defaultValue", Value::null()),
        ].into_iter().collect())));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("fieldTwo")),
            ("description", Value::string("The second field")),
            ("type", Value::object(vec![
                ("ofType", Value::object(vec![
                    ("name", Value::string("String")),
                ].into_iter().collect())),
            ].into_iter().collect())),
            ("defaultValue", Value::null()),
        ].into_iter().collect())));
    });
}
