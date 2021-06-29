use juniper::{
    graphql_object, graphql_value, EmptyMutation, EmptySubscription, GraphQLEnum, GraphQLError,
    RootNode, Value,
};

#[derive(GraphQLEnum)]
enum UserKind {
    Admin,
    User,
    Guest,
}

struct User {
    #[allow(dead_code)]
    id: i32,
    name: String,
    kind: UserKind,
}

#[graphql_object]
impl User {
    async fn id(&self) -> i32 {
        self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn friends(&self) -> Vec<User> {
        (0..10)
            .map(|index| User {
                id: index,
                name: format!("user{}", index),
                kind: UserKind::User,
            })
            .collect()
    }

    async fn kind(&self) -> &UserKind {
        &self.kind
    }

    async fn delayed() -> bool {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        true
    }
}

struct Query;

#[graphql_object]
impl Query {
    fn field_sync(&self) -> &'static str {
        "field_sync"
    }

    async fn field_async_plain() -> String {
        "field_async_plain".to_string()
    }

    fn user(id: String) -> User {
        User {
            id: 1,
            name: id,
            kind: UserKind::User,
        }
    }

    async fn delayed() -> bool {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        true
    }
}

#[tokio::test]
async fn async_simple() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
    let doc = r#"
        query {
            fieldSync
            fieldAsyncPlain
            delayed
            user(id: "user1") {
                kind
                name
                delayed
            }
        }
    "#;

    let vars = Default::default();
    let (res, errs) = juniper::execute(doc, None, &schema, &vars, &())
        .await
        .unwrap();

    assert!(errs.is_empty());

    let obj = res.into_object().unwrap();
    let value = Value::Object(obj);

    assert_eq!(
        value,
        graphql_value!({
            "delayed": true,
            "fieldAsyncPlain": "field_async_plain",
            "fieldSync": "field_sync",
            "user": {
                "delayed": true,
                "kind": "USER",
                "name": "user1",
            },
        }),
    );
}

#[tokio::test]
async fn async_field_validation_error() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
    let doc = r#"
        query {
            nonExistentField
            fieldSync
            fieldAsyncPlain
            delayed
            user(id: "user1") {
                kind
                name
                delayed
            }
        }
    "#;

    let vars = Default::default();
    let result = juniper::execute(doc, None, &schema, &vars, &()).await;
    assert!(result.is_err());

    let error = result.err().unwrap();
    let is_validation_error = match error {
        GraphQLError::ValidationError(_) => true,
        _ => false,
    };
    assert!(is_validation_error);
}

// FIXME: test seems broken by design, re-enable later
// #[tokio::test]
// async fn resolve_into_stream_validation_error() {
//     let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
//     let doc = r#"
//         subscription {
//             nonExistent
//         }
//     "#;
//     let vars = Default::default();
//     let result = juniper::resolve_into_stream(doc, None, &schema, &vars, &()).await;
//     assert!(result.is_err());

//     let error = result.err().unwrap();
//     let is_validation_error = match error {
//         GraphQLError::ValidationError(_) => true,
//         _ => false,
//     };
//     assert!(is_validation_error);
// }

fn main() {}
