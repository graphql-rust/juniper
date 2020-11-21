//! Tests for `#[graphql_interface]` macro.

use juniper::{
    execute, graphql_interface, graphql_object, graphql_value, DefaultScalarValue, EmptyMutation,
    EmptySubscription, Executor, FieldError, FieldResult, GraphQLObject, GraphQLType,
    IntoFieldError, RootNode, ScalarValue, Variables,
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

    #[graphql_interface(dyn = DynHero)]
    trait Hero {
        fn info(&self) -> &str;
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            unimplemented!()
        }

        fn hero(&self) -> Box<DynHero<'_, __S>> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        let schema = schema(QueryRoot);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        kind
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name() {
        let schema = schema(QueryRoot);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        name
                    }}
                }}"#,
                interface,
            );

            let expected_name: &str = *interface;
            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"name": expected_name}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_no_description() {
        let schema = schema(QueryRoot);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        description
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"description": None}}), vec![])),
            );
        }
    }
}

mod trivial {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero {
        fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
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

    #[graphql_interface(dyn)]
    impl Hero for Human {
        fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
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

    #[graphql_interface(dyn)]
    impl Hero for Droid {
        fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        let schema = schema(QueryRoot::Human);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        kind
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        let schema = schema(QueryRoot::Human);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        possibleTypes {{
                            kind
                            name
                        }}
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"possibleTypes": [
                        {"kind": "OBJECT", "name": "Droid"},
                        {"kind": "OBJECT", "name": "Human"},
                    ]}}),
                    vec![],
                )),
            );
        }
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
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"interfaces": [
                        {"kind": "INTERFACE", "name": "Character"},
                        {"kind": "INTERFACE", "name": "Hero"},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name() {
        let schema = schema(QueryRoot::Human);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        name
                    }}
                }}"#,
                interface,
            );

            let expected_name: &str = *interface;
            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"name": expected_name}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_no_description() {
        let schema = schema(QueryRoot::Human);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        description
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"description": None}}), vec![])),
            );
        }
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

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterEnum)]
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        async fn id(&self) -> &str;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero {
        async fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
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

    #[graphql_interface(dyn)]
    impl Hero for Human {
        async fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
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

    #[graphql_interface(dyn)]
    impl Hero for Droid {
        async fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        let schema = schema(QueryRoot::Human);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        kind
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        let schema = schema(QueryRoot::Human);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        possibleTypes {{
                            kind
                            name
                        }}
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"possibleTypes": [
                        {"kind": "OBJECT", "name": "Droid"},
                        {"kind": "OBJECT", "name": "Human"},
                    ]}}),
                    vec![],
                )),
            );
        }
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
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"interfaces": [
                        {"kind": "INTERFACE", "name": "Character"},
                        {"kind": "INTERFACE", "name": "Hero"},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name() {
        let schema = schema(QueryRoot::Human);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        name
                    }}
                }}"#,
                interface,
            );

            let expected_name: &str = *interface;
            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"name": expected_name}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_no_description() {
        let schema = schema(QueryRoot::Human);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        description
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"description": None}}), vec![])),
            );
        }
    }
}

mod explicit_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;

        async fn info(&self) -> String {
            "None available".to_owned()
        }
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero {
        async fn id(&self) -> &str {
            "Non-identified"
        }

        fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
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

    #[graphql_interface(async, dyn)]
    impl Hero for Human {
        fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
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

    #[graphql_interface(dyn)]
    impl Hero for Droid {
        async fn id(&self) -> &str {
            &self.id
        }

        fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
            (QueryRoot::Droid, "droid-99", "None available"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
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

    #[tokio::test]
    async fn dyn_resolves_fields() {
        const DOC: &str = r#"{
            hero {
                id
                info
            }
        }"#;

        for (root, expected_id, expected_info) in &[
            (QueryRoot::Human, "Non-identified", "earth"),
            (QueryRoot::Droid, "droid-99", "run"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"hero": {
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

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero {
        fn info(&self) -> Result<&str, CustomError>;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
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

    #[graphql_interface(dyn)]
    impl Hero for Human {
        fn info(&self) -> Result<&str, CustomError> {
            Ok(&self.home_planet)
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
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

    #[graphql_interface(dyn)]
    impl Hero for Droid {
        fn info(&self) -> Result<&str, CustomError> {
            Ok(&self.primary_function)
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_correct_graphql_type() {
        let schema = schema(QueryRoot::Human);

        for (interface, field) in &[("Character", "id"), ("Hero", "info")] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        name
                        kind
                        fields {{
                            name
                            type {{
                                kind
                                ofType {{
                                    name
                                }}
                            }}
                        }}
                    }}
                }}"#,
                interface,
            );

            let expected_name: &str = *interface;
            let expected_field_name: &str = *field;
            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {
                        "name": expected_name,
                        "kind": "INTERFACE",
                        "fields": [{
                            "name": expected_field_name,
                            "type": {"kind": "NON_NULL", "ofType": {"name": "String"}},
                        }]
                    }}),
                    vec![],
                )),
            );
        }
    }
}

