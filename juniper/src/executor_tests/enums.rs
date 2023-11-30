use crate::{
    executor::Variables,
    graphql_value, graphql_vars,
    parser::SourcePosition,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    validation::RuleError,
    value::{DefaultScalarValue, Object},
    GraphQLEnum,
    GraphQLError::ValidationError,
};

#[derive(GraphQLEnum, Debug)]
enum Color {
    Red,
    Green,
    Blue,
}

struct TestType;

#[crate::graphql_object]
impl TestType {
    fn to_string(color: Color) -> String {
        format!("Color::{color:?}")
    }

    fn a_color() -> Color {
        Color::Red
    }
}

async fn run_variable_query<F>(query: &str, vars: Variables<DefaultScalarValue>, f: F)
where
    F: Fn(&Object<DefaultScalarValue>),
{
    let schema = RootNode::new(
        TestType,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = crate::execute(query, None, &schema, &vars, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {result:#?}");

    let obj = result.as_object_value().expect("Result is not an object");

    f(obj);
}

async fn run_query<F>(query: &str, f: F)
where
    F: Fn(&Object<DefaultScalarValue>),
{
    run_variable_query(query, Variables::new(), f).await;
}

#[tokio::test]
async fn accepts_enum_literal() {
    run_query("{ toString(color: RED) }", |result| {
        assert_eq!(
            result.get_field_value("toString"),
            Some(&graphql_value!("Color::Red")),
        );
    })
    .await;
}

#[tokio::test]
async fn serializes_as_output() {
    run_query("{ aColor }", |result| {
        assert_eq!(
            result.get_field_value("aColor"),
            Some(&graphql_value!("RED")),
        );
    })
    .await;
}

#[tokio::test]
async fn does_not_accept_string_literals() {
    let schema = RootNode::new(
        TestType,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let query = r#"{ toString(color: "RED") }"#;
    let vars = graphql_vars! {};

    let error = crate::execute(query, None, &schema, &vars, &())
        .await
        .unwrap_err();

    assert_eq!(
        error,
        ValidationError(vec![RuleError::new(
            r#"Invalid value for argument "color", reason: Invalid value ""RED"" for enum "Color""#,
            &[SourcePosition::new(18, 0, 18)],
        )])
    );
}

#[tokio::test]
async fn accepts_strings_in_variables() {
    run_variable_query(
        "query q($color: Color!) { toString(color: $color) }",
        graphql_vars! {"color": "RED"},
        |result| {
            assert_eq!(
                result.get_field_value("toString"),
                Some(&graphql_value!("Color::Red")),
            );
        },
    )
    .await;
}

#[tokio::test]
async fn does_not_accept_incorrect_enum_name_in_variables() {
    let schema = RootNode::new(
        TestType,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let query = r#"query q($color: Color!) { toString(color: $color) }"#;
    let vars = graphql_vars! {"color": "BLURPLE"};

    let error = crate::execute(query, None, &schema, &vars, &())
        .await
        .unwrap_err();

    assert_eq!(
        error,
        ValidationError(vec![RuleError::new(
            r#"Variable "$color" got invalid value. Invalid value for enum "Color"."#,
            &[SourcePosition::new(8, 0, 8)],
        )]),
    );
}

#[tokio::test]
async fn does_not_accept_incorrect_type_in_variables() {
    let schema = RootNode::new(
        TestType,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let query = r#"query q($color: Color!) { toString(color: $color) }"#;
    let vars = graphql_vars! {"color": 123};

    let error = crate::execute(query, None, &schema, &vars, &())
        .await
        .unwrap_err();

    assert_eq!(
        error,
        ValidationError(vec![RuleError::new(
            r#"Variable "$color" got invalid value. Expected "Color", found not a string or enum."#,
            &[SourcePosition::new(8, 0, 8)],
        )]),
    );
}
