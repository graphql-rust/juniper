//! Tests for `#[derive(GraphQLInputObject)]` macro.

pub mod common;

use juniper::{
    GraphQLInputObject, ID, RuleError, execute, graphql_object, graphql_value, graphql_vars,
    parser::SourcePosition,
};

use self::common::util::schema;

// Override `std::prelude` items to check whether macros expand hygienically.
use self::common::hygiene::*;

mod trivial {
    use super::*;

    #[derive(GraphQLInputObject)]
    enum UserBy {
        Id(ID),
        Username(prelude::String),
        RegistrationNumber(i32),
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn user_info(by: UserBy) -> prelude::String {
            match by {
                UserBy::Id(id) => id.into(),
                UserBy::Username(name) => name,
                UserBy::RegistrationNumber(_) => "int".into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        // language=GraphQL
        const DOC: &str = r#"{
            userId: userInfo(by: {id: "123"})
            userName: userInfo(by: {username: "John"})
            userNum: userInfo(by: {registrationNumber: 123})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"userId": "123", "userName": "John", "userNum": "int"}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn errs_on_multiple_multiple() {
        // language=GraphQL
        const DOC: &str = r#"{
            userInfo(by: {id: "123", username: "John"})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Err(RuleError::new(
                "Invalid value for argument \"by\", reason: \
                 Exactly one key must be specified",
                &[SourcePosition::new(27, 1, 25)],
            )
            .into()),
        );
    }

    #[tokio::test]
    async fn errs_on_no_fields() {
        // language=GraphQL
        const DOC: &str = r#"{
            userInfo(by: {})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Err(RuleError::new(
                "Invalid value for argument \"by\", reason: \
                 Exactly one key must be specified",
                &[SourcePosition::new(27, 1, 25)],
            )
            .into()),
        );
    }

    #[tokio::test]
    async fn errs_on_null_field() {
        // language=GraphQL
        const DOC: &str = r#"{
            userInfo(by: {id: null})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Err(RuleError::new(
                "Invalid value for argument \"by\", reason: \
                 Value for member field \"id\" must be specified",
                &[SourcePosition::new(27, 1, 25)],
            )
            .into()),
        );
    }

    #[tokio::test]
    async fn is_graphql_input_object() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                kind
                isOneOf
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"kind": "INPUT_OBJECT", "isOneOf": true}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "UserBy"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_input_fields() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields {
                    name
                    description
                    type {
                        name
                        ofType {
                            name
                        }
                    }
                    defaultValue
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [{
                    "name": "id",
                    "description": null,
                    "type": {"name": "ID", "ofType": null},
                    "defaultValue": null,
                }, {
                    "name": "username",
                    "description": null,
                    "type": {"name": "String", "ofType": null},
                    "defaultValue": null,
                }, {
                    "name": "registrationNumber",
                    "description": null,
                    "type": {"name": "Int", "ofType": null},
                    "defaultValue": null,
                }]}}),
                vec![],
            )),
        );
    }
}

mod single {
    use super::*;

    #[derive(GraphQLInputObject)]
    enum UserBy {
        Id(ID),
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn user_info(by: UserBy) -> prelude::String {
            let UserBy::Id(id) = by;
            id.into()
        }
    }

    #[tokio::test]
    async fn resolves() {
        // language=GraphQL
        const DOC: &str = r#"{
            userInfo(by: {id: "123"})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"userInfo": "123"}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_input_object() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                kind
                isOneOf
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"kind": "INPUT_OBJECT", "isOneOf": true}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "UserBy"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_input_fields() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields {
                    name
                    description
                    type {
                        name
                        ofType {
                            name
                        }
                    }
                    defaultValue
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [{
                    "name": "id",
                    "description": null,
                    "type": {"name": "ID", "ofType": null},
                    "defaultValue": null,
                }]}}),
                vec![],
            )),
        );
    }
}

mod nested {
    use super::*;

    #[derive(GraphQLInputObject)]
    enum By {
        User(UserBy),
    }

