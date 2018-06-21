use ast::InputValue;
use executor::Variables;
use parser::SourcePosition;
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use validation::RuleError;
use value::{Value, Object};
use GraphQLError::ValidationError;

#[derive(GraphQLEnum, Debug)]
#[graphql(_internal)]
enum Color {
    Red,
    Green,
    Blue,
}
struct TestType;

graphql_object!(TestType: () |&self| {
    field to_string(color: Color) -> String {
        format!("Color::{:?}", color)
    }

    field a_color() -> Color {
        Color::Red
    }
});

fn run_variable_query<F>(query: &str, vars: Variables, f: F)
where
    F: Fn(&Object) -> (),
{
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let (result, errs) = ::execute(query, None, &schema, &vars, &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let obj = result.as_object_value().expect("Result is not an object");

    f(obj);
}

fn run_query<F>(query: &str, f: F)
where
    F: Fn(&Object) -> (),
{
    run_variable_query(query, Variables::new(), f);
}

#[test]
fn accepts_enum_literal() {
    run_query("{ toString(color: RED) }", |result| {
        assert_eq!(result.get_field_value("toString"), Some(&Value::string("Color::Red")));
    });
}

#[test]
fn serializes_as_output() {
    run_query("{ aColor }", |result| {
        assert_eq!(result.get_field_value("aColor"), Some(&Value::string("RED")));
    });
}

#[test]
fn does_not_accept_string_literals() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"{ toString(color: "RED") }"#;
    let vars = vec![].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &()).unwrap_err();

    assert_eq!(
        error,
        ValidationError(vec![RuleError::new(
            r#"Invalid value for argument "color", expected type "Color!""#,
            &[SourcePosition::new(18, 0, 18)],
        )])
    );
}

#[test]
fn accepts_strings_in_variables() {
    run_variable_query(
        "query q($color: Color!) { toString(color: $color) }",
        vec![("color".to_owned(), InputValue::string("RED"))]
            .into_iter()
            .collect(),
        |result| {
            assert_eq!(result.get_field_value("toString"), Some(&Value::string("Color::Red")));
        },
    );
}

#[test]
fn does_not_accept_incorrect_enum_name_in_variables() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($color: Color!) { toString(color: $color) }"#;
    let vars = vec![("color".to_owned(), InputValue::string("BLURPLE"))]
        .into_iter()
        .collect();

    let error = ::execute(query, None, &schema, &vars, &()).unwrap_err();

    assert_eq!(
        error,
        ValidationError(vec![RuleError::new(
            r#"Variable "$color" got invalid value. Invalid value for enum "Color"."#,
            &[SourcePosition::new(8, 0, 8)],
        )])
    );
}

#[test]
fn does_not_accept_incorrect_type_in_variables() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($color: Color!) { toString(color: $color) }"#;
    let vars = vec![("color".to_owned(), InputValue::int(123))]
        .into_iter()
        .collect();

    let error = ::execute(query, None, &schema, &vars, &()).unwrap_err();

    assert_eq!(
        error,
        ValidationError(vec![RuleError::new(
            r#"Variable "$color" got invalid value. Expected "Color", found not a string or enum."#,
            &[SourcePosition::new(8, 0, 8)],
        )])
    );
}
