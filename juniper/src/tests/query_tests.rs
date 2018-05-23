use ast::InputValue;
use executor::Variables;
use schema::model::RootNode;
use tests::model::Database;
use types::scalars::EmptyMutation;
use value::Value;

#[test]
fn test_hero_name() {
    let doc = r#"
        {
            hero {
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
                    "hero",
                    Value::object(vec![("name", Value::string("R2-D2"))].into_iter().collect()),
                )].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}

#[test]
fn test_hero_field_order() {
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    let doc = r#"
        {
            hero {
                id
                name
            }
        }"#;
    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("id", Value::string("2001")),
                            ("name", Value::string("R2-D2")),
                        ].into_iter()
                            .collect(),
                    ),
                )].into_iter()
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
        ::execute(doc_reversed, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("name", Value::string("R2-D2")),
                            ("id", Value::string("2001")),
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
fn test_hero_name_and_friends() {
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
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("id", Value::string("2001")),
                            ("name", Value::string("R2-D2")),
                            (
                                "friends",
                                Value::list(vec![
                                    Value::object(
                                        vec![("name", Value::string("Luke Skywalker"))]
                                            .into_iter()
                                            .collect(),
                                    ),
                                    Value::object(
                                        vec![("name", Value::string("Han Solo"))]
                                            .into_iter()
                                            .collect(),
                                    ),
                                    Value::object(
                                        vec![("name", Value::string("Leia Organa"))]
                                            .into_iter()
                                            .collect(),
                                    ),
                                ]),
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
fn test_hero_name_and_friends_and_friends_of_friends() {
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
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("id", Value::string("2001")),
                            ("name", Value::string("R2-D2")),
                            (
                                "friends",
                                Value::list(vec![
                                    Value::object(
                                        vec![
                                            ("name", Value::string("Luke Skywalker")),
                                            (
                                                "appearsIn",
                                                Value::list(vec![
                                                    Value::string("NEW_HOPE"),
                                                    Value::string("EMPIRE"),
                                                    Value::string("JEDI"),
                                                ]),
                                            ),
                                            (
                                                "friends",
                                                Value::list(vec![
                                                    Value::object(
                                                        vec![("name", Value::string("Han Solo"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![(
                                                            "name",
                                                            Value::string("Leia Organa"),
                                                        )].into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::string("C-3PO"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::string("R2-D2"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                ]),
                                            ),
                                        ].into_iter()
                                            .collect(),
                                    ),
                                    Value::object(
                                        vec![
                                            ("name", Value::string("Han Solo")),
                                            (
                                                "appearsIn",
                                                Value::list(vec![
                                                    Value::string("NEW_HOPE"),
                                                    Value::string("EMPIRE"),
                                                    Value::string("JEDI"),
                                                ]),
                                            ),
                                            (
                                                "friends",
                                                Value::list(vec![
                                                    Value::object(
                                                        vec![(
                                                            "name",
                                                            Value::string("Luke Skywalker"),
                                                        )].into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![(
                                                            "name",
                                                            Value::string("Leia Organa"),
                                                        )].into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::string("R2-D2"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                ]),
                                            ),
                                        ].into_iter()
                                            .collect(),
                                    ),
                                    Value::object(
                                        vec![
                                            ("name", Value::string("Leia Organa")),
                                            (
                                                "appearsIn",
                                                Value::list(vec![
                                                    Value::string("NEW_HOPE"),
                                                    Value::string("EMPIRE"),
                                                    Value::string("JEDI"),
                                                ]),
                                            ),
                                            (
                                                "friends",
                                                Value::list(vec![
                                                    Value::object(
                                                        vec![(
                                                            "name",
                                                            Value::string("Luke Skywalker"),
                                                        )].into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::string("Han Solo"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::string("C-3PO"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                    Value::object(
                                                        vec![("name", Value::string("R2-D2"))]
                                                            .into_iter()
                                                            .collect(),
                                                    ),
                                                ]),
                                            ),
                                        ].into_iter()
                                            .collect(),
                                    ),
                                ]),
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
fn test_query_name() {
    let doc = r#"{ human(id: "1000") { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "human",
                    Value::object(
                        vec![("name", Value::string("Luke Skywalker"))]
                            .into_iter()
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
fn test_query_alias_single() {
    let doc = r#"{ luke: human(id: "1000") { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "luke",
                    Value::object(
                        vec![("name", Value::string("Luke Skywalker"))]
                            .into_iter()
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
fn test_query_alias_multiple() {
    let doc = r#"
        {
            luke: human(id: "1000") { name }
            leia: human(id: "1003") { name }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![
                    (
                        "luke",
                        Value::object(
                            vec![("name", Value::string("Luke Skywalker"))]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                    (
                        "leia",
                        Value::object(
                            vec![("name", Value::string("Leia Organa"))]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}

#[test]
fn test_query_alias_multiple_with_fragment() {
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
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![
                    (
                        "luke",
                        Value::object(
                            vec![
                                ("name", Value::string("Luke Skywalker")),
                                ("homePlanet", Value::string("Tatooine")),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                    (
                        "leia",
                        Value::object(
                            vec![
                                ("name", Value::string("Leia Organa")),
                                ("homePlanet", Value::string("Alderaan")),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}

#[test]
fn test_query_name_variable() {
    let doc = r#"query FetchSomeIDQuery($someId: String!) { human(id: $someId) { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    let vars = vec![("someId".to_owned(), InputValue::string("1000"))]
        .into_iter()
        .collect();

    assert_eq!(
        ::execute(doc, None, &schema, &vars, &database),
        Ok((
            Value::object(
                vec![(
                    "human",
                    Value::object(
                        vec![("name", Value::string("Luke Skywalker"))]
                            .into_iter()
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
fn test_query_name_invalid_variable() {
    let doc = r#"query FetchSomeIDQuery($someId: String!) { human(id: $someId) { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    let vars = vec![("someId".to_owned(), InputValue::string("some invalid id"))]
        .into_iter()
        .collect();

    assert_eq!(
        ::execute(doc, None, &schema, &vars, &database),
        Ok((
            Value::object(vec![("human", Value::null())].into_iter().collect()),
            vec![]
        ))
    );
}

#[test]
fn test_query_friends_names() {
    let doc = r#"{ human(id: "1000") { friends { name } } }"#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "human",
                    Value::object(
                        vec![(
                            "friends",
                            Value::list(vec![
                                Value::object(
                                    vec![("name", Value::string("Han Solo"))]
                                        .into_iter()
                                        .collect(),
                                ),
                                Value::object(
                                    vec![("name", Value::string("Leia Organa"))]
                                        .into_iter()
                                        .collect(),
                                ),
                                Value::object(
                                    vec![("name", Value::string("C-3PO"))].into_iter().collect(),
                                ),
                                Value::object(
                                    vec![("name", Value::string("R2-D2"))].into_iter().collect(),
                                ),
                            ]),
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
fn test_query_inline_fragments_droid() {
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
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "hero",
                    Value::object(
                        vec![
                            ("name", Value::string("R2-D2")),
                            ("__typename", Value::string("Droid")),
                            ("primaryFunction", Value::string("Astromech")),
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
fn test_query_inline_fragments_human() {
    let doc = r#"
        query InlineFragments {
            hero(episode: EMPIRE) {
                name
                __typename
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
                    "hero",
                    Value::object(
                        vec![
                            ("name", Value::string("Luke Skywalker")),
                            ("__typename", Value::string("Human")),
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
fn test_object_typename() {
    let doc = r#"
        {
            human(id: "1000") {
                __typename
            }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &database),
        Ok((
            Value::object(
                vec![(
                    "human",
                    Value::object(
                        vec![("__typename", Value::string("Human"))]
                            .into_iter()
                            .collect(),
                    ),
                )].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}
