use std::collections::HashMap;

use value::Value;
use ast::InputValue;
use executor::Variables;
use schema::model::RootNode;
use ::GraphQLError::ValidationError;
use validation::RuleError;
use parser::SourcePosition;
use types::scalars::EmptyMutation;

#[derive(Debug)]
struct TestComplexScalar;

struct TestType;

graphql_scalar!(TestComplexScalar {
    resolve(&self) -> Value {
        Value::string("SerializedValue")
    }

    from_input_value(v: &InputValue) -> Option<TestComplexScalar> {
        if let Some(s) = v.as_string_value() {
            if s == "SerializedValue" {
                return Some(TestComplexScalar);
            }
        }

        None
    }
});


graphql_input_object!(
    #[derive(Debug)]
    struct TestInputObject {
        a: Option<String>,
        b: Option<Vec<Option<String>>>,
        c: String,
        d: Option<TestComplexScalar>,
    }
);

graphql_input_object!(
    #[derive(Debug)]
    struct TestNestedInputObject {
        na: TestInputObject,
        nb: String,
    }
);

graphql_object!(TestType: () |&self| {
    field field_with_object_input(input: Option<TestInputObject>) -> String {
        format!("{:?}", input)
    }

    field field_with_nullable_string_input(input: Option<String>) -> String {
        format!("{:?}", input)
    }

    field field_with_non_nullable_string_input(input: String) -> String {
        format!("{:?}", input)
    }

    field field_with_default_argument_value(input = ("Hello World".to_owned()): String) -> String {
        format!("{:?}", input)
    }

    field field_with_nested_object_input(input: Option<TestNestedInputObject>) -> String {
        format!("{:?}", input)
    }

    field list(input: Option<Vec<Option<String>>>) -> String {
        format!("{:?}", input)
    }

    field nn_list(input: Vec<Option<String>>) -> String {
        format!("{:?}", input)
    }

    field list_nn(input: Option<Vec<String>>) -> String {
        format!("{:?}", input)
    }

    field nn_list_nn(input: Vec<String>) -> String {
        format!("{:?}", input)
    }
});

