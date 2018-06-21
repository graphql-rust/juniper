use executor::Variables;
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use value::{Value, Object};

struct TestType;

graphql_object!(TestType: () |&self| {
    field a() -> &str {
        "a"
    }

    field b() -> &str {
        "b"
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
fn scalar_include_true() {
    run_query("{ a, b @include(if: true) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), Some(&Value::string("b")));
    });
}

#[test]
fn scalar_include_false() {
    run_query("{ a, b @include(if: false) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), None);
    });
}

#[test]
fn scalar_skip_false() {
    run_query("{ a, b @skip(if: false) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), Some(&Value::string("b")));
    });
}

#[test]
fn scalar_skip_true() {
    run_query("{ a, b @skip(if: true) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), None);
    });
}

#[test]
fn fragment_spread_include_true() {
    run_query(
        "{ a, ...Frag @include(if: true) } fragment Frag on TestType { b }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
            assert_eq!(result.get_field_value("b"), Some(&Value::string("b")));
        },
    );
}

#[test]
fn fragment_spread_include_false() {
    run_query(
        "{ a, ...Frag @include(if: false) } fragment Frag on TestType { b }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
            assert_eq!(result.get_field_value("b"), None);
        },
    );
}

#[test]
fn fragment_spread_skip_false() {
    run_query(
        "{ a, ...Frag @skip(if: false) } fragment Frag on TestType { b }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
            assert_eq!(result.get_field_value("b"), Some(&Value::string("b")));
        },
    );
}

#[test]
fn fragment_spread_skip_true() {
    run_query(
        "{ a, ...Frag @skip(if: true) } fragment Frag on TestType { b }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
            assert_eq!(result.get_field_value("b"), None);
        },
    );
}

#[test]
fn inline_fragment_include_true() {
    run_query(
        "{ a, ... on TestType @include(if: true) { b } }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
            assert_eq!(result.get_field_value("b"), Some(&Value::string("b")));
        },
    );
}

#[test]
fn inline_fragment_include_false() {
    run_query(
        "{ a, ... on TestType @include(if: false) { b } }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
            assert_eq!(result.get_field_value("b"), None);
        },
    );
}

#[test]
fn inline_fragment_skip_false() {
    run_query("{ a, ... on TestType @skip(if: false) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), Some(&Value::string("b")));
    });
}

#[test]
fn inline_fragment_skip_true() {
    run_query("{ a, ... on TestType @skip(if: true) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), None);
    });
}

#[test]
fn anonymous_inline_fragment_include_true() {
    run_query("{ a, ... @include(if: true) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), Some(&Value::string("b")));
    });
}

#[test]
fn anonymous_inline_fragment_include_false() {
    run_query("{ a, ... @include(if: false) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), None);
    });
}

#[test]
fn anonymous_inline_fragment_skip_false() {
    run_query("{ a, ... @skip(if: false) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), Some(&Value::string("b")));
    });
}

#[test]
fn anonymous_inline_fragment_skip_true() {
    run_query("{ a, ... @skip(if: true) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), None);
    });
}

#[test]
fn scalar_include_true_skip_true() {
    run_query("{ a, b @include(if: true) @skip(if: true) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), None);
    });
}

#[test]
fn scalar_include_true_skip_false() {
    run_query("{ a, b @include(if: true) @skip(if: false) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), Some(&Value::string("b")));
    });
}

#[test]
fn scalar_include_false_skip_true() {
    run_query("{ a, b @include(if: false) @skip(if: true) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), None);
    });
}

#[test]
fn scalar_include_false_skip_false() {
    run_query("{ a, b @include(if: false) @skip(if: false) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&Value::string("a")));
        assert_eq!(result.get_field_value("b"), None);
    });
}
