//! Tests for `#[graphql_interface]` macro.

use juniper::{
    execute, graphql_interface, graphql_object, graphql_value, graphql_vars, DefaultScalarValue,
    EmptyMutation, EmptySubscription, Executor, FieldError, FieldResult, GraphQLInputObject,
    GraphQLObject, GraphQLType, GraphQLUnion, IntoFieldError, RootNode, ScalarValue,
};

fn schema<'q, C, Q>(query_root: Q) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>>
where
    Q: GraphQLType<DefaultScalarValue, Context = C, TypeInfo = ()> + 'q,
{
    RootNode::new(
        query_root,
        EmptyMutation::<C>::new(),
        EmptySubscription::<C>::new(),
    )
}

fn schema_with_scalar<'q, S, C, Q>(
    query_root: Q,
) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>, S>
where
    Q: GraphQLType<S, Context = C, TypeInfo = ()> + 'q,
    S: ScalarValue + 'q,
{
    RootNode::new_with_scalar_value(
        query_root,
        EmptyMutation::<C>::new(),
        EmptySubscription::<C>::new(),
    )
}

mod no_implers {
    use super::*;

    #[graphql_interface]
    trait Character {
        fn id(&self) -> &str;
    }

    struct QueryRoot;

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        let schema = schema(QueryRoot);

        let doc = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_trait_name() {
        let schema = schema(QueryRoot);

        let doc = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        let schema = schema(QueryRoot);

        let doc = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod trivial {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        let schema = schema(QueryRoot::Human);

        let doc = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        let schema = schema(QueryRoot::Human);

        let doc = r#"{
            __type(name: "Character") {
                possibleTypes {
                    kind
                    name
                }
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
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
        let schema = schema(QueryRoot::Human);

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

            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
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
        let schema = schema(QueryRoot::Human);

        let doc = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        let schema = schema(QueryRoot::Human);

        let doc = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod explicit_alias {
    use super::*;

    #[graphql_interface(enum = CharacterEnum, for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterEnum)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterEnum)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterEnum {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod trivial_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        async fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        let schema = schema(QueryRoot::Human);

        let doc = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        let schema = schema(QueryRoot::Human);

        let doc = r#"{
            __type(name: "Character") {
                possibleTypes {
                    kind
                    name
                }
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
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
        let schema = schema(QueryRoot::Human);

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

            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
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
        let schema = schema(QueryRoot::Human);

        let doc = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        let schema = schema(QueryRoot::Human);

        let doc = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod explicit_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;

        async fn info(&self) -> String;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        info: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        async fn info(&self) -> String {
            format!("Primary function is {}", &self.primary_function)
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    info: "Home planet is earth".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_fields() {
        const DOC: &str = r#"{
            character {
                id
                info
            }
        }"#;

        for (root, expected_id, expected_info) in &[
            (QueryRoot::Human, "human-32", "Home planet is earth"),
            (QueryRoot::Droid, "droid-99", "Primary function is run"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"character": {
                        "id": expected_id,
                        "info": expected_info,
                    }}),
                    vec![],
                )),
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> Result<&str, CustomError>;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> Result<String, CustomError> {
            Ok(self.id.clone())
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_correct_graphql_type() {
        let schema = schema(QueryRoot::Human);

        let doc = r#"{
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

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character<A = (), B: ?Sized = ()> {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue<(), u8>)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name_without_type_params() {
        let doc = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }
}

mod generic_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character<A = (), B: ?Sized = ()> {
        async fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue<(), u8>)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name_without_type_params() {
        let doc = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }
}

mod generic_lifetime_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character<'me, A> {
        async fn id<'a>(&'a self) -> &'a str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue<()>)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue<()>)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue<'_, ()> {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name_without_type_params() {
        let doc = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }
}

mod argument {
    use super::*;

    #[graphql_interface(for = Human)]
    trait Character {
        fn id_wide(&self, is_number: bool) -> &str;

        async fn id_wide2(&self, is_number: bool, r#async: Option<i32>) -> &str;
    }

    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Human {
        fn id(&self) -> &str {
            &self.id
        }

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        fn id_wide(&self, is_number: bool) -> &str {
            if is_number {
                &self.id
            } else {
                "none"
            }
        }

        async fn id_wide2(&self, is_number: bool, _async: Option<i32>) -> &str {
            if is_number {
                &self.id
            } else {
                "none"
            }
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            }
            .into()
        }
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
        let schema = schema(QueryRoot);

        for (input, expected) in &[
            (
                "{ character { idWide(isNumber: true), idWide2(isNumber: true) } }",
                "human-32",
            ),
            (
                "{ character { idWide(isNumber: false), idWide2(isNumber: false, async: 5) } }",
                "none",
            ),
        ] {
            let expected: &str = *expected;

            assert_eq!(
                execute(*input, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"character": {
                        "idWide": expected,
                        "idWide2": expected,
                    }}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn camelcases_name() {
        let schema = schema(QueryRoot);

        let doc = r#"{
            __type(name: "Character") {
                fields {
                    name
                    args {
                        name
                    }
                }
            }
        }"#;
        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{
                    "name": "idWide",
                    "args": [
                        {"name": "isNumber"},
                    ],
                },{
                    "name": "idWide2",
                    "args": [
                        {"name": "isNumber"},
                        {"name": "async"}
                    ],
                }]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        let schema = schema(QueryRoot);

        let doc = r#"{
            __type(name: "Character") {
                fields {
                    args {
                        description
                    }
                }
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"args": [{"description": null}]},
                    {"args": [{"description": null}, {"description": null}]},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_no_defaults() {
        let schema = schema(QueryRoot);

        let doc = r#"{
            __type(name: "Character") {
                fields {
                    args {
                        defaultValue
                    }
                }
            }
        }"#;

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"args": [{"defaultValue": null}]},
                    {"args": [{"defaultValue": null}, {"defaultValue": null}]},
                ]}}),
                vec![],
            )),
        );
    }
}

