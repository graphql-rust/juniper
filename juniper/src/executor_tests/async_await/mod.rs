use crate::{RootNode, Value};

#[derive(crate::GraphQLEnumInternal)]
enum UserKind {
    Admin,
    User,
    Guest,
}

struct User {
    id: u64,
    name: String,
    kind: UserKind,
}

#[crate::object_internal]
impl User {
    async fn name(&self) -> &str {
        &self.name
    }

    async fn friends(&self) -> Vec<User> {
        let friends = (0..10)
            .map(|index| User {
                id: index,
                name: format!("user{}", index),
                kind: UserKind::User,
            })
            .collect();
        friends
    }

    async fn kind(&self) -> &UserKind {
        &self.kind
    }

    async fn delayed() -> bool {
        let when = tokio::clock::now() + std::time::Duration::from_millis(100);
        tokio::timer::Delay::new(when).await;
        true
    }
}

struct Query;

#[crate::object_internal]
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
        let when = tokio::clock::now() + std::time::Duration::from_millis(100);
        tokio::timer::Delay::new(when).await;
        true
    }
}

struct Mutation;

#[crate::object_internal]
impl Mutation {}

fn run<O>(f: impl std::future::Future<Output = O>) -> O {
    tokio::runtime::current_thread::Runtime::new()
        .unwrap()
        .block_on(f)
}

#[test]
fn async_simple() {
    let schema = RootNode::new(Query, Mutation);
    let doc = r#"
        query { 
            fieldSync
            fieldAsyncPlain 
            delayed  
            user(id: "user1") {
                name
            }
        }
    "#;

    let vars = Default::default();
    let f = crate::execute_async(doc, None, &schema, &vars, &());

    let (res, errs) = run(f).unwrap();

    assert!(errs.is_empty());

    let mut obj = res.into_object().unwrap();
    obj.sort_by_field();
    let value = Value::Object(obj);

    assert_eq!(
        value,
        crate::graphql_value!({
            "delayed": true,
            "fieldAsyncPlain": "field_async_plain",
            "fieldSync": "field_sync",
            "user": {
                "kind": "USER",
                // "name": "user1",
                // "delayed": true,
            },
        }),
    );
}
