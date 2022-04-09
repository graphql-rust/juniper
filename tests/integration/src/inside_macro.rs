use juniper::{graphql_value, graphql_vars, EmptyMutation, EmptySubscription, RootNode};

macro_rules! impl_id {
    ($typename:ident) => {
        #[juniper::graphql_object]
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
    let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(res, graphql_value!({"id": 42}));
}
