//! Checks that long looping chain of fragments doesn't cause a stack overflow.
//!
//! ```graphql
//! # Fragment loop example
//! query {
//!     ...a
//! }
//!
//! fragment a on Query {
//!     ...b
//! }
//!
//! fragment b on Query {
//!     ...a
//! }
//! ```

use std::iter;

use itertools::Itertools as _;
use juniper::{graphql_object, graphql_vars, EmptyMutation, EmptySubscription};

struct Query;

#[graphql_object]
impl Query {
    fn dummy() -> bool {
        false
    }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation, EmptySubscription>;

#[tokio::test]
async fn test() {
    const PERM: &str = "abcefghijk";
    const CIRCLE_SIZE: usize = 7500;

    let query = iter::once(format!("query {{ ...{PERM} }} "))
        .chain(
            PERM.chars()
                .permutations(PERM.len())
                .map(|vec| vec.into_iter().collect::<String>())
                .take(CIRCLE_SIZE)
                .collect::<Vec<_>>()
                .into_iter()
                .circular_tuple_windows::<(_, _)>()
                .map(|(cur, next)| format!("fragment {cur} on Query {{ ...{next} }} ")),
        )
        .collect::<String>();

    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());
    let _ = juniper::execute(&query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap_err();
}