mod generic {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character<A = (), B: ?Sized = ()> {
        fn id(&self) -> &str;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero<A, B: ?Sized> {
        fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<u8, (), __S>])]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl<A, B: ?Sized> Character<A, B> for Human {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[graphql_interface(dyn)]
    impl<A, B: ?Sized> Hero<A, B> for Human {
        fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue<(), u8>, DynHero<u8, (), __S>])]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl<A, B: ?Sized> Character<A, B> for Droid {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[graphql_interface(dyn)]
    impl<A, B: ?Sized> Hero<A, B> for Droid {
        fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, u8, (), S>> {
            let ch: Box<DynHero<'_, u8, (), _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name_without_type_params() {
        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        name
                    }}
                }}"#,
                interface,
            );

            let schema = schema(QueryRoot::Human);

            let expected_name: &str = *interface;
            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"name": expected_name}}), vec![])),
            );
        }
    }
}

mod generic_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character<A = (), B: ?Sized = ()> {
        async fn id(&self) -> &str;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero<A, B: ?Sized> {
        async fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<u8, (), __S>])]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl<A, B: ?Sized> Character<A, B> for Human {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[graphql_interface(dyn)]
    impl<A, B: ?Sized> Hero<A, B> for Human {
        async fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue<(), u8>, DynHero<u8, (), __S>])]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl<A, B: ?Sized> Character<A, B> for Droid {
        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[graphql_interface(dyn)]
    impl<A, B: ?Sized> Hero<A, B> for Droid {
        async fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, u8, (), S>> {
            let ch: Box<DynHero<'_, u8, (), _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name_without_type_params() {
        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        name
                    }}
                }}"#,
                interface,
            );

            let schema = schema(QueryRoot::Human);

            let expected_name: &str = *interface;
            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"name": expected_name}}), vec![])),
            );
        }
    }
}

mod generic_lifetime_async {
    use super::*;

    #[graphql_interface(for = [Human, Droid])]
    trait Character<'me, A> {
        async fn id<'a>(&'a self) -> &'a str;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero<'me, A> {
        async fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue<()>, DynHero<(), __S>])]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl<'me, A> Character<'me, A> for Human {
        async fn id<'a>(&'a self) -> &'a str {
            &self.id
        }
    }

    #[graphql_interface(dyn)]
    impl<'me, A> Hero<'me, A> for Human {
        async fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue<()>, DynHero<(), __S>])]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl<'me, A> Character<'me, A> for Droid {
        async fn id<'a>(&'a self) -> &'a str {
            &self.id
        }
    }

    #[graphql_interface(dyn)]
    impl<'me, A> Hero<'me, A> for Droid {
        async fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, '_, (), S>> {
            let ch: Box<DynHero<'_, '_, (), _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn uses_trait_name_without_type_params() {
        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        name
                    }}
                }}"#,
                interface,
            );

            let schema = schema(QueryRoot::Human);

            let expected_name: &str = *interface;
            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"__type": {"name": expected_name}}), vec![])),
            );
        }
    }
}

mod argument {
    use super::*;

    #[graphql_interface(for = Human)]
    trait Character {
        fn id_wide(&self, is_number: bool) -> &str;
    }

    #[graphql_interface(dyn = DynHero, for = Human)]
    trait Hero {
        fn info_wide(&self, is_planet: bool) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id_wide(&self, is_number: bool) -> &str {
            if is_number {
                &self.id
            } else {
                "none"
            }
        }
    }

    #[graphql_interface(dyn)]
    impl Hero for Human {
        fn info_wide(&self, is_planet: bool) -> &str {
            if is_planet {
                &self.home_planet
            } else {
                &self.id
            }
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            }
            .into()
        }

        fn hero(&self) -> Box<DynHero<'_, S>> {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn enum_resolves_id_field() {
        let schema = schema(QueryRoot);

