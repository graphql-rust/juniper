//! Tests for `#[graphql_interface]` macro.

use juniper::{
    execute, graphql_interface, graphql_object, graphql_value, DefaultScalarValue, EmptyMutation,
    EmptySubscription, FieldError, FieldResult, GraphQLObject, GraphQLType, IntoFieldError,
    RootNode, ScalarValue, Variables,
};

fn schema<'q, C, S, Q>(query_root: Q) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>, S>
where
    Q: GraphQLType<S, Context = C, TypeInfo = ()> + 'q,
    S: ScalarValue + 'q,
{
    RootNode::new(
        query_root,
        EmptyMutation::<C>::new(),
        EmptySubscription::<C>::new(),
    )
}

mod trivial {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        fn id(&self) -> &str {
            &self.id
        }
    }

    type DynCharacter<'a, S = DefaultScalarValue> =
        dyn Character<S, Context = (), TypeInfo = ()> + 'a + Send + Sync;

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                possibleTypes {
                    kind
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"possibleTypes": [
                    {"kind": "OBJECT", "name": "Droid"},
                    {"kind": "OBJECT", "name": "Human"},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn registers_itself_in_implementers() {
        for object in &["Human", "Droid"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        interfaces {{
                            kind
                            name
                        }}
                    }}
                }}"#,
                object,
            );

            let schema = schema(QueryRoot::Human);

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"interfaces": [
                        {"kind": "INTERFACE", "name": "Character"},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"description": None}}), vec![])),
        );
    }
}

mod dyn_alias {
    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_trait_name() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"description": None}}), vec![])),
        );
    }
}

mod trivial_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character {
        async fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                possibleTypes {
                    kind
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"possibleTypes": [
                    {"kind": "OBJECT", "name": "Droid"},
                    {"kind": "OBJECT", "name": "Human"},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn registers_itself_in_implementers() {
        for object in &["Human", "Droid"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        interfaces {{
                            kind
                            name
                        }}
                    }}
                }}"#,
                object,
            );

            let schema = schema(QueryRoot::Human);

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"interfaces": [
                        {"kind": "INTERFACE", "name": "Character"},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"description": None}}), vec![])),
        );
    }
}

mod explicit_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character {
        fn id(&self) -> &str;

        async fn info(&self) -> String {
            "None available".to_owned()
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }

        async fn info(&self) -> String {
            format!("Home planet is {}", &self.home_planet)
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface(async)]
    impl Character for Droid {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn resolves_info_field() {
        const DOC: &str = r#"{
            character {
                info
            }
        }"#;

        for (root, expected) in &[
            (QueryRoot::Human, "Home planet is earth"),
            (QueryRoot::Droid, "None available"),
        ] {
            let schema = schema(*root);

            let expected: &str = *expected;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"info": expected}}), vec![])),
            );
        }
    }
}

mod fallible_field {
    use super::*;

    struct CustomError;

    impl<S: ScalarValue> IntoFieldError<S> for CustomError {
        fn into_field_error(self) -> FieldError<S> {
            juniper::FieldError::new("Whatever", graphql_value!({"code": "some"}))
        }
    }

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character {
        fn id(&self) -> Result<&str, CustomError>;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> Result<&str, CustomError> {
            Ok(&self.id)
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        fn id(&self) -> Result<&str, CustomError> {
            Ok(&self.id)
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_correct_graphql_type() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
                kind
                fields {
                    name
                    type {
                        kind
                        ofType {
                            name
                        }
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {
                    "name": "Character",
                    "kind": "INTERFACE",
                    "fields": [{
                        "name": "id",
                        "type": {"kind": "NON_NULL", "ofType": {"name": "String"}},
                    }]
                }}),
                vec![],
            )),
        );
    }
}

