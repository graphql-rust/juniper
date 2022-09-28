//! Checks that `#[graphql_object]` macro correctly expands inside a declarative
//! macro definition.
//! See [#1051](https://github.com/graphql-rust/juniper/pull/1051) and
//! [#1054](https://github.com/graphql-rust/juniper/pull/1054) for details.

use juniper::{
    graphql_object, graphql_value, graphql_vars, EmptyMutation, EmptySubscription, RootNode,
};

macro_rules! impl_id {
    ($typename:ident) => {
        #[graphql_object]
        impl $typename {
            fn id(&self) -> i32 {
                42
            }
        }
    };
}

struct Unit;
impl_id!(Unit);

#[tokio::test]
async fn works() {
    let query = r#"
        query Unit {
            id
        }
    "#;

    let schema = RootNode::new(Unit, EmptyMutation::new(), EmptySubscription::new());
    let (res, errs) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();

    assert_eq!(errs.len(), 0);
    assert_eq!(res, graphql_value!({"id": 42}));
}