        for (input, expected) in &[
            ("{ character { idWide(isNumber: true) } }", "human-32"),
            ("{ character { idWide(isNumber: false) } }", "none"),
        ] {
            let expected: &str = *expected;

            assert_eq!(
                execute(*input, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"idWide": expected}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        let schema = schema(QueryRoot);

        for (input, expected) in &[
            ("{ hero { infoWide(isPlanet: true) } }", "earth"),
            ("{ hero { infoWide(isPlanet: false) } }", "human-32"),
        ] {
            let expected: &str = *expected;

            assert_eq!(
                execute(*input, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"infoWide": expected}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn camelcases_name() {
        let schema = schema(QueryRoot);

        for (interface, field, arg) in &[
            ("Character", "idWide", "isNumber"),
            ("Hero", "infoWide", "isPlanet"),
        ] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        fields {{
                            name
                            args {{
                                name
                            }}
                        }}
                    }}
                }}"#,
                interface,
            );

            let expected_field_name: &str = *field;
            let expected_arg_name: &str = *arg;
            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"fields": [
                        {"name": expected_field_name, "args": [{"name": expected_arg_name}]},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn has_no_description() {
        let schema = schema(QueryRoot);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        fields {{
                            args {{
                                description
                            }}
                        }}
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": { "fields": [{"args": [{"description": None}]}]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn has_no_defaults() {
        let schema = schema(QueryRoot);

        for interface in &["Character", "Hero"] {
            let doc = format!(
                r#"{{
                    __type(name: "{}") {{
                        fields {{
                            args {{
                                defaultValue
                            }}
                        }}
                    }}
                }}"#,
                interface,
            );

            assert_eq!(
                execute(&doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": { "fields": [{"args": [{"defaultValue": None}]}]}}),
                    vec![],
                )),
            );
        }
    }
}

mod default_argument {
    use super::*;

    #[graphql_interface(for = Human)]
    trait Character {
        async fn id(
            &self,
            #[graphql(default)] first: String,
            #[graphql(default = "second".to_string())] second: String,
            #[graphql(default = "t")] third: String,
        ) -> String;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
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
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
            }
            .into()
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
    #[graphql_interface(for = Human)]
    trait Character {
        /// Rust `id` docs.
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
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

    #[graphql_interface(for = Human)]
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
    #[graphql(impl = CharacterValue)]
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
    #[graphql_interface(name = "MyChar", desc = "My character.", for = Human)]
    trait Character {
        /// Rust `id` docs.
        #[graphql(name = "myId", desc = "My character ID.", deprecated = "Not used.")]
        #[deprecated(note = "Should be omitted.")]
        fn id(
            &self,
            #[graphql(name = "myName", desc = "My argument.", default)] n: Option<String>,
        ) -> &str;

        #[graphql(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        fn a(&self) -> &str {
            "a"
        }

        fn b(&self) -> &str {
            "b"
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
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

    #[graphql_interface(for = [Human, Droid])]
    #[graphql_interface(scalar = DefaultScalarValue)]
    trait Character {
        fn id(&self) -> &str;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    #[graphql_interface(scalar = DefaultScalarValue)]
    trait Hero {
        async fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero], scalar = DefaultScalarValue)]
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

    #[graphql_interface(dyn, scalar = DefaultScalarValue)]
    impl Hero for Human {
        async fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero], scalar = DefaultScalarValue)]
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

    #[graphql_interface(dyn, scalar = DefaultScalarValue)]
    impl Hero for Droid {
        async fn info(&self) -> &str {
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

        fn hero(&self) -> Box<DynHero<'_>> {
            let ch: Box<DynHero<'_>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
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

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    #[graphql_interface(scalar = MyScalarValue)]
    trait Hero {
        async fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero], scalar = MyScalarValue)]
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

    #[graphql_interface(dyn, scalar = MyScalarValue)]
    impl Hero for Human {
        async fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero], scalar = MyScalarValue)]
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

    #[graphql_interface(dyn, scalar = MyScalarValue)]
    impl Hero for Droid {
        async fn info(&self) -> &str {
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

        fn hero(&self) -> Box<DynHero<'_>> {
            let ch: Box<DynHero<'_>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema_with_scalar::<MyScalarValue, _, _>(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
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

    #[graphql_interface(dyn = DynHero, for = [Human, Droid], scalar = S)]
    trait Hero<S: ScalarValue = DefaultScalarValue> {
        async fn info(&self) -> FieldResult<&str, S>;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue<__S>, DynHero<__S>])]
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

    #[graphql_interface(dyn, scalar = S)]
    impl<S: ScalarValue> Hero<S> for Human {
        async fn info(&self) -> FieldResult<&str, S> {
            Ok(&self.home_planet)
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue<__S>, DynHero<__S>])]
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

    #[graphql_interface(dyn, scalar = S)]
    impl<S: ScalarValue> Hero<S> for Droid {
        async fn info(&self) -> FieldResult<&str, S> {
            Ok(&self.primary_function)
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
        fn character(&self) -> CharacterValue<S> {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(*root);

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
            );
        }
    }
}