mod default_argument {
    use super::*;

    #[derive(GraphQLInputObject, Debug)]
    struct Point {
        x: i32,
    }

    #[graphql_interface(for = Human)]
    trait Character {
        async fn id(
            &self,
            #[graphql(default)] first: String,
            #[graphql(default = "second".to_string())] second: String,
            #[graphql(default = "t")] third: String,
        ) -> String;

        fn info(&self, #[graphql(default = Point { x: 1 })] coord: Point) -> i32;
    }

    struct Human;

    #[graphql_object(impl = CharacterValue)]
    impl Human {
        fn info(&self, coord: Point) -> i32 {
            coord.x
        }

        async fn id(&self, first: String, second: String, third: String) -> String {
            format!("{}|{}&{}", first, second, third)
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human.into()
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
                execute(*input, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn resolves_info_field() {
        let schema = schema(QueryRoot);

        for (input, expected) in &[
            ("{ character { info } }", 1),
            ("{ character { info(coord: {x: 2}) } }", 2),
        ] {
            let expected: i32 = *expected;

            assert_eq!(
                execute(*input, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"info": expected}}), vec![])),
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
                        type {
                            name
                            ofType {
                                name
                            }
                        }
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{
                    "args": [{
                        "name": "first",
                        "defaultValue": r#""""#,
                        "type": {"name": "String", "ofType": null},
                    }, {
                        "name": "second",
                        "defaultValue": r#""second""#,
                        "type": {"name": "String", "ofType": null},
                    }, {
                        "name": "third",
                        "defaultValue": r#""t""#,
                        "type": {"name": "String", "ofType": null},
                    }],
                }, {
                    "args": [{
                        "name": "coord",
                        "defaultValue": "{x: 1}",
                        "type": {"name": "Point", "ofType": null},
                    }],
                }]}}),
                vec![],
            )),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    /// Rust docs.
    #[graphql_interface(for = Human)]
    trait Character {
        /// Rust `id` docs.
        /// Long.
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            }
            .into()
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "Rust docs.",
                    "fields": [{"description": "Rust `id` docs.\nLong."}],
                }}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_attr {
    use super::*;

    #[graphql_interface(for = Human)]
    trait Character {
        fn id(&self) -> &str;

        #[deprecated]
        fn a(&self) -> &str;

        #[deprecated(note = "Use `id`.")]
        fn b(&self) -> &str;
    }

    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Human {
        fn id(&self) -> &str {
            &self.id
        }

        fn human_planet(&self) -> &str {
            &self.home_planet
        }

        fn a() -> &'static str {
            "a"
        }

        fn b() -> String {
            "b".to_owned()
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            }
            .into()
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"character": {"a": "a", "b": "b"}}), vec![])),
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "deprecationReason": null},
                    {"name": "a", "deprecationReason": null},
                    {"name": "b", "deprecationReason": "Use `id`."},
                ]}}),
                vec![],
            )),
        );
    }
}

mod explicit_name_description_and_deprecation {
    use super::*;