fn run_variable_query<F>(query: &str, vars: Variables, f: F)
    where F: Fn(&HashMap<String, Value>) -> ()
{
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let (result, errs) = ::execute(query, None, &schema, &vars, &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let obj = result.as_object_value().expect("Result is not an object");

    f(obj);
}

fn run_query<F>(query: &str, f: F)
    where F: Fn(&HashMap<String, Value>) -> ()
{
    run_variable_query(query, Variables::new(), f);
}

#[test]
fn inline_complex_input() {
    run_query(
        r#"{ fieldWithObjectInput(input: {a: "foo", b: ["bar"], c: "baz"}) }"#,
        |result| {
            assert_eq!(
                result.get("fieldWithObjectInput"),
                Some(&Value::string(r#"Some(TestInputObject { a: Some("foo"), b: Some([Some("bar")]), c: "baz", d: None })"#)));
        });
}

#[test]
fn inline_parse_single_value_to_list() {
    run_query(
        r#"{ fieldWithObjectInput(input: {a: "foo", b: "bar", c: "baz"}) }"#,
        |result| {
            assert_eq!(
                result.get("fieldWithObjectInput"),
                Some(&Value::string(r#"Some(TestInputObject { a: Some("foo"), b: Some([Some("bar")]), c: "baz", d: None })"#)));
        });
}

#[test]
fn inline_runs_from_input_value_on_scalar() {
    run_query(
        r#"{ fieldWithObjectInput(input: {c: "baz", d: "SerializedValue"}) }"#,
        |result| {
            assert_eq!(
                result.get("fieldWithObjectInput"),
                Some(&Value::string(r#"Some(TestInputObject { a: None, b: None, c: "baz", d: Some(TestComplexScalar) })"#)));
        });
}

#[test]
fn variable_complex_input() {
    run_variable_query(
        r#"query q($input: TestInputObject) { fieldWithObjectInput(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::object(vec![
                ("a", InputValue::string("foo")),
                ("b", InputValue::list(vec![InputValue::string("bar")])),
                ("c", InputValue::string("baz")),
            ].into_iter().collect())),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("fieldWithObjectInput"),
                Some(&Value::string(r#"Some(TestInputObject { a: Some("foo"), b: Some([Some("bar")]), c: "baz", d: None })"#)));
        });
}

#[test]
fn variable_parse_single_value_to_list() {
    run_variable_query(
        r#"query q($input: TestInputObject) { fieldWithObjectInput(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::object(vec![
                ("a", InputValue::string("foo")),
                ("b", InputValue::string("bar")),
                ("c", InputValue::string("baz")),
            ].into_iter().collect())),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("fieldWithObjectInput"),
                Some(&Value::string(r#"Some(TestInputObject { a: Some("foo"), b: Some([Some("bar")]), c: "baz", d: None })"#)));
        });
}

#[test]
fn variable_runs_from_input_value_on_scalar() {
    run_variable_query(
        r#"query q($input: TestInputObject) { fieldWithObjectInput(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::object(vec![
                ("c", InputValue::string("baz")),
                ("d", InputValue::string("SerializedValue")),
            ].into_iter().collect())),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("fieldWithObjectInput"),
                Some(&Value::string(r#"Some(TestInputObject { a: None, b: None, c: "baz", d: Some(TestComplexScalar) })"#)));
        });
}

#[test]
fn variable_error_on_nested_non_null() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: TestInputObject) { fieldWithObjectInput(input: $input) }"#;
    let vars = vec![
        ("input".to_owned(), InputValue::object(vec![
            ("a", InputValue::string("foo")),
            ("b", InputValue::string("bar")),
            ("c", InputValue::null()),
        ].into_iter().collect()))
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" got invalid value. In field "c": Expected "String!", found null."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn variable_error_on_incorrect_type() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: TestInputObject) { fieldWithObjectInput(input: $input) }"#;
    let vars = vec![
        ("input".to_owned(), InputValue::string("foo bar")),
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" got invalid value. Expected "TestInputObject", found not an object."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn variable_error_on_omit_non_null() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: TestInputObject) { fieldWithObjectInput(input: $input) }"#;
    let vars = vec![
        ("input".to_owned(), InputValue::object(vec![
            ("a", InputValue::string("foo")),
            ("b", InputValue::string("bar")),
        ].into_iter().collect()))
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" got invalid value. In field "c": Expected "String!", found null."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn variable_multiple_errors_with_nesting() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: TestNestedInputObject) { fieldWithNestedObjectInput(input: $input) }"#;
    let vars = vec![
        ("input".to_owned(), InputValue::object(vec![
            ("na", InputValue::object(vec![
                ("a", InputValue::string("foo")),
            ].into_iter().collect())),
        ].into_iter().collect()))
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" got invalid value. In field "na": In field "c": Expected "String!", found null."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
        RuleError::new(
            r#"Variable "$input" got invalid value. In field "nb": Expected "String!", found null."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn variable_error_on_additional_field() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: TestInputObject) { fieldWithObjectInput(input: $input) }"#;
    let vars = vec![
        ("input".to_owned(), InputValue::object(vec![
            ("a", InputValue::string("foo")),
            ("b", InputValue::string("bar")),
            ("c", InputValue::string("baz")),
            ("extra", InputValue::string("dog")),
        ].into_iter().collect()))
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" got invalid value. In field "extra": Unknown field."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn allow_nullable_inputs_to_be_omitted() {
    run_query(
        r#"{ fieldWithNullableStringInput }"#,
        |result| {
            assert_eq!(
                result.get("fieldWithNullableStringInput"),
                Some(&Value::string(r#"None"#)));
        });
}

#[test]
fn allow_nullable_inputs_to_be_omitted_in_variable() {
    run_query(
        r#"query q($value: String) { fieldWithNullableStringInput(input: $value) }"#,
        |result| {
            assert_eq!(
                result.get("fieldWithNullableStringInput"),
                Some(&Value::string(r#"None"#)));
        });
}

#[test]
fn allow_nullable_inputs_to_be_set_to_null_in_variable() {
    run_variable_query(
        r#"query q($value: String) { fieldWithNullableStringInput(input: $value) }"#,
        vec![
            ("value".to_owned(), InputValue::null()),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("fieldWithNullableStringInput"),
                Some(&Value::string(r#"None"#)));
        });
}

#[test]
fn allow_nullable_inputs_to_be_set_to_value_in_variable() {
    run_variable_query(
        r#"query q($value: String) { fieldWithNullableStringInput(input: $value) }"#,
        vec![
            ("value".to_owned(), InputValue::string("a")),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("fieldWithNullableStringInput"),
                Some(&Value::string(r#"Some("a")"#)));
        });
}

#[test]
fn allow_nullable_inputs_to_be_set_to_value_directly() {
    run_query(
        r#"{ fieldWithNullableStringInput(input: "a") }"#,
        |result| {
            assert_eq!(
                result.get("fieldWithNullableStringInput"),
                Some(&Value::string(r#"Some("a")"#)));
        });
}

#[test]
fn does_not_allow_non_nullable_input_to_be_omitted_in_variable() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($value: String!) { fieldWithNonNullableStringInput(input: $value) }"#;
    let vars = vec![
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$value" of required type "String!" was not provided."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn does_not_allow_non_nullable_input_to_be_set_to_null_in_variable() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($value: String!) { fieldWithNonNullableStringInput(input: $value) }"#;
    let vars = vec![
        ("value".to_owned(), InputValue::null()),
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$value" of required type "String!" was not provided."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn allow_non_nullable_inputs_to_be_set_to_value_in_variable() {
    run_variable_query(
        r#"query q($value: String!) { fieldWithNonNullableStringInput(input: $value) }"#,
        vec![
            ("value".to_owned(), InputValue::string("a")),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("fieldWithNonNullableStringInput"),
                Some(&Value::string(r#""a""#)));
        });
}

#[test]
fn allow_non_nullable_inputs_to_be_set_to_value_directly() {
    run_query(
        r#"{ fieldWithNonNullableStringInput(input: "a") }"#,
        |result| {
            assert_eq!(
                result.get("fieldWithNonNullableStringInput"),
                Some(&Value::string(r#""a""#)));
        });
}

#[test]
fn allow_lists_to_be_null() {
    run_variable_query(
        r#"query q($input: [String]) { list(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::null()),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("list"),
                Some(&Value::string(r#"None"#)));
        });
}

#[test]
fn allow_lists_to_contain_values() {
    run_variable_query(
        r#"query q($input: [String]) { list(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::list(vec![
                InputValue::string("A"),
            ])),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("list"),
                Some(&Value::string(r#"Some([Some("A")])"#)));
        });
}

#[test]
fn allow_lists_to_contain_null() {
    run_variable_query(
        r#"query q($input: [String]) { list(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::list(vec![
                InputValue::string("A"),
                InputValue::null(),
                InputValue::string("B"),
            ])),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("list"),
                Some(&Value::string(r#"Some([Some("A"), None, Some("B")])"#)));
        });
}

#[test]
fn does_not_allow_non_null_lists_to_be_null() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: [String]!) { nnList(input: $input) }"#;
    let vars = vec![
        ("input".to_owned(), InputValue::null()),
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" of required type "[String]!" was not provided."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn allow_non_null_lists_to_contain_values() {
    run_variable_query(
        r#"query q($input: [String]!) { nnList(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::list(vec![
                InputValue::string("A"),
            ])),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("nnList"),
                Some(&Value::string(r#"[Some("A")]"#)));
        });
}
#[test]
fn allow_non_null_lists_to_contain_null() {
    run_variable_query(
        r#"query q($input: [String]!) { nnList(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::list(vec![
                InputValue::string("A"),
                InputValue::null(),
                InputValue::string("B"),
            ])),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("nnList"),
                Some(&Value::string(r#"[Some("A"), None, Some("B")]"#)));
        });
}

#[test]
fn allow_lists_of_non_null_to_be_null() {
    run_variable_query(
        r#"query q($input: [String!]) { listNn(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::null()),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("listNn"),
                Some(&Value::string(r#"None"#)));
        });
}

#[test]
fn allow_lists_of_non_null_to_contain_values() {
    run_variable_query(
        r#"query q($input: [String!]) { listNn(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::list(vec![
                InputValue::string("A"),
            ])),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("listNn"),
                Some(&Value::string(r#"Some(["A"])"#)));
        });
}

#[test]
fn does_not_allow_lists_of_non_null_to_contain_null() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: [String!]) { listNn(input: $input) }"#;
    let vars = vec![
        ("input".to_owned(), InputValue::list(vec![
            InputValue::string("A"),
            InputValue::null(),
            InputValue::string("B"),
        ])),
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" got invalid value. In element #1: Expected "String!", found null."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn does_not_allow_non_null_lists_of_non_null_to_contain_null() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: [String!]!) { nnListNn(input: $input) }"#;
    let vars = vec![
        ("input".to_owned(), InputValue::list(vec![
            InputValue::string("A"),
            InputValue::null(),
            InputValue::string("B"),
        ])),
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" got invalid value. In element #1: Expected "String!", found null."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn does_not_allow_non_null_lists_of_non_null_to_be_null() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: [String!]!) { nnListNn(input: $input) }"#;
    let vars = vec![
        ("value".to_owned(), InputValue::null()),
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" of required type "[String!]!" was not provided."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn allow_non_null_lists_of_non_null_to_contain_values() {
    run_variable_query(
        r#"query q($input: [String!]!) { nnListNn(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::list(vec![
                InputValue::string("A"),
            ])),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("nnListNn"),
                Some(&Value::string(r#"["A"]"#)));
        });
}

#[test]
fn does_not_allow_invalid_types_to_be_used_as_values() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: TestType!) { fieldWithObjectInput(input: $input) }"#;
    let vars = vec![
        ("value".to_owned(), InputValue::list(vec![
            InputValue::string("A"),
            InputValue::string("B"),
        ])),
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" expected value of type "TestType!" which cannot be used as an input type."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn does_not_allow_unknown_types_to_be_used_as_values() {
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let query = r#"query q($input: UnknownType!) { fieldWithObjectInput(input: $input) }"#;
    let vars = vec![
        ("value".to_owned(), InputValue::list(vec![
            InputValue::string("A"),
            InputValue::string("B"),
        ])),
    ].into_iter().collect();

    let error = ::execute(query, None, &schema, &vars, &())
        .unwrap_err();

    assert_eq!(error, ValidationError(vec![
        RuleError::new(
            r#"Variable "$input" expected value of type "UnknownType!" which cannot be used as an input type."#,
            &[SourcePosition::new(8, 0, 8)],
        ),
    ]));
}

#[test]
fn default_argument_when_not_provided() {
    run_query(
        r#"{ fieldWithDefaultArgumentValue }"#,
        |result| {
            assert_eq!(
                result.get("fieldWithDefaultArgumentValue"),
                Some(&Value::string(r#""Hello World""#)));
        });
}

#[test]
fn default_argument_when_nullable_variable_not_provided() {
    run_query(
        r#"query q($input: String) { fieldWithDefaultArgumentValue(input: $input) }"#,
        |result| {
            assert_eq!(
                result.get("fieldWithDefaultArgumentValue"),
                Some(&Value::string(r#""Hello World""#)));
        });
}

#[test]
fn default_argument_when_nullable_variable_set_to_null() {
    run_variable_query(
        r#"query q($input: String) { fieldWithDefaultArgumentValue(input: $input) }"#,
        vec![
            ("input".to_owned(), InputValue::null()),
        ].into_iter().collect(),
        |result| {
            assert_eq!(
                result.get("fieldWithDefaultArgumentValue"),
                Some(&Value::string(r#""Hello World""#)));
        });
}
