use juniper::{
    EmptyMutation, EmptySubscription, GraphQLInputObject, RootNode, graphql_object, graphql_value,
    graphql_vars,
};

mod as_output_field {
    use super::*;

    struct Query;

    #[graphql_object]
    impl Query {
        fn roll() -> [bool; 3] {
            [true, false, true]
        }
    }

    #[tokio::test]
    async fn works() {
        let query = r#"
            query Query {
                roll
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(res, graphql_value!({"roll": [true, false, true]}));
    }
}

mod as_input_field {
    use super::*;

    #[derive(GraphQLInputObject)]
    struct Input {
        two: [bool; 2],
    }

    #[derive(GraphQLInputObject)]
    struct InputSingle {
        one: [bool; 1],
    }

    struct Query;

    #[graphql_object]
    impl Query {
        fn first(input: InputSingle) -> bool {
            input.one[0]
        }

        fn second(input: Input) -> bool {
            input.two[1]
        }
    }

    #[tokio::test]
    async fn works() {
        let query = r#"
            query Query {
                second(input: { two: [true, false] })
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(res, graphql_value!({"second": false}));
    }

    #[tokio::test]
    async fn fails_on_incorrect_count() {
        let query = r#"
            query Query {
                second(input: { two: [true, true, false] })
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let res = juniper::execute(query, None, &schema, &graphql_vars! {}, &()).await;

        assert!(res.is_err(), "result succeeded: {res:#?}");
        assert_eq!(
            res.unwrap_err().to_string(),
            "Invalid value for argument \"input\", reason: Error on \"Input\" field \"two\": \
             Expected list of length 2, but \"[true, true, false]\" has length 3. At 2:30\n",
        );
    }

    #[tokio::test]
    async fn cannot_coerce_from_raw_value_if_multiple() {
        let query = r#"
            query Query {
                second(input: { two: true })
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let res = juniper::execute(query, None, &schema, &graphql_vars! {}, &()).await;

        assert!(res.is_err(), "result succeeded: {res:#?}");
        assert_eq!(
            res.unwrap_err().to_string(),
            "Invalid value for argument \"input\", reason: Error on \"Input\" field \"two\": \
             Expected list of length 2, but \"true\" has length 1. At 2:30\n",
        );
    }

    #[tokio::test]
    async fn can_coerce_from_raw_value_if_single() {
        let query = r#"
            query Query {
                first(input: { one: true })
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(res, graphql_value!({"first": true}));
    }
}

mod as_input_argument {
    use super::*;

    struct Query;

    #[graphql_object]
    impl Query {
        fn second(input: [bool; 2]) -> bool {
            input[1]
        }

        fn first(input: [bool; 1]) -> bool {
            input[0]
        }

        fn third(#[graphql(default = [true, false, false])] input: [bool; 3]) -> bool {
            input[2]
        }
    }

    #[tokio::test]
    async fn works() {
        let query = r#"
            query Query {
                second(input: [false, true])
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(res, graphql_value!({"second": true}));
    }

    #[tokio::test]
    async fn fails_on_incorrect_count() {
        let query = r#"
            query Query {
                second(input: [true, true, false])
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let res = juniper::execute(query, None, &schema, &graphql_vars! {}, &()).await;

        assert!(res.is_err(), "result succeeded: {res:#?}");
        assert_eq!(
            res.unwrap_err().to_string(),
            "Invalid value for argument \"input\", reason: Expected list of length 2, \
             but \"[true, true, false]\" has length 3. At 2:30\n",
        );
    }

    #[tokio::test]
    async fn cannot_coerce_from_raw_value_if_multiple() {
        let query = r#"
            query Query {
                second(input: true)
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let res = juniper::execute(query, None, &schema, &graphql_vars! {}, &()).await;

        assert!(res.is_err(), "result succeeded: {res:#?}");
        assert_eq!(
            res.unwrap_err().to_string(),
            "Invalid value for argument \"input\", reason: Expected list of length 2, \
             but \"true\" has length 1. At 2:30\n",
            "invalid error returned",
        );
    }

    #[tokio::test]
    async fn can_coerce_from_raw_value_if_single() {
        let query = r#"
            query Query {
                first(input: true)
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(res, graphql_value!({"first": true}));
    }

    #[tokio::test]
    async fn picks_default() {
        let query = r#"
            query Query {
                third
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(res, graphql_value!({"third": false}));
    }

    #[tokio::test]
    async fn picks_specified_over_default() {
        let query = r#"
            query Query {
                third(input: [false, false, true])
            }
        "#;

        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(res, graphql_value!({"third": true}));
    }
}