mod explicit_custom_context {
    use super::*;

    pub struct CustomContext;

    impl juniper::Context for CustomContext {}

    #[graphql_interface(for = [Human, Droid], context = CustomContext)]
    trait Character {
        async fn id<'a>(&'a self, context: &CustomContext) -> &'a str;

        async fn info<'b>(&'b self, ctx: &()) -> &'b str;

        fn more<'c>(&'c self, #[graphql(context)] custom: &CustomContext) -> &'c str;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    #[graphql_interface(context = CustomContext)]
    trait Hero {
        async fn id<'a>(&'a self, context: &CustomContext) -> &'a str;

        async fn info<'b>(&'b self, ctx: &()) -> &'b str;

        fn more<'c>(&'c self, #[graphql(context)] custom: &CustomContext) -> &'c str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>], context = CustomContext)]
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

    #[graphql_interface(dyn)]
    impl Hero for Human {
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
    #[graphql(impl = [CharacterValue, DynHero<__S>], context = CustomContext)]
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

    #[graphql_interface(dyn)]
    impl Hero for Droid {
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

    #[graphql_object(context = CustomContext, scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext).await,
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_fields() {
        for interface in &["character", "hero"] {
            let doc = format!(
                r#"{{
                    {} {{
                        id
                        info
                        more
                    }}
                }}"#,
                interface,
            );

            let expected_interface: &str = *interface;

            for (root, expected_id, expected_info, expexted_more) in &[
                (QueryRoot::Human, "human-32", "earth", "human"),
                (QueryRoot::Droid, "droid-99", "run", "droid"),
            ] {
                let schema = schema(*root);

                let expected_id: &str = *expected_id;
                let expected_info: &str = *expected_info;
                let expexted_more: &str = *expexted_more;
                assert_eq!(
                    execute(&doc, None, &schema, &Variables::new(), &CustomContext).await,
                    Ok((
                        graphql_value!({expected_interface: {
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
}

mod inferred_custom_context_from_field {
    use super::*;

    pub struct CustomContext(String);

    impl juniper::Context for CustomContext {}

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id<'a>(&self, context: &'a CustomContext) -> &'a str;

        fn info<'b>(&'b self, context: &()) -> &'b str;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero {
        async fn id<'a>(&self, context: &'a CustomContext) -> &'a str;

        async fn info<'b>(&'b self, context: &()) -> &'b str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>], context = CustomContext)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id<'a>(&self, ctx: &'a CustomContext) -> &'a str {
            &ctx.0
        }

        fn info<'b>(&'b self, _: &()) -> &'b str {
            &self.home_planet
        }
    }

    #[graphql_interface(dyn)]
    impl Hero for Human {
        async fn id<'a>(&self, ctx: &'a CustomContext) -> &'a str {
            &ctx.0
        }

        async fn info<'b>(&'b self, _: &()) -> &'b str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>], context = CustomContext)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        fn id<'a>(&self, ctx: &'a CustomContext) -> &'a str {
            &ctx.0
        }

        fn info<'b>(&'b self, _: &()) -> &'b str {
            &self.primary_function
        }
    }

    #[graphql_interface(dyn)]
    impl Hero for Droid {
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

    #[graphql_object(context = CustomContext, scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &ctx).await,
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
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_fields() {
        for interface in &["character", "hero"] {
            let doc = format!(
                r#"{{
                    {} {{
                        id
                        info
                    }}
                }}"#,
                interface,
            );

            let expected_interface: &str = *interface;

            for (root, expected_id, expected_info) in &[
                (QueryRoot::Human, "human-ctx", "earth"),
                (QueryRoot::Droid, "droid-ctx", "run"),
            ] {
                let schema = schema(*root);
                let ctx = CustomContext(expected_id.to_string());

                let expected_id: &str = *expected_id;
                let expected_info: &str = *expected_info;
                assert_eq!(
                    execute(&doc, None, &schema, &Variables::new(), &ctx).await,
                    Ok((
                        graphql_value!({expected_interface: {
                            "id": expected_id,
                            "info": expected_info,
                        }}),
                        vec![],
                    )),
                );
            }
        }
    }
}

