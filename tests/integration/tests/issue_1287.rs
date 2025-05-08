//! Checks that error is propagated from a fragment the same way as without it.
//! See [#1287](https://github.com/graphql-rust/juniper/issues/1287) for details.

use juniper::{EmptyMutation, EmptySubscription, Variables, graphql_object};

struct MyObject;

#[graphql_object]
impl MyObject {
    fn erroring_field() -> Result<i32, &'static str> {
        Err("This field always errors")
    }
}

struct Query;

#[graphql_object]
impl Query {
    fn my_object() -> MyObject {
        MyObject
    }

    fn just_a_field() -> i32 {
        3
    }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation, EmptySubscription>;

#[tokio::test]
async fn error_propagates_same_way() {
    // language=GraphQL
    let without_fragment = r"{ 
        myObject { erroringField } 
        justAField 
    }";
    // language=GraphQL
    let with_fragment = r"
        query {
            myObject {
                ...MyObjectFragment
            }
            justAField
        }

        fragment MyObjectFragment on MyObject {
            erroringField
        }
    ";
    
    let (expected, _) = juniper::execute(
        without_fragment,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .await
    .unwrap();
    let (actual, _) = juniper::execute(
        with_fragment,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .await
    .unwrap();

    assert_eq!(actual, expected);
}
