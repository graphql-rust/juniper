use std::collections::HashSet;

use executor::Variables;
use schema::model::RootNode;
use tests::model::Database;
use types::scalars::EmptyMutation;
use value::Value;

#[test]
fn test_query_type_name() {
    let doc = r#"
        query IntrospectionQueryTypeQuery {
          __schema {
            queryType {
              name
            }
          }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "__schema",
                    Value::object(
                        vec![(
                            "queryType",
                            Value::object(
                                vec![("name", Value::string("Query"))].into_iter().collect(),
                            ),
                        )].into_iter()
                            .collect(),
                    ),
                )].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}

#[test]
fn test_specific_type_name() {
    let doc = r#"
        query IntrospectionQueryTypeQuery {
          __type(name: "Droid") {
            name
          }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "__type",
                    Value::object(vec![("name", Value::string("Droid"))].into_iter().collect()),
                )].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}

#[test]
fn test_specific_object_type_name_and_kind() {
    let doc = r#"
        query IntrospectionDroidKindQuery {
          __type(name: "Droid") {
            name
            kind
          }
        }
        "#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "__type",
                    Value::object(
                        vec![
                            ("name", Value::string("Droid")),
                            ("kind", Value::string("OBJECT")),
                        ].into_iter()
                            .collect(),
                    ),
                )].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}

#[test]
fn test_specific_interface_type_name_and_kind() {
    let doc = r#"
        query IntrospectionDroidKindQuery {
          __type(name: "Character") {
            name
            kind
          }
        }
        "#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "__type",
                    Value::object(
                        vec![
                            ("name", Value::string("Character")),
                            ("kind", Value::string("INTERFACE")),
                        ].into_iter()
                            .collect(),
                    ),
                )].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}

#[test]
fn test_documentation() {
    let doc = r#"
        query IntrospectionDroidDescriptionQuery {
          __type(name: "Droid") {
            name
            description
          }
        }
        "#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "__type",
                    Value::object(
                        vec![
                            ("name", Value::string("Droid")),
                            (
                                "description",
                                Value::string("A mechanical creature in the Star Wars universe."),
                            ),
                        ].into_iter()
                            .collect(),
                    ),
                )].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}

#[test]
fn test_possible_types() {
    let doc = r#"
        query IntrospectionDroidDescriptionQuery {
          __type(name: "Character") {
            possibleTypes {
              name
            }
          }
        }
        "#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    let result = ::execute(doc, None, &schema, &Variables::new(), &database);

    println!("Result: {:#?}", result);

    let (result, errors) = result.ok().expect("Query returned error");

    assert_eq!(errors, vec![]);

    let possible_types = result
        .as_object_value()
        .expect("execution result not an object")
        .get_field_value("__type")
        .expect("'__type' not present in result")
        .as_object_value()
        .expect("'__type' not an object")
        .get_field_value("possibleTypes")
        .expect("'possibleTypes' not present in '__type'")
        .as_list_value()
        .expect("'possibleTypes' not a list")
        .iter()
        .map(|t| {
            t.as_object_value()
                .expect("possible type not an object")
                .get_field_value("name")
                .expect("'name' not present in type")
                .as_string_value()
                .expect("'name' not a string")
        })
        .collect::<HashSet<_>>();

    assert_eq!(possible_types, vec!["Human", "Droid"].into_iter().collect());
}
