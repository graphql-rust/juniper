//! Checks that non-`Null` variables may carry a default value, per [§6.1.2]
//! of the GraphQL spec.
//! See [#1376](https://github.com/graphql-rust/juniper/pull/1376) for details.
//!
//! [§6.1.2]: https://spec.graphql.org/October2021/#sec-Coercing-Variable-Values

use juniper::{
    EmptyMutation, EmptySubscription, RootNode, graphql_object, graphql_value, graphql_vars,
};

pub struct Query;

#[graphql_object]
impl Query {
    fn hello() -> &'static str {
        "world"
    }
}

type Schema = RootNode<Query, EmptyMutation, EmptySubscription>;

const QUERY: &str = r#"
    query ($var: Boolean! = true) {
        __typename @skip(if: $var)
    }
"#;

#[tokio::test]
async fn default_applies_when_variable_not_provided() {
    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

    assert_eq!(
        juniper::execute(QUERY, None, &schema, &graphql_vars! {}, &()).await,
        Ok((graphql_value!({}), vec![])),
    );
}

#[tokio::test]
async fn provided_variable_overrides_default() {
    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

    assert_eq!(
        juniper::execute(QUERY, None, &schema, &graphql_vars! {"var": false}, &()).await,
        Ok((graphql_value!({"__typename": "Query"}), vec![])),
    );
}