mod inferred_custom_context_from_downcast {
    use super::*;

    struct Database {
        droid: Option<Droid>,
    }

    impl juniper::Context for Database {}

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        #[graphql(downcast)]
        fn as_human<'s>(&'s self, _: &Database) -> Option<&'s Human>;

        async fn id(&self) -> &str;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero {
        #[graphql(downcast)]
        fn as_droid<'db>(&self, db: &'db Database) -> Option<&'db Droid>;

        async fn info(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>], context = Database)]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn as_human<'s>(&'s self, _: &Database) -> Option<&'s Human> {
            Some(self)
        }

        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[graphql_interface(dyn)]
    impl Hero for Human {
        fn as_droid<'db>(&self, _: &'db Database) -> Option<&'db Droid> {
            None
        }

        async fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>], context = Database)]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface]
    impl Character for Droid {
        fn as_human<'s>(&'s self, _: &Database) -> Option<&'s Human> {
            None
        }

        async fn id(&self) -> &str {
            &self.id
        }
    }

    #[graphql_interface(dyn)]
    impl Hero for Droid {
        fn as_droid<'db>(&self, db: &'db Database) -> Option<&'db Droid> {
            db.droid.as_ref()
        }

        async fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = Database, scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
        let db = Database {
            droid: Some(Droid {
                id: "droid-88".to_string(),
                primary_function: "sit".to_string(),
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
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-88", "primaryFunction": "sit"}}),
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
            let schema = schema(root.clone());
            let db = Database { droid: None };

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &db).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(root.clone());
            let db = Database { droid: None };

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &db).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
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
            S: Send + Sync,
        {
            executor.look_ahead().field_name()
        }

        async fn info<'b>(
            &'b self,
            #[graphql(executor)] another: &Executor<'_, '_, (), S>,
        ) -> &'b str
        where
            S: Send + Sync;
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid], scalar = S)]
    trait Hero<S: ScalarValue> {
        async fn id<'a>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str
        where
            S: Send + Sync,
        {
            executor.look_ahead().field_name()
        }

        async fn info<'b>(
            &'b self,
            #[graphql(executor)] another: &Executor<'_, '_, (), S>,
        ) -> &'b str
        where
            S: Send + Sync;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue<__S>, DynHero<__S>])]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface(scalar = S)]
    impl<S: ScalarValue> Character<S> for Human {
        async fn info<'b>(&'b self, _: &Executor<'_, '_, (), S>) -> &'b str
        where
            S: Send + Sync,
        {
            &self.home_planet
        }
    }

    #[graphql_interface(dyn, scalar = S)]
    impl<S: ScalarValue> Hero<S> for Human {
        async fn info<'b>(&'b self, _: &Executor<'_, '_, (), S>) -> &'b str
        where
            S: Send + Sync,
        {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue<__S>, DynHero<__S>])]
    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_interface(scalar = S)]
    impl<S: ScalarValue> Character<S> for Droid {
        async fn info<'b>(&'b self, _: &Executor<'_, '_, (), S>) -> &'b str
        where
            S: Send + Sync,
        {
            &self.primary_function
        }
    }

    #[graphql_interface(dyn, scalar = S)]
    impl<S: ScalarValue> Hero<S> for Droid {
        async fn info<'b>(&'b self, _: &Executor<'_, '_, (), S>) -> &'b str
        where
            S: Send + Sync,
        {
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

        fn hero(&self) -> Box<DynHero<'_, DefaultScalarValue>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_fields() {
        for interface in &["character", "hero"] {
            let doc = format!(
                r#"{{
                    {} {{
                        id
                        info
                    }}
                }}"#,
                interface,
            );

            let expected_interface: &str = *interface;

            for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
                let schema = schema(*root);

                let expected_info: &str = *expected_info;
                assert_eq!(
                    execute(&doc, None, &schema, &Variables::new(), &()).await,
                    Ok((
                        graphql_value!({expected_interface: {"id": "id", "info": expected_info}}),
                        vec![],
                    )),
                );
            }
        }
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

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;

        #[graphql(downcast)]
        fn as_human(&self) -> Option<&Human> {
            None
        }
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    trait Hero {
        fn info(&self) -> &str;

        #[graphql(downcast)]
        fn as_droid(&self) -> Option<&Droid> {
            None
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_interface]
    impl Character for Human {
        fn id(&self) -> &str {
            &self.id
        }

        fn as_human(&self) -> Option<&Human> {
            Some(self)
        }
    }

    #[graphql_interface(dyn)]
    impl Hero for Human {
        fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>])]
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

    #[graphql_interface(dyn)]
    impl Hero for Droid {
        fn info(&self) -> &str {
            &self.primary_function
        }

        fn as_droid(&self) -> Option<&Droid> {
            Some(self)
        }
    }

    #[derive(Clone)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
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
                graphql_value!({"hero": {"droidId": "droid-99", "primaryFunction": "run"}}),
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
            let schema = schema(root.clone());

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(root.clone());

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_not_field() {
        let schema = schema(QueryRoot::Human);

        for (doc, field) in &[
            (r#"{__type(name: "Character") { fields { name } } }"#, "id"),
            (r#"{__type(name: "Hero") { fields { name } } }"#, "info"),
        ] {
            let expected_field: &str = *field;

            assert_eq!(
                execute(*doc, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({"__type": {"fields": [{"name": expected_field}]}}),
                    vec![],
                )),
            );
        }
    }
}

