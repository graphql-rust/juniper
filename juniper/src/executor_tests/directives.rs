use crate::{
    executor::Variables,
    graphql_value,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    value::{DefaultScalarValue, Object},
};

struct TestType;

#[crate::graphql_object]
impl TestType {
    fn a() -> &'static str {
        "a"
    }

    fn b() -> &'static str {
        "b"
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
async fn scalar_include_true() {
    run_query("{ a, b @include(if: true) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), Some(&graphql_value!("b")));
    })
    .await;
}

#[tokio::test]
async fn scalar_include_false() {
    run_query("{ a, b @include(if: false) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), None);
    })
    .await;
}

#[tokio::test]
async fn scalar_skip_false() {
    run_query("{ a, b @skip(if: false) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), Some(&graphql_value!("b")));
    })
    .await;
}

#[tokio::test]
async fn scalar_skip_true() {
    run_query("{ a, b @skip(if: true) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), None);
    })
    .await;
}

#[tokio::test]
async fn fragment_spread_include_true() {
    run_query(
        "{ a, ...Frag @include(if: true) } fragment Frag on TestType { b }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
            assert_eq!(result.get_field_value("b"), Some(&graphql_value!("b")));
        },
    )
    .await;
}

#[tokio::test]
async fn fragment_spread_include_false() {
    run_query(
        "{ a, ...Frag @include(if: false) } fragment Frag on TestType { b }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
            assert_eq!(result.get_field_value("b"), None);
        },
    )
    .await;
}

#[tokio::test]
async fn fragment_spread_skip_false() {
    run_query(
        "{ a, ...Frag @skip(if: false) } fragment Frag on TestType { b }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
            assert_eq!(result.get_field_value("b"), Some(&graphql_value!("b")));
        },
    )
    .await;
}

#[tokio::test]
async fn fragment_spread_skip_true() {
    run_query(
        "{ a, ...Frag @skip(if: true) } fragment Frag on TestType { b }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
            assert_eq!(result.get_field_value("b"), None);
        },
    )
    .await;
}

#[tokio::test]
async fn inline_fragment_include_true() {
    run_query(
        "{ a, ... on TestType @include(if: true) { b } }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
            assert_eq!(result.get_field_value("b"), Some(&graphql_value!("b")));
        },
    )
    .await;
}

#[tokio::test]
async fn inline_fragment_include_false() {
    run_query(
        "{ a, ... on TestType @include(if: false) { b } }",
        |result| {
            assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
            assert_eq!(result.get_field_value("b"), None);
        },
    )
    .await;
}

#[tokio::test]
async fn inline_fragment_skip_false() {
    run_query("{ a, ... on TestType @skip(if: false) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), Some(&graphql_value!("b")));
    })
    .await;
}

#[tokio::test]
async fn inline_fragment_skip_true() {
    run_query("{ a, ... on TestType @skip(if: true) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), None);
    })
    .await;
}

#[tokio::test]
async fn anonymous_inline_fragment_include_true() {
    run_query("{ a, ... @include(if: true) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), Some(&graphql_value!("b")));
    })
    .await;
}

#[tokio::test]
async fn anonymous_inline_fragment_include_false() {
    run_query("{ a, ... @include(if: false) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), None);
    })
    .await;
}

#[tokio::test]
async fn anonymous_inline_fragment_skip_false() {
    run_query("{ a, ... @skip(if: false) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), Some(&graphql_value!("b")));
    })
    .await;
}

#[tokio::test]
async fn anonymous_inline_fragment_skip_true() {
    run_query("{ a, ... @skip(if: true) { b } }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), None);
    })
    .await;
}

#[tokio::test]
async fn scalar_include_true_skip_true() {
    run_query("{ a, b @include(if: true) @skip(if: true) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), None);
    })
    .await;
}

#[tokio::test]
async fn scalar_include_true_skip_false() {
    run_query("{ a, b @include(if: true) @skip(if: false) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), Some(&graphql_value!("b")));
    })
    .await;
}

#[tokio::test]
async fn scalar_include_false_skip_true() {
    run_query("{ a, b @include(if: false) @skip(if: true) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), None);
    })
    .await;
}

#[tokio::test]
async fn scalar_include_false_skip_false() {
    run_query("{ a, b @include(if: false) @skip(if: false) }", |result| {
        assert_eq!(result.get_field_value("a"), Some(&graphql_value!("a")));
        assert_eq!(result.get_field_value("b"), None);
    })
    .await;
}