    #[derive(GraphQLInputObject)]
    enum UserBy {
        Id(ID),
        Username(prelude::String),
        RegistrationNumber(i32),
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn user_info(by: By) -> prelude::String {
            let By::User(by) = by;
            match by {
                UserBy::Id(id) => id.into(),
                UserBy::Username(name) => name,
                UserBy::RegistrationNumber(_) => "int".into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        // language=GraphQL
        const DOC: &str = r#"{
            userId: userInfo(by: {user: {id: "123"}})
            userName: userInfo(by: {user: {username: "John"}})
            userNum: userInfo(by: {user: {registrationNumber: 123}})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"userId": "123", "userName": "John", "userNum": "int"}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn errs_on_multiple_multiple() {
        // language=GraphQL
        const DOC: &str = r#"{
            userInfo(by: {user: {id: "123", username: "John"}})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Err(RuleError::new(
                "Invalid value for argument \"by\", reason: \
                 Error on \"By\" field \"user\": \
                 Exactly one key must be specified",
                &[SourcePosition::new(27, 1, 25)],
            )
            .into()),
        );
    }

    #[tokio::test]
    async fn errs_on_no_fields() {
        // language=GraphQL
        const DOC: &str = r#"{
            userInfo(by: {user: {}})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Err(RuleError::new(
                "Invalid value for argument \"by\", reason: \
                 Error on \"By\" field \"user\": \
                 Exactly one key must be specified",
                &[SourcePosition::new(27, 1, 25)],
            )
            .into()),
        );
    }

    #[tokio::test]
    async fn errs_on_null_field() {
        // language=GraphQL
        const DOC: &str = r#"{
            userInfo(by: {user: {id: null}})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Err(RuleError::new(
                "Invalid value for argument \"by\", reason: \
                 Error on \"By\" field \"user\": \
                 Value for member field \"id\" must be specified",
                &[SourcePosition::new(27, 1, 25)],
            )
            .into()),
        );
    }
}

mod ignored_variant {
    use super::*;

    #[expect(dead_code, reason = "GraphQL schema testing")]
    #[derive(GraphQLInputObject)]
    enum UserBy {
        Id(ID),
        #[graphql(ignore)]
        Username(prelude::String),
        #[graphql(skip)]
        RegistrationNumber(i32),
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn user_info(by: UserBy) -> prelude::String {
            match by {
                UserBy::Id(id) => id.into(),
                UserBy::Username(_) => unreachable!(),
                UserBy::RegistrationNumber(_) => unreachable!(),
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        // language=GraphQL
        const DOC: &str = r#"{
            userInfo(by: {id: "123"})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"userInfo": "123"}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_input_fields() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields {
                    name
                    description
                    type {
                        name
                        ofType {
                            name
                        }
                    }
                    defaultValue
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [{
                    "name": "id",
                    "description": null,
                    "type": {"name": "ID", "ofType": null},
                    "defaultValue": null,
                }]}}),
                vec![],
            )),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    /// Selector for searching users.
    #[derive(GraphQLInputObject)]
    enum UserBy {
        /// By ID selector.
        Id(ID),

        /// By username selector.
        Username(prelude::String),
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn user_info(by: UserBy) -> prelude::String {
            match by {
                UserBy::Id(id) => id.into(),
                UserBy::Username(name) => name,
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        // language=GraphQL
        const DOC: &str = r#"{
            userId: userInfo(by: {id: "123"})
            userName: userInfo(by: {username: "John"})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"userId": "123", "userName": "John"}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_description() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "Selector for searching users.",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_input_fields_descriptions() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields {
                    name
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [{
                    "name": "id",
                    "description": "By ID selector.",
                }, {
                    "name": "username",
                    "description": "By username selector.",
                }]}}),
                vec![],
            )),
        );
    }
}

mod description_and_name_from_graphql_attr {
    use super::*;

    /// Ignored doc.
    #[derive(GraphQLInputObject)]
    #[graphql(name = "UserBy", desc = "Selector for searching users.")]
    enum By {
        /// Ignored doc.
        #[graphql(name = "ID", description = "By ID selector.")]
        Id(ID),

