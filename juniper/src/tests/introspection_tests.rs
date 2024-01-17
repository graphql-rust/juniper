use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    graphql_vars,
    introspection::IntrospectionFormat,
    schema::model::RootNode,
    tests::fixtures::starwars::schema::{Database, Query},
    types::scalars::{EmptyMutation, EmptySubscription},
};

use super::schema_introspection::*;

#[tokio::test]
async fn test_introspection_query_type_name() {
    let doc = r#"
        query IntrospectionQueryTypeQuery {
          __schema {
            queryType {
              name
            }
          }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
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

#[tokio::test]
async fn test_introspection_type_name() {
    let doc = r#"
        query IntrospectionQueryTypeQuery {
          __type(name: "Droid") {
            name
          }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
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

#[tokio::test]
async fn test_introspection_specific_object_type_name_and_kind() {
    let doc = r#"
        query IntrospectionDroidKindQuery {
          __type(name: "Droid") {
            name
            kind
          }
        }
        "#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
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

#[tokio::test]
async fn test_introspection_specific_interface_type_name_and_kind() {
    let doc = r#"
        query IntrospectionDroidKindQuery {
          __type(name: "Character") {
            name
            kind
          }
        }
        "#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
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

#[tokio::test]
async fn test_introspection_documentation() {
    let doc = r#"
        query IntrospectionDroidDescriptionQuery {
          __type(name: "Droid") {
            name
            description
          }
        }
        "#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
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

#[tokio::test]
async fn test_introspection_directives() {
    let q = r#"
        query IntrospectionQuery {
          __schema {
            directives {
              name
              locations
            }
          }
        }
    "#;

    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    let result = crate::execute(q, None, &schema, &graphql_vars! {}, &database)
        .await
        .unwrap();

    let expected = graphql_value!({
        "__schema": {
            "directives": [
                {
                    "name": "deprecated",
                    "locations": [
                        "FIELD_DEFINITION",
                        "ENUM_VALUE",
                    ],
                },
                {
                    "name": "include",
                    "locations": [
                        "FIELD",
                        "FRAGMENT_SPREAD",
                        "INLINE_FRAGMENT",
                    ],
                },
                {
                    "name": "skip",
                    "locations": [
                        "FIELD",
                        "FRAGMENT_SPREAD",
                        "INLINE_FRAGMENT",
                    ],
                },
                {
                    "name": "specifiedBy",
                    "locations": [
                        "SCALAR",
                    ],
                },
            ],
        },
    });

    assert_eq!(result, (expected, vec![]));
}

#[tokio::test]
async fn test_introspection_possible_types() {
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
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    let result = crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await;

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

#[tokio::test]
async fn test_builtin_introspection_query() {
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );
    let result = crate::introspect(&schema, &database, IntrospectionFormat::default()).unwrap();
    let expected = schema_introspection_result();

    assert_eq!(result, (expected, vec![]));
}

#[tokio::test]
async fn test_builtin_introspection_query_without_descriptions() {
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    let result =
        crate::introspect(&schema, &database, IntrospectionFormat::WithoutDescriptions).unwrap();
    let expected = schema_introspection_result_without_descriptions();

    assert_eq!(result, (expected, vec![]));
}