    /// Rust docs.
    #[graphql_interface(name = "MyChar", desc = "My character.", for = Human)]
    trait Character {
        /// Rust `id` docs.
        #[graphql(name = "myId", desc = "My character ID.", deprecated = "Not used.")]
        #[deprecated(note = "Should be omitted.")]
        fn id(&self, #[graphql(name = "myName", desc = "My argument.")] n: Option<String>) -> &str;

        #[graphql(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        fn a(&self) -> &str;

        fn b(&self) -> &str;
    }

    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Human {
        fn my_id(&self, #[graphql(name = "myName")] _: Option<String>) -> &str {
            &self.id
        }

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        fn a() -> String {
            "a".to_owned()
        }

        fn b() -> &'static str {
            "b"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            }
            .into()
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "My character.",
                    "fields": [{
                        "name": "myId",
                        "description": "My character ID.",
                        "args": [{"description": "My argument."}],
                    }, {
                        "name": "a",
                        "description": null,
                        "args": [],
                    }, {
                        "name": "b",
                        "description": null,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "fields": [{
                        "name": "myId",
                        "isDeprecated": true,
                        "deprecationReason": "Not used.",
                    }, {
                        "name": "a",
                        "isDeprecated": true,
                        "deprecationReason": null,
                    }, {
                        "name": "b",
                        "isDeprecated": false,
                        "deprecationReason": null,
                    }],
                }}),
                vec![],
            )),
        );
    }
}

mod renamed_all_fields_and_args {
    use super::*;

    #[graphql_interface(rename_all = "none", for = Human)]
    trait Character {
        fn id(&self) -> &str;

        async fn home_planet(&self, planet_name: String) -> String;

        async fn r#async_info(&self, r#my_num: i32) -> i32;
    }

    struct Human;

    #[graphql_object(rename_all = "none", impl = CharacterValue)]
    impl Human {
        fn id() -> &'static str {
            "human-32"
        }

        fn home_planet(planet_name: String) -> String {
            planet_name
        }

        fn r#async_info(r#my_num: i32) -> i32 {
            r#my_num
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human.into()
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            character {
                id
                home_planet(planet_name: "earth")
                async_info(my_num: 3)
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "id": "human-32",
                    "home_planet": "earth",
                    "async_info": 3,
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_correct_fields_and_args_names() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "args": []},
                    {"name": "home_planet", "args": [{"name": "planet_name"}]},
                    {"name": "async_info", "args": [{"name": "my_num"}]},
                ]}}),
                vec![],
            )),
        );
    }
}

mod explicit_scalar {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    #[graphql_interface(scalar = DefaultScalarValue)]
    trait Character {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue, scalar = DefaultScalarValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue, scalar = DefaultScalarValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod custom_scalar {
    use crate::custom_scalar::MyScalarValue;

    use super::*;

    #[graphql_interface(for = [Human, Droid], scalar = MyScalarValue)]
    trait Character {
        async fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue, scalar = MyScalarValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue, scalar = MyScalarValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema_with_scalar::<MyScalarValue, _, _>(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod explicit_generic_scalar {
    use super::*;

    #[graphql_interface(for = [Human, Droid], scalar = S)]
    trait Character<S: ScalarValue = DefaultScalarValue> {
        fn id(&self) -> FieldResult<&str, S>;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue<__S>)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue<__S>)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character<S: ScalarValue>(&self) -> CharacterValue<S> {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod bounded_generic_scalar {
    use super::*;

    #[graphql_interface(for = [Human, Droid], scalar = S: ScalarValue + Clone)]
    trait Character {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue, scalar = S: ScalarValue + Clone)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue, scalar = S: ScalarValue + Clone)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Clone + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod explicit_custom_context {
    use super::*;

    struct CustomContext;

    impl juniper::Context for CustomContext {}

    #[graphql_interface(for = [Human, Droid], context = CustomContext)]
    trait Character {
        async fn id<'a>(&'a self, context: &CustomContext) -> &'a str;

        async fn info<'b>(&'b self, ctx: &()) -> &'b str;

        fn more<'c>(&'c self, #[graphql(context)] custom: &CustomContext) -> &'c str;
    }

    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_object(impl = CharacterValue, context = CustomContext)]
    impl Human {
        fn id<'a>(&'a self, _context: &CustomContext) -> &'a str {
            &self.id
        }

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        fn info<'b>(&'b self, _ctx: &()) -> &'b str {
            &self.home_planet
        }