mod generic {
    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character<A, B> {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character<u8, ()>)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl<A, B> Character<A, B> for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character<u8, ()>)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl<A, B> Character<A, B> for Droid {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_, u8, ()>> {
            let ch: Box<DynCharacter<'_, u8, ()>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                possibleTypes {
                    kind
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"possibleTypes": [
                    {"kind": "OBJECT", "name": "Droid"},
                    {"kind": "OBJECT", "name": "Human"},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn registers_itself_in_implementers() {
        for object in &["Human", "Droid"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        interfaces {{
                            kind
                            name
                        }}
                    }}
                }}"#,
                object,
            );

            let schema = schema(QueryRoot::Human);

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"interfaces": [
                        {"kind": "INTERFACE", "name": "Character"},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name_without_type_params() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }
}

mod generic_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character<A, B> {
        async fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character<u8, ()>)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl<A, B> Character<A, B> for Human {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character<u8, ()>)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl<A, B> Character<A, B> for Droid {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_, u8, ()>> {
            let ch: Box<DynCharacter<'_, u8, ()>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                possibleTypes {
                    kind
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"possibleTypes": [
                    {"kind": "OBJECT", "name": "Droid"},
                    {"kind": "OBJECT", "name": "Human"},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn registers_itself_in_implementers() {
        for object in &["Human", "Droid"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        interfaces {{
                            kind
                            name
                        }}
                    }}
                }}"#,
                object,
            );

            let schema = schema(QueryRoot::Human);

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"interfaces": [
                        {"kind": "INTERFACE", "name": "Character"},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name_without_type_params() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }
}

mod generic_lifetime_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character<'me, A, B> {
        async fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character<'_, u8, ()>)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl<'me, A, B> Character<'me, A, B> for Human {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character<'_, u8, ()>)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl<'me, A, B> Character<'me, A, B> for Droid {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_, '_, u8, ()>> {
            let ch: Box<DynCharacter<'_, '_, u8, ()>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_trait_name_without_type_params() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }
}

mod argument {
    use super::*;