mod external_downcast {
    use super::*;

    struct Database {
        human: Option<Human>,
        droid: Option<Droid>,
    }

    impl juniper::Context for Database {}

    #[graphql_interface(for = [Human, Droid])]
    #[graphql_interface(context = Database, on Human = CharacterValue::as_human)]
    trait Character {
        fn id(&self) -> &str;
    }

    impl CharacterValue {
        fn as_human<'db>(&self, db: &'db Database) -> Option<&'db Human> {
            db.human.as_ref()
        }
    }

    #[graphql_interface(dyn = DynHero, for = [Human, Droid])]
    #[graphql_interface(context = Database)]
    #[graphql_interface(on Droid = DynHero::as_droid)]
    trait Hero {
        fn info(&self) -> &str;
    }

    impl<'a, S: ScalarValue> DynHero<'a, S> {
        fn as_droid<'db>(&self, db: &'db Database) -> Option<&'db Droid> {
            db.droid.as_ref()
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>], context = Database)]
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

    #[graphql_interface(dyn)]
    impl Hero for Human {
        fn info(&self) -> &str {
            &self.home_planet
        }
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [CharacterValue, DynHero<__S>], context = Database)]
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

    #[graphql_interface(dyn)]
    impl Hero for Droid {
        fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = Database, scalar = S)]
    impl<S: ScalarValue + Send + Sync> QueryRoot {
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

        fn hero(&self) -> Box<DynHero<'_, S>> {
            let ch: Box<DynHero<'_, _>> = match self {
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
        let db = Database {
            human: Some(Human {
                id: "human-64".to_string(),
                home_planet: "mars".to_string(),
            }),
            droid: None,
        };

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &db).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-64", "homePlanet": "mars"}}),
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
        let db = Database {
            human: None,
            droid: None,
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
    async fn dyn_resolves_human() {
        const DOC: &str = r#"{
            hero {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);
        let db = Database {
            human: None,
            droid: None,
        };

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &db).await,
            Ok((
                graphql_value!({"hero": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn dyn_resolves_droid() {
        const DOC: &str = r#"{
            hero {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);
        let db = Database {
            human: None,
            droid: Some(Droid {
                id: "droid-01".to_string(),
                primary_function: "swim".to_string(),
            }),
        };

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &db).await,
            Ok((
                graphql_value!({"hero": {"droidId": "droid-01", "primaryFunction": "swim"}}),
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

        let db = Database {
            human: Some(Human {
                id: "human-64".to_string(),
                home_planet: "mars".to_string(),
            }),
            droid: None,
        };

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root.clone());

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &db).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn dyn_resolves_info_field() {
        const DOC: &str = r#"{
            hero {
                info
            }
        }"#;

        let db = Database {
            human: None,
            droid: Some(Droid {
                id: "droid-01".to_string(),
                primary_function: "swim".to_string(),
            }),
        };

        for (root, expected_info) in &[(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(root.clone());

            let expected_info: &str = *expected_info;
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &db).await,
                Ok((graphql_value!({"hero": {"info": expected_info}}), vec![])),
            );
        }
    }
}
