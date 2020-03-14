use crate::{
    ast::InputValue,
    executor::Variables,
    schema::model::RootNode,
    tests::{model::Database, schema::Query},
    types::scalars::EmptyMutation,
    value::Value,
};

#[tokio::test]
async fn test_hero_name() {
    let doc = r#"
        {
            hero {
                name
            }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(vec![("name", Value::scalar("R2-D2"))].into_iter().collect()),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_hero_field_order() {
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    let doc = r#"
        {
            hero {
                id
                name
            }
        }"#;
    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("id", Value::scalar("2001")),
                            ("name", Value::scalar("R2-D2")),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );

    let doc_reversed = r#"
        {
            hero {
                name
                id
            }
        }"#;
    assert_eq!(
        crate::execute(doc_reversed, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("name", Value::scalar("R2-D2")),
                            ("id", Value::scalar("2001")),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_hero_name_and_friends() {
    let doc = r#"
        {
            hero {
                id
                name
                friends {
                    name
                }
            }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("id", Value::scalar("2001")),
                            ("name", Value::scalar("R2-D2")),
                            (
                                "friends",
                                Value::list(vec![
                                    Value::object(
                                        vec![("name", Value::scalar("Luke Skywalker"))]
                                            .into_iter()
                                            .collect(),
                                    ),
                                    Value::object(
                                        vec![("name", Value::scalar("Han Solo"))]
                                            .into_iter()
                                            .collect(),
                                    ),
                                    Value::object(
                                        vec![("name", Value::scalar("Leia Organa"))]
                                            .into_iter()
                                            .collect(),
                                    ),
                                ]),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_hero_name_and_friends_and_friends_of_friends() {
    let doc = r#"
        {
            hero {
                id
                name
                friends {
                    name
                    appearsIn
                    friends {
                        name
                    }
                }
            }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("id", Value::scalar("2001")),
                            ("name", Value::scalar("R2-D2")),
                            (
                                "friends",
                                Value::list(vec![
                                    Value::object(
                                        vec![
                                            ("name", Value::scalar("Luke Skywalker")),
                                            (
                                                "appearsIn",
                                                Value::list(vec![
                                                    Value::scalar("NEW_HOPE"),
                                                    Value::scalar("EMPIRE"),
                                                    Value::scalar("JEDI"),
                                                ]),
                                            ),
                                            (
                                                "friends",
                                                Value::list(vec![
                                                    Value::object(
                                                        vec![("name", Value::scalar("Han Solo"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![(
                                                            "name",
                                                            Value::scalar("Leia Organa"),
                                                        )]
                                                        .into_iter()
                                                        .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::scalar("C-3PO"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::scalar("R2-D2"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                ]),
                                            ),
                                        ]
                                        .into_iter()
                                        .collect(),
                                    ),
                                    Value::object(
                                        vec![
                                            ("name", Value::scalar("Han Solo")),
                                            (
                                                "appearsIn",
                                                Value::list(vec![
                                                    Value::scalar("NEW_HOPE"),
                                                    Value::scalar("EMPIRE"),
                                                    Value::scalar("JEDI"),
                                                ]),
                                            ),
                                            (
                                                "friends",
                                                Value::list(vec![
                                                    Value::object(
                                                        vec![(
                                                            "name",
                                                            Value::scalar("Luke Skywalker"),
                                                        )]
                                                        .into_iter()
                                                        .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![(
                                                            "name",
                                                            Value::scalar("Leia Organa"),
                                                        )]
                                                        .into_iter()
                                                        .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::scalar("R2-D2"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                ]),
                                            ),
                                        ]
                                        .into_iter()
                                        .collect(),
                                    ),
                                    Value::object(
                                        vec![
                                            ("name", Value::scalar("Leia Organa")),
                                            (
                                                "appearsIn",
                                                Value::list(vec![
                                                    Value::scalar("NEW_HOPE"),
                                                    Value::scalar("EMPIRE"),
                                                    Value::scalar("JEDI"),
                                                ]),
                                            ),
                                            (
                                                "friends",
                                                Value::list(vec![
                                                    Value::object(
                                                        vec![(
                                                            "name",
                                                            Value::scalar("Luke Skywalker"),
                                                        )]
                                                        .into_iter()
                                                        .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::scalar("Han Solo"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::scalar("C-3PO"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::scalar("R2-D2"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                ]),
                                            ),
                                        ]
                                        .into_iter()
                                        .collect(),
                                    ),
                                ]),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_query_name() {
    let doc = r#"{ human(id: "1000") { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "human",
                    Value::object(
                        vec![("name", Value::scalar("Luke Skywalker"))]
                            .into_iter()
                            .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_query_alias_single() {
    let doc = r#"{ luke: human(id: "1000") { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "luke",
                    Value::object(
                        vec![("name", Value::scalar("Luke Skywalker"))]
                            .into_iter()
                            .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_query_alias_multiple() {
    let doc = r#"
        {
            luke: human(id: "1000") { name }
            leia: human(id: "1003") { name }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![
                    (
                        "luke",
                        Value::object(
                            vec![("name", Value::scalar("Luke Skywalker"))]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                    (
                        "leia",
                        Value::object(
                            vec![("name", Value::scalar("Leia Organa"))]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_query_alias_multiple_with_fragment() {
    let doc = r#"
        query UseFragment {
            luke: human(id: "1000") { ...HumanFragment }
            leia: human(id: "1003") { ...HumanFragment }
        }

        fragment HumanFragment on Human {
            name
            homePlanet
        }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![
                    (
                        "luke",
                        Value::object(
                            vec![
                                ("name", Value::scalar("Luke Skywalker")),
                                ("homePlanet", Value::scalar("Tatooine")),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    ),
                    (
                        "leia",
                        Value::object(
                            vec![
                                ("name", Value::scalar("Leia Organa")),
                                ("homePlanet", Value::scalar("Alderaan")),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    ),
                ]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_query_name_variable() {
    let doc = r#"query FetchSomeIDQuery($someId: String!) { human(id: $someId) { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    let vars = vec![("someId".to_owned(), InputValue::scalar("1000"))]
        .into_iter()
        .collect();

    assert_eq!(
        crate::execute(doc, None, &schema, &vars, &database).await,
        Ok((
            Value::object(
                vec![(
                    "human",
                    Value::object(
                        vec![("name", Value::scalar("Luke Skywalker"))]
                            .into_iter()
                            .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_query_name_invalid_variable() {
    let doc = r#"query FetchSomeIDQuery($someId: String!) { human(id: $someId) { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    let vars = vec![("someId".to_owned(), InputValue::scalar("some invalid id"))]
        .into_iter()
        .collect();

    assert_eq!(
        crate::execute(doc, None, &schema, &vars, &database).await,
        Ok((
            Value::object(vec![("human", Value::null())].into_iter().collect()),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_query_friends_names() {
    let doc = r#"{ human(id: "1000") { friends { name } } }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "human",
                    Value::object(
                        vec![(
                            "friends",
                            Value::list(vec![
                                Value::object(
                                    vec![("name", Value::scalar("Han Solo"))]
                                        .into_iter()
                                        .collect(),
                                ),
                                Value::object(
                                    vec![("name", Value::scalar("Leia Organa"))]
                                        .into_iter()
                                        .collect(),
                                ),
                                Value::object(
                                    vec![("name", Value::scalar("C-3PO"))].into_iter().collect(),
                                ),
                                Value::object(
                                    vec![("name", Value::scalar("R2-D2"))].into_iter().collect(),
                                ),
                            ]),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_query_inline_fragments_droid() {
    let doc = r#"
        query InlineFragments {
            hero {
                name
                __typename

                ...on Droid {
                    primaryFunction
                }
            }
        }
        "#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("name", Value::scalar("R2-D2")),
                            ("__typename", Value::scalar("Droid")),
                            ("primaryFunction", Value::scalar("Astromech")),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_query_inline_fragments_human() {
    let doc = r#"
        query InlineFragments {
            hero(episode: EMPIRE) {
                name
                __typename
            }
        }
        "#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("name", Value::scalar("Luke Skywalker")),
                            ("__typename", Value::scalar("Human")),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_object_typename() {
    let doc = r#"
        {
            human(id: "1000") {
                __typename
            }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(Query, EmptyMutation::<Database>::new());

    assert_eq!(
        crate::execute(doc, None, &schema, &Variables::new(), &database).await,
        Ok((
            Value::object(
                vec![(
                    "human",
                    Value::object(
                        vec![("__typename", Value::scalar("Human"))]
                            .into_iter()
                            .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}