        fn more(&self, #[graphql(context)] _: &CustomContext) -> &'static str {
            "human"
        }
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue, context = CustomContext)]
    impl Droid {
        async fn id<'a>(&'a self) -> &'a str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }

        async fn info<'b>(&'b self) -> &'b str {
            &self.primary_function
        }

        fn more(&self) -> &'static str {
            "droid"
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = CustomContext, scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_fields() {
        let doc = r#"{
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
                execute(&doc, None, &schema, &graphql_vars! {}, &CustomContext).await,
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

    struct CustomContext(String);

    impl juniper::Context for CustomContext {}

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id<'a>(&self, context: &'a CustomContext) -> &'a str;

        fn info<'b>(&'b self, context: &()) -> &'b str;
    }

    struct Human {
        home_planet: String,
    }

    #[graphql_object(impl = CharacterValue, context = CustomContext)]
    impl Human {
        fn id<'a>(&self, ctx: &'a CustomContext) -> &'a str {
            &ctx.0
        }

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        fn info<'b>(&'b self, _context: &()) -> &'b str {
            &self.home_planet
        }
    }

    struct Droid {
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue, context = CustomContext)]
    impl Droid {
        fn id<'a>(&self, ctx: &'a CustomContext) -> &'a str {
            &ctx.0
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }

        fn info<'b>(&'b self) -> &'b str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = CustomContext, scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((
                graphql_value!({"character": {"humanId": "in-ctx", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((
                graphql_value!({"character": {"droidId": "in-droid", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_fields() {
        let doc = r#"{
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
                execute(&doc, None, &schema, &graphql_vars! {}, &ctx).await,
                Ok((
                    graphql_value!({"character": {
                        "id": expected_id,
                        "info": expected_info,
                    }}),
                    vec![],
                )),
            );
        }
    }
}

mod executor {
    use juniper::LookAheadMethods as _;

    use super::*;

    #[graphql_interface(for = [Human, Droid], scalar = S)]
    trait Character<S: ScalarValue> {
        async fn id<'a>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str
        where
            S: Send + Sync;

        async fn info<'b>(
            &'b self,
            arg: Option<i32>,
            #[graphql(executor)] another: &Executor<'_, '_, (), S>,
        ) -> &'b str
        where
            S: Send + Sync;
    }

    struct Human {
        home_planet: String,
    }

    #[graphql_object(impl = CharacterValue<__S>)]
    impl Human {
        async fn id<'a, S: ScalarValue>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str {
            executor.look_ahead().field_name()
        }

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        async fn info<'b>(&'b self, _arg: Option<i32>) -> &'b str {
            &self.home_planet
        }
    }

    struct Droid {
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue<__S>)]
    impl Droid {
        fn id<'a, S: ScalarValue>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str {
            executor.look_ahead().field_name()
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }

        async fn info<'b, S: ScalarValue>(
            &'b self,
            _arg: Option<i32>,
            _executor: &Executor<'_, '_, (), S>,
        ) -> &'b str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue<DefaultScalarValue> {
            match self {
                Self::Human => Human {
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "humanId", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droidId", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_fields() {
        let doc = r#"{
            character {
                id
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"character": {"id": "id", "info": expected_info}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn not_arg() {
        let doc = r#"{
            __type(name: "Character") {
                fields {
                    name
                    args {
                        name
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "args": []},
                    {"name": "info", "args": [{"name": "arg"}]},
                ]}}),
                vec![],
            )),
        );
    }
}

mod ignored_method {
    use super::*;

    #[graphql_interface(for = Human)]
    trait Character {
        fn id(&self) -> &str;

        #[graphql(ignore)]
        fn ignored(&self) -> Option<&Human> {
            None
        }

        #[graphql(skip)]
        fn skipped(&self) {}
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            }
            .into()
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{"name": "id"}]}}),
                vec![],
            )),
        );
    }
}

mod field_return_subtyping {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> Option<String>;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
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
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod field_return_union_subtyping {
    use super::*;

    #[derive(GraphQLObject)]
    struct Strength {
        value: i32,
    }

    #[derive(GraphQLObject)]
    struct Knowledge {
        value: i32,
    }

    #[allow(dead_code)]
    #[derive(GraphQLUnion)]
    enum KeyFeature {
        Strength(Strength),
        Knowledge(Knowledge),
    }

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> Option<String>;

        fn key_feature(&self) -> KeyFeature;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
        key_feature: Knowledge,
    }

    struct Droid {
        id: String,
        primary_function: String,
        strength: i32,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }

        fn key_feature(&self) -> Strength {
            Strength {
                value: self.strength,
            }
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                    key_feature: Knowledge { value: 10 },
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                    strength: 42,
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                    keyFeature {
                        value
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth", "keyFeature": {"value": 10}}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                    keyFeature {
                        value
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run", "keyFeature": {"value": 42}}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
                keyFeature {
                    ...on Strength {
                        value
                    }
                    ... on Knowledge {
                        value
                    }
                }
            }
        }"#;

        for (root, expected_id, expected_val) in &[
            (QueryRoot::Human, "human-32", 10),
            (QueryRoot::Droid, "droid-99", 42),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            let expected_val = *expected_val;
            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"character": {"id": expected_id, "keyFeature": {"value": expected_val}}}),
                    vec![]
                )),
            );
        }
    }
}

mod additional_nullable_argument {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> Option<String>;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self, is_present: Option<bool>) -> &str {
            is_present
                .unwrap_or_default()
                .then(|| self.id.as_str())
                .unwrap_or("missing")
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue + Send + Sync)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn enum_resolves_human() {
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
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id(isPresent: true)
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "missing"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}