        /// By username selector.
        Username(prelude::String),
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn user_info(by: By) -> prelude::String {
            match by {
                By::Id(id) => id.into(),
                By::Username(name) => name,
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        // language=GraphQL
        const DOC: &str = r#"{
            userId: userInfo(by: {ID: "123"})
            userName: userInfo(by: {username: "John"})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"userId": "123", "userName": "John"}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_description_and_name() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                name
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "name": "UserBy",
                    "description": "Selector for searching users.",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_input_fields_descriptions_and_names() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields {
                    name
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [{
                    "name": "ID",
                    "description": "By ID selector.",
                }, {
                    "name": "username",
                    "description": "By username selector.",
                }]}}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_graphql_attr {
    use super::*;

    #[derive(GraphQLInputObject)]
    enum UserBy {
        Id(ID),
        #[graphql(deprecated = "Do not use.")]
        #[deprecated(note = "Should be omitted.")]
        Username(prelude::String),
        #[graphql(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        RegistrationNumber(i32),
    }

    struct QueryRoot;

    #[expect(deprecated, reason = "GraphQL schema testing")]
    #[graphql_object]
    impl QueryRoot {
        fn user_info(by: UserBy) -> prelude::String {
            match by {
                UserBy::Id(id) => id.into(),
                UserBy::Username(name) => name,
                UserBy::RegistrationNumber(_) => "int".into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        // language=GraphQL
        const DOC: &str = r#"{
            userId: userInfo(by: {id: "123"})
            userName: userInfo(by: {username: "John"})
            userNum: userInfo(by: {registrationNumber: 123})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"userId": "123", "userName": "John", "userNum": "int"}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn deprecates_fields() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields(includeDeprecated: true) {
                    name
                    isDeprecated
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [
                    {"name": "id", "isDeprecated": false},
                    {"name": "username", "isDeprecated": true},
                    {"name": "registrationNumber", "isDeprecated": true},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn provides_deprecation_reason() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields(includeDeprecated: true) {
                    name
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [
                    {"name": "id", "deprecationReason": null},
                    {"name": "username", "deprecationReason": "Do not use."},
                    {"name": "registrationNumber", "deprecationReason": null},
                ]}}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_rust_attr {
    use super::*;

    #[derive(GraphQLInputObject)]
    enum UserBy {
        Id(ID),
        #[deprecated(note = "Should be omitted.")]
        Username(prelude::String),
        #[deprecated]
        RegistrationNumber(i32),
    }

    struct QueryRoot;

    #[expect(deprecated, reason = "GraphQL schema testing")]
    #[graphql_object]
    impl QueryRoot {
        fn user_info(by: UserBy) -> prelude::String {
            match by {
                UserBy::Id(id) => id.into(),
                UserBy::Username(name) => name,
                UserBy::RegistrationNumber(_) => "int".into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        // language=GraphQL
        const DOC: &str = r#"{
            userId: userInfo(by: {id: "123"})
            userName: userInfo(by: {username: "John"})
            userNum: userInfo(by: {registrationNumber: 123})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"userId": "123", "userName": "John", "userNum": "int"}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn deprecates_fields() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields(includeDeprecated: true) {
                    name
                    isDeprecated
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [
                    {"name": "id", "isDeprecated": false},
                    {"name": "username", "isDeprecated": true},
                    {"name": "registrationNumber", "isDeprecated": true},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn provides_deprecation_reason() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields(includeDeprecated: true) {
                    name
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [
                    {"name": "id", "deprecationReason": null},
                    {"name": "username", "deprecationReason": "Should be omitted."},
                    {"name": "registrationNumber", "deprecationReason": null},
                ]}}),
                vec![],
            )),
        );
    }
}

mod renamed_all_variants {
    use super::*;

    #[derive(GraphQLInputObject)]
    #[graphql(rename_all = "none")]
    enum UserBy {
        Id(ID),
        Username(prelude::String),
        RegistrationNumber(i32),
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn user_info(by: UserBy) -> prelude::String {
            match by {
                UserBy::Id(id) => id.into(),
                UserBy::Username(name) => name,
                UserBy::RegistrationNumber(_) => "int".into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        // language=GraphQL
        const DOC: &str = r#"{
            userId: userInfo(by: {Id: "123"})
            userName: userInfo(by: {Username: "John"})
            userNum: userInfo(by: {RegistrationNumber: 123})
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"userId": "123", "userName": "John", "userNum": "int"}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_input_fields() {
        // language=GraphQL
        const DOC: &str = r#"{
            __type(name: "UserBy") {
                inputFields {
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"inputFields": [
                    {"name": "Id"},
                    {"name": "Username"},
                    {"name": "RegistrationNumber"},
                ]}}),
                vec![],
            )),
        );
    }
}