    #[graphql_interface(for = Human, dyn = DynCharacter)]
    trait Character {
        fn id_wide(&self, is_planet: bool) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id_wide(&self, is_planet: bool) -> &str {
            if is_planet {
                &self.home_planet
            } else {
                &self.id
            }
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        let schema = schema(QueryRoot);

        for (input, expected) in &[
            ("{ character { idWide(isPlanet: true) } }", "earth"),
            ("{ character { idWide(isPlanet: false) } }", "human-32"),
        ] {
            let expected: &str = *expected;

            assert_eq!(
                execute(*input, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"idWide": expected}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn camelcases_name() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                fields {
                    name
                    args {
                        name
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "idWide", "args": [{"name": "isPlanet"}]},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                fields {
                    args {
                        description
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": { "fields": [{"args": [{"description": None}]}]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_no_defaults() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                fields {
                    args {
                        defaultValue
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": { "fields": [{"args": [{"defaultValue": None}]}]}}),
                vec![],
            )),
        );
    }
}

mod default_argument {
    use super::*;

    #[graphql_interface(for = Human, dyn = DynCharacter)]
    trait Character {
        async fn id(
            &self,
            #[graphql_interface(default)] first: String,
            #[graphql_interface(default = "second".to_string())] second: String,
            #[graphql_interface(default = "t")] third: String,
        ) -> String;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
    }

    #[graphql_interface]
    impl Character for Human {
        async fn id(&self, first: String, second: String, third: String) -> String {
            format!("{}|{}&{}", first, second, third)
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            Box::new(Human {
                id: "human-32".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        let schema = schema(QueryRoot);

        for (input, expected) in &[
            ("{ character { id } }", "|second&t"),
            (r#"{ character { id(first: "first") } }"#, "first|second&t"),
            (r#"{ character { id(second: "") } }"#, "|&t"),
            (
                r#"{ character { id(first: "first", second: "") } }"#,
                "first|&t",
            ),
            (
                r#"{ character { id(first: "first", second: "", third: "") } }"#,
                "first|&",
            ),
        ] {
            let expected: &str = *expected;

            assert_eq!(
                execute(*input, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_defaults() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                fields {
                    args {
                        name
                        defaultValue
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{"args": [
                    {"name": "first", "defaultValue": r#""""#},
                    {"name": "second", "defaultValue": r#""second""#},
                    {"name": "third", "defaultValue": r#""t""#},
                ]}]}}),
                vec![],
            )),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    /// Rust docs.
    #[graphql_interface(for = Human, dyn = DynCharacter)]
    trait Character {
        /// Rust `id` docs.
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn uses_doc_comment_as_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
                fields {
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "Rust docs.", "fields": [{"description": "Rust `id` docs."}],
                }}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_attr {
    #![allow(deprecated)]

    use super::*;

    #[graphql_interface(for = Human, dyn = DynCharacter)]
    trait Character {
        fn id(&self) -> &str;

        #[deprecated]
        fn a(&self) -> &str {
            "a"
        }

        #[deprecated(note = "Use `id`.")]
        fn b(&self) -> &str {
            "b"
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"character": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_deprecated_fields() {
        const DOC: &str = r#"{
            character {
                a
                b
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"character": {"a": "a", "b": "b"}}), vec![],)),
        );
    }

    #[tokio::test]
    async fn deprecates_fields() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                fields(includeDeprecated: true) {
                    name
                    isDeprecated
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "isDeprecated": false},
                    {"name": "a", "isDeprecated": true},
                    {"name": "b", "isDeprecated": true},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn provides_deprecation_reason() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                fields(includeDeprecated: true) {
                    name
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "deprecationReason": None},
                    {"name": "a", "deprecationReason": None},
                    {"name": "b", "deprecationReason": "Use `id`."},
                ]}}),
                vec![],
            )),
        );
    }
}

mod explicit_name_description_and_deprecation {
    #![allow(deprecated)]

    use super::*;

    /// Rust docs.
    #[graphql_interface(name = "MyChar", desc = "My character.", for = Human, dyn = DynCharacter)]
    trait Character {
        /// Rust `id` docs.
        #[graphql_interface(name = "myId", desc = "My character ID.", deprecated = "Not used.")]
        #[deprecated(note = "Should be omitted.")]
        fn id(
            &self,
            #[graphql_interface(name = "myName", desc = "My argument.", default)] n: Option<String>,
        ) -> &str;

        #[graphql_interface(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        fn a(&self) -> &str {
            "a"
        }

        fn b(&self) -> &str {
            "b"
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self, _: Option<String>) -> &str {
            &self.id
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            character {
                myId
                a
                b
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"myId": "human-32", "a": "a", "b": "b"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_custom_name() {
        const DOC: &str = r#"{
            __type(name: "MyChar") {
                name
                fields(includeDeprecated: true) {
                    name
                    args {
                        name
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {
                    "name": "MyChar",
                    "fields": [
                        {"name": "myId", "args": [{"name": "myName"}]},
                        {"name": "a", "args": []},
                        {"name": "b", "args": []},
                    ],
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_custom_description() {
        const DOC: &str = r#"{
            __type(name: "MyChar") {
                description
                fields(includeDeprecated: true) {
                    name
                    description
                    args {
                        description
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "My character.",
                    "fields": [{
                        "name": "myId",
                        "description": "My character ID.",
                        "args": [{"description": "My argument."}],
                    }, {
                        "name": "a",
                        "description": None,
                        "args": [],
                    }, {
                        "name": "b",
                        "description": None,
                        "args": [],
                    }],
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_custom_deprecation() {
        const DOC: &str = r#"{
            __type(name: "MyChar") {
                fields(includeDeprecated: true) {
                    name
                    isDeprecated
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {
                    "fields": [{
                        "name": "myId",
                        "isDeprecated": true,
                        "deprecationReason": "Not used.",
                    }, {
                        "name": "a",
                        "isDeprecated": true,
                        "deprecationReason": None,
                    }, {
                        "name": "b",
                        "isDeprecated": false,
                        "deprecationReason": None,
                    }],
                }}),
                vec![],
            )),
        );
    }
}

mod explicit_scalar {
    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    #[graphql_interface(scalar = DefaultScalarValue)]
    trait Character {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, scalar = DefaultScalarValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface(scalar = DefaultScalarValue)]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, scalar = DefaultScalarValue)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface(scalar = DefaultScalarValue)]
    impl Character for Droid {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema::<_, DefaultScalarValue, _>(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema::<_, DefaultScalarValue, _>(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema::<_, DefaultScalarValue, _>(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod custom_scalar {
    use crate::custom_scalar::MyScalarValue;

    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    #[graphql_interface(scalar = MyScalarValue)]
    trait Character {
        async fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, scalar = MyScalarValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface(scalar = MyScalarValue)]
    impl Character for Human {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, scalar = MyScalarValue)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface(scalar = MyScalarValue)]
    impl Character for Droid {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema::<_, MyScalarValue, _>(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema::<_, MyScalarValue, _>(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema::<_, MyScalarValue, _>(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod explicit_generic_scalar {
    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter, scalar = S)]
    trait Character<S: ScalarValue = DefaultScalarValue> {
        fn id(&self) -> FieldResult<&str, S>;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface(scalar = S)]
    impl<S: ScalarValue> Character<S> for Human {
        fn id(&self) -> FieldResult<&str, S> {
            Ok(&self.id)
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface(scalar = S)]
    impl<S: ScalarValue> Character<S> for Droid {
        fn id(&self) -> FieldResult<&str, S> {
            Ok(&self.id)
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod explicit_custom_context {
    use super::*;

    pub struct CustomContext;

    impl juniper::Context for CustomContext {}

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter, context = CustomContext)]
    trait Character {
        async fn id<'a>(&'a self, context: &CustomContext) -> &'a str;

        async fn info<'b>(&'b self, ctx: &()) -> &'b str;

        fn more<'c>(&'c self, #[graphql_interface(context)] custom: &CustomContext) -> &'c str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, context = CustomContext)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        async fn id<'a>(&'a self, _: &CustomContext) -> &'a str {
            &self.id
        }

        async fn info<'b>(&'b self, _: &()) -> &'b str {
            &self.home_planet
        }

        fn more(&self, _: &CustomContext) -> &'static str {
            "human"
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, context = CustomContext)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        async fn id<'a>(&'a self, _: &CustomContext) -> &'a str {
            &self.id
        }

        async fn info<'b>(&'b self, _: &()) -> &'b str {
            &self.primary_function
        }

        fn more(&self, _: &CustomContext) -> &'static str {
            "droid"
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &CustomContext).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &CustomContext).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
                info
                more
            }
        }"#;

        for (root, expected_id, expected_info, expexted_more) in &[
            (QueryRoot::Human, "human-32", "earth", "human"),
            (QueryRoot::Droid, "droid-99", "run", "droid"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            let expected_info: &str = *expected_info;
            let expexted_more: &str = *expexted_more;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &CustomContext).await,
                Ok((
                    graphql_value!({"character": {
                        "id": expected_id,
                        "info": expected_info,
                        "more": expexted_more,
                    }}),
                    vec![],
                )),
            );
        }
    }
}

mod inferred_custom_context_from_field {
    use super::*;

    pub struct CustomContext(String);

    impl juniper::Context for CustomContext {}

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character {
        async fn id<'a>(&self, context: &'a CustomContext) -> &'a str;

        async fn info<'b>(&'b self, context: &()) -> &'b str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, context = CustomContext)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        async fn id<'a>(&self, ctx: &'a CustomContext) -> &'a str {
            &ctx.0
        }

        async fn info<'b>(&'b self, _: &()) -> &'b str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, context = CustomContext)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        async fn id<'a>(&self, ctx: &'a CustomContext) -> &'a str {
            &ctx.0
        }

        async fn info<'b>(&'b self, _: &()) -> &'b str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);
        let ctx = CustomContext("in-ctx".into());

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &ctx).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);
        let ctx = CustomContext("in-droid".into());

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &ctx).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
                info
            }
        }"#;

        for (root, expected_id, expected_info) in &[
            (QueryRoot::Human, "human-ctx", "earth"),
            (QueryRoot::Droid, "droid-ctx", "run"),
        ] {
            let schema = schema(*root);
            let ctx = CustomContext(expected_id.to_string());

            let expected_id: &str = *expected_id;
            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &ctx).await,
                Ok((
                    graphql_value!({"character": {"id": expected_id, "info": expected_info}}),
                    vec![],
                )),
            );
        }
    }
}

