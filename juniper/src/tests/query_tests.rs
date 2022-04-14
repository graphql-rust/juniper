use crate::{
    graphql_value, graphql_vars,
    schema::model::RootNode,
    tests::fixtures::starwars::schema::{Database, Query},
    types::scalars::{EmptyMutation, EmptySubscription},
};

#[tokio::test]
async fn test_hero_name() {
    let doc = r#"{
        hero {
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
        Ok((graphql_value!({"hero": {"name": "R2-D2"}}), vec![])),
    );
}

#[tokio::test]
async fn test_hero_field_order() {
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    let doc = r#"{
        hero {
            id
            name
        }
    }"#;
    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
        Ok((
            graphql_value!({"hero": {"id": "2001", "name": "R2-D2"}}),
            vec![],
        )),
    );

    let doc_reversed = r#"{
        hero {
            name
            id
        }
    }"#;
    assert_eq!(
        crate::execute(doc_reversed, None, &schema, &graphql_vars! {}, &database).await,
        Ok((
            graphql_value!({"hero": {"name": "R2-D2", "id": "2001"}}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_hero_name_and_friends() {
    let doc = r#"{
        hero {
            id
            name
            friends {
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
            graphql_value!({"hero": {
                "id": "2001",
                "name": "R2-D2",
                "friends": [
                    {"name": "Luke Skywalker"},
                    {"name": "Han Solo"},
                    {"name": "Leia Organa"},
                ],
            }}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_hero_name_and_friends_and_friends_of_friends() {
    let doc = r#"{
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
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
        Ok((
            graphql_value!({"hero": {
                "id": "2001",
                "name": "R2-D2",
                "friends": [{
                    "name": "Luke Skywalker",
                    "appearsIn": ["NEW_HOPE", "EMPIRE", "JEDI"],
                    "friends": [
                        {"name": "Han Solo"},
                        {"name": "Leia Organa"},
                        {"name": "C-3PO"},
                        {"name": "R2-D2"},
                    ],
                }, {
                    "name": "Han Solo",
                    "appearsIn": ["NEW_HOPE", "EMPIRE", "JEDI"],
                    "friends": [
                        {"name": "Luke Skywalker"},
                        {"name": "Leia Organa"},
                        {"name": "R2-D2"},
                    ],
                }, {
                    "name": "Leia Organa",
                    "appearsIn": ["NEW_HOPE", "EMPIRE", "JEDI"],
                    "friends": [
                        {"name": "Luke Skywalker"},
                        {"name": "Han Solo"},
                        {"name": "C-3PO"},
                        {"name": "R2-D2"},
                    ],
                }],
            }}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_query_name() {
    let doc = r#"{ human(id: "1000") { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
        Ok((
            graphql_value!({"human": {"name": "Luke Skywalker"}}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_query_alias_single() {
    let doc = r#"{ luke: human(id: "1000") { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
        Ok((graphql_value!({"luke": {"name": "Luke Skywalker"}}), vec![])),
    );
}

#[tokio::test]
async fn test_query_alias_multiple() {
    let doc = r#"{
        luke: human(id: "1000") { name }
        leia: human(id: "1003") { name }
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
                "luke": {"name": "Luke Skywalker"},
                "leia": {"name": "Leia Organa"},
            }),
            vec![],
        )),
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
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
        Ok((
            graphql_value!({
                "luke": {"name": "Luke Skywalker", "homePlanet": "Tatooine"},
                "leia": {"name": "Leia Organa", "homePlanet": "Alderaan"},
            }),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_query_name_variable() {
    let doc = r#"query FetchSomeIDQuery($someId: String!) { human(id: $someId) { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );
    let vars = graphql_vars! {"someId": "1000"};

    assert_eq!(
        crate::execute(doc, None, &schema, &vars, &database).await,
        Ok((
            graphql_value!({"human": {"name": "Luke Skywalker"}}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_query_name_invalid_variable() {
    let doc = r#"query FetchSomeIDQuery($someId: String!) { human(id: $someId) { name } }"#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );
    let vars = graphql_vars! {"someId": "some invalid id"};

    assert_eq!(
        crate::execute(doc, None, &schema, &vars, &database).await,
        Ok((graphql_value!({ "human": null }), vec![])),
    );
}

#[tokio::test]
async fn test_query_friends_names() {
    let doc = r#"{ human(id: "1000") { friends { name } } }"#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    assert_eq!(
        crate::execute(doc, None, &schema, &graphql_vars! {}, &database).await,
        Ok((
            graphql_value!({"human": {
                "friends": [
                    {"name": "Han Solo"},
                    {"name": "Leia Organa"},
                    {"name": "C-3PO"},
                    {"name": "R2-D2"},
                ],
            }}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_query_inline_fragments_droid() {
    let doc = r#"query InlineFragments {
        hero {
            name
            __typename

            ...on Droid {
                primaryFunction
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
            graphql_value!({"hero": {
                "__typename": "Droid",
                "name": "R2-D2",
                "primaryFunction": "Astromech",
            }}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_query_inline_fragments_human() {
    let doc = r#"query InlineFragments {
        hero(episode: EMPIRE) {
            __typename
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
            graphql_value!({"hero": {
                "__typename": "Human",
                "name": "Luke Skywalker",
            }}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_object_typename() {
    let doc = r#"{
        human(id: "1000") {
            __typename
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
        Ok((graphql_value!({"human": {"__typename": "Human"}}), vec![])),
    );
}

#[tokio::test]
async fn interface_inline_fragment_friends() {
    let doc = r#"{
        human(id: "1002") {
            friends {
                name
                ... on Human { homePlanet }
                ... on Droid { primaryFunction }
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
            graphql_value!({"human": {
                "friends": [
                    {"name": "Luke Skywalker", "homePlanet": "Tatooine"},
                    {"name": "Leia Organa", "homePlanet": "Alderaan"},
                    {"name": "R2-D2", "primaryFunction": "Astromech"},
                ],
            }}),
            vec![],
        ))
    );
}
