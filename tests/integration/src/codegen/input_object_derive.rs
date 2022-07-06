//! Tests for `#[derive(GraphQLInputObject)]` macro.

use juniper::{
    execute, graphql_object, graphql_value, graphql_vars, parser::SourcePosition, GraphQLError,
    GraphQLInputObject, RuleError,
};

use crate::util::schema;

mod trivial {
    use super::*;

    #[derive(GraphQLInputObject)]
    struct Point2D {
        x: f64,
        y: f64,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn x(point: Point2D) -> f64 {
            point.x
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            x(point: { x: 10, y: 20 })
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"x": 10.0}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_input_object() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INPUT_OBJECT"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Point2D"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
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
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                inputFields {
                    name
                    description
                    type {
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
                graphql_value!({"__type": {"inputFields": [
                    {
                        "name": "x",
                        "description": null,
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                    {
                        "name": "y",
                        "description": null,
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod default_value {
    use super::*;

    #[derive(GraphQLInputObject)]
    struct Point2D {
        #[graphql(default = 10.0)]
        x: f64,
        #[graphql(default = 10.0)]
        y: f64,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn x(point: Point2D) -> f64 {
            point.x
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"query q($x4: Float!) {
            x(point: { y: 20 })
            x2: x(point: { x: 20 })
            x3: x(point: {})
            x4: x(point: { x: $x4 })
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {"x4": 40}, &()).await,
            Ok((
                graphql_value!({
                    "x": 10.0,
                    "x2": 20.0,
                    "x3": 10.0,
                    "x4": 40.0,
                }),
                vec![]
            )),
        );
    }

    #[tokio::test]
    async fn err_on_null() {
        const DOC: &str = r#"{ x(point: { y: null }) }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Err(GraphQLError::ValidationError(vec![RuleError::new(
                "Invalid value for argument \"point\", expected type \"Point2D!\"",
                &[SourcePosition::new(11, 0, 11)],
            )]))
        );
    }

    #[tokio::test]
    async fn err_on_missing_var() {
        const DOC: &str = r#"query q($x: Float!){ x(point: { x: $x }) }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Err(GraphQLError::ValidationError(vec![RuleError::new(
                "Variable \"$x\" of required type \"Float!\" was not provided.",
                &[SourcePosition::new(8, 0, 8)],
            )]))
        );
    }

    #[tokio::test]
    async fn is_graphql_input_object() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INPUT_OBJECT"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_input_fields() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                inputFields {
                    name
                    description
                    type {
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
                graphql_value!({"__type": {"inputFields": [
                    {
                        "name": "x",
                        "description": null,
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": "10",
                    },
                    {
                        "name": "y",
                        "description": null,
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": "10",
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod default_nullable_value {
    use super::*;

    #[derive(GraphQLInputObject)]
    struct Point2D {
        #[graphql(default = 10.0)]
        x: Option<f64>,
        #[graphql(default = 10.0)]
        y: Option<f64>,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn x(point: Point2D) -> Option<f64> {
            point.x
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"query q(
            $x6: Float, 
            $x7: Float, 
            $x8: Float,
            $x9: Float = 40,
            $x10: Float = 50,
            $x11: Float = 60,
        ) {
            x(point: { y: 20 })
            x2: x(point: { x: 20 })
            x3: x(point: {})
            x4: x(point: { x: null })
            x5: x(point: { y: null })
            x6: x(point: { x: $x6 })
            x7: x(point: { x: $x7 })
            x8: x(point: { x: $x8 })
            x9: x(point: { x: $x9 })
            x10: x(point: { x: $x10 })
            x11: x(point: { x: $x11 })
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(
                DOC,
                None,
                &schema,
                &graphql_vars! {
                    "x6": 30.0,
                    "x7": null,
                    "x9": 100,
                    "x10": null,
                },
                &(),
            )
            .await,
            Ok((
                graphql_value!({
                    "x": 10.0,
                    "x2": 20.0,
                    "x3": 10.0,
                    "x4": null,
                    "x5": 10.0,
                    "x6": 30.0,
                    "x7": null,
                    "x8": 10.0,
                    "x9": 100.0,
                    "x10": null,
                    "x11": 60.0,
                }),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn is_graphql_input_object() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INPUT_OBJECT"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_input_fields() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
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
                graphql_value!({"__type": {"inputFields": [
                    {
                        "name": "x",
                        "description": null,
                        "type": {"name": "Float", "ofType": null},
                        "defaultValue": "10",
                    },
                    {
                        "name": "y",
                        "description": null,
                        "type": {"name": "Float", "ofType": null},
                        "defaultValue": "10",
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod ignored_field {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum System {
        Cartesian,
    }

    #[derive(GraphQLInputObject)]
    struct Point2D {
        x: f64,
        y: f64,
        #[graphql(ignore)]
        shift: f64,
        #[graphql(skip, default = System::Cartesian)]
        system: System,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn x(point: Point2D) -> f64 {
            assert_eq!(point.shift, f64::default());
            assert_eq!(point.system, System::Cartesian);
            point.x
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            x(point: { x: 10, y: 20 })
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"x": 10.0}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_input_object() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INPUT_OBJECT"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Point2D"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
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
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                inputFields {
                    name
                    description
                    type {
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
                graphql_value!({"__type": {"inputFields": [
                    {
                        "name": "x",
                        "description": null,
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                    {
                        "name": "y",
                        "description": null,
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    /// Point in a Cartesian system.
    #[derive(GraphQLInputObject)]
    struct Point2D {
        /// Abscissa value.
        x: f64,

        /// Ordinate value.
        y_coord: f64,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn x(point: Point2D) -> f64 {
            point.x
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            x(point: { x: 10, yCoord: 20 })
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"x": 10.0}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_input_object() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INPUT_OBJECT"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Point2D"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_description() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "Point in a Cartesian system.",
                }}),
                vec![]
            )),
        );
    }

    #[tokio::test]
    async fn has_input_fields() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                inputFields {
                    name
                    description
                    type {
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
                graphql_value!({"__type": {"inputFields": [
                    {
                        "name": "x",
                        "description": "Abscissa value.",
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                    {
                        "name": "yCoord",
                        "description": "Ordinate value.",
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod description_from_graphql_attr {
    use super::*;

    /// Ignored doc.
    #[derive(GraphQLInputObject)]
    #[graphql(name = "Point", desc = "Point in a Cartesian system.")]
    struct Point2D {
        /// Ignored doc.
        #[graphql(name = "x", description = "Abscissa value.")]
        x_coord: f64,

        /// Ordinate value.
        y: f64,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn x(point: Point2D) -> f64 {
            point.x_coord
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            x(point: { x: 10, y: 20 })
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"x": 10.0}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_input_object() {
        const DOC: &str = r#"{
            __type(name: "Point") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INPUT_OBJECT"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
        const DOC: &str = r#"{
            __type(name: "Point") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Point"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_description() {
        const DOC: &str = r#"{
            __type(name: "Point") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "Point in a Cartesian system.",
                }}),
                vec![]
            )),
        );
    }

    #[tokio::test]
    async fn has_input_fields() {
        const DOC: &str = r#"{
            __type(name: "Point") {
                inputFields {
                    name
                    description
                    type {
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
                graphql_value!({"__type": {"inputFields": [
                    {
                        "name": "x",
                        "description": "Abscissa value.",
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                    {
                        "name": "y",
                        "description": "Ordinate value.",
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod renamed_all_fields {
    use super::*;

    #[derive(GraphQLInputObject)]
    #[graphql(rename_all = "none")]
    struct Point2D {
        x_coord: f64,
        y: f64,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn x(point: Point2D) -> f64 {
            point.x_coord
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            x(point: { x_coord: 10, y: 20 })
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"x": 10.0}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_input_object() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INPUT_OBJECT"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_input_fields() {
        const DOC: &str = r#"{
            __type(name: "Point2D") {
                inputFields {
                    name
                    description
                    type {
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
                graphql_value!({"__type": {"inputFields": [
                    {
                        "name": "x_coord",
                        "description": null,
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                    {
                        "name": "y",
                        "description": null,
                        "type": {"ofType": {"name": "Float"}},
                        "defaultValue": null,
                    },
                ]}}),
                vec![],
            )),
        );
    }
}