mod inferred_custom_context_from_downcast {
    use super::*;

    struct Database {
        droid: Option<Droid>,
    }

    impl juniper::Context for Database {}

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character {
        #[graphql_interface(downcast)]
        fn as_droid<'db>(&self, db: &'db Database) -> Option<&'db Droid>;

        async fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, context = Database)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn as_droid<'db>(&self, _: &'db Database) -> Option<&'db Droid> {
            None
        }

        async fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, context = Database)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        fn as_droid<'db>(&self, db: &'db Database) -> Option<&'db Droid> {
            db.droid.as_ref()
        }

        async fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = Database)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);
        let db = Database { droid: None };

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &db).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);
        let db = Database {
            droid: Some(Droid {
                id: "droid-88".to_string(),
                primary_function: "sit".to_string(),
            }),
        };

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &db).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-88", "primaryFunction": "sit"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);
            let db = Database { droid: None };

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &db).await,
                Ok((
                    graphql_value!({"character": {"info": expected_info}}),
                    vec![],
                )),
            );
        }
    }
}

mod ignored_method {
    use super::*;

    #[graphql_interface(for = Human, dyn = DynCharacter)]
    trait Character {
        fn id(&self) -> &str;

        #[graphql_interface(ignore)]
        fn ignored(&self) -> Option<&Human> {
            None
        }

