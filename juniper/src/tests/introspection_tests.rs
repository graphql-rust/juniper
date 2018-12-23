use std::collections::HashSet;

use executor::Variables;
use schema::model::RootNode;
use tests::model::Database;
use types::scalars::EmptyMutation;
use value::Value;

#[test]
fn test_introspection_query_type_name() {
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
            graphql_value!({
                "__schema": {
                    "queryType": {
                        "name": "Query"
                    }
                }

            }),
            vec![]
        ))
    );
}

#[test]
fn test_introspection_type_name() {
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
            graphql_value!({
                "__type": {
                    "name": "Droid",
                },
            }),
            vec![]
        ))
    );
}

#[test]
fn test_introspection_specific_object_type_name_and_kind() {
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
            graphql_value!({
                "__type": {
                    "name": "Droid",
                    "kind": "OBJECT",
                }
            }),
            vec![],
        ))
    );
}

#[test]
fn test_introspection_specific_interface_type_name_and_kind() {
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
            graphql_value!({
                "__type": {
                    "name": "Character",
                    "kind": "INTERFACE",
                }
            }),
            vec![]
        ))
    );
}

#[test]
fn test_introspection_documentation() {
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
            graphql_value!({
                "__type": {
                    "name": "Droid",
                    "description": "A mechanical creature in the Star Wars universe.",
                },
            }),
            vec![]
        ))
    );
}

#[test]
fn test_introspection_possible_types() {
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
                .as_scalar_value::<String>()
                .expect("'name' not a string") as &str
        })
        .collect::<HashSet<_>>();

    assert_eq!(possible_types, vec!["Human", "Droid"].into_iter().collect());
}