        #[graphql_interface(skip)]
        fn skipped(&self) {}
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"character": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_not_field() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                fields {
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{"name": "id"}]}}),
                vec![],
            )),
        );
    }
}

mod downcast_method {
    use super::*;

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    trait Character {
        fn id(&self) -> &str;

        #[graphql_interface(downcast)]
        fn as_droid(&self) -> Option<&Droid> {
            None
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn as_droid(&self) -> Option<&Droid> {
            Some(self)
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_not_field() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                fields {
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{"name": "id"}]}}),
                vec![],
            )),
        );
    }
}

mod external_downcast {
    use super::*;

    struct Database {
        droid: Option<Droid>,
    }

    impl juniper::Context for Database {}

    #[graphql_interface(for = [Human, Droid], dyn = DynCharacter)]
    #[graphql_interface(context = Database)]
    #[graphql_interface(on Droid = DynCharacter::as_droid)]
    trait Character {
        fn id(&self) -> &str;
    }

    impl<'a, S: ScalarValue> DynCharacter<'a, S> {
        fn as_droid<'db>(&self, db: &'db Database) -> Option<&'db Droid> {
            db.droid.as_ref()
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, context = Database)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = dyn Character, context = Database)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = Database)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "?????".to_string(),
                    primary_function: "???".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);
        let db = Database { droid: None };

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &db).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);
        let db = Database {
            droid: Some(Droid {
                id: "droid-99".to_string(),
                primary_function: "run".to_string(),
            }),
        };

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &db).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let db = Database {
            droid: Some(Droid {
                id: "droid-99".to_string(),
                primary_function: "run".to_string(),
            }),
        };

        for (root, expected_id) in &[(QueryRoot::Human, "human-32"), (QueryRoot::Droid, "?????")] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &db).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}
