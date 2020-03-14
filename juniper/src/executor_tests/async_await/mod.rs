use crate::{RootNode, Value};

#[derive(crate::GraphQLEnumInternal)]
enum UserKind {
    Admin,
    User,
    Guest,
}

struct User {
    id: i32,
    name: String,
    kind: UserKind,
}

#[crate::graphql_object_internal]
impl User {
    async fn id(&self) -> i32 {
        self.id
    }

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
        tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
        true
    }
}

struct Query;

#[crate::graphql_object_internal]
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
        tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
        true
    }
}

struct Mutation;

#[crate::graphql_object_internal]
impl Mutation {}

#[tokio::test]
async fn async_simple() {
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
    let (res, errs) = crate::execute(doc, None, &schema, &vars, &())
        .await
        .unwrap();

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
                "name": "user1",
            },
        }),
    );
}
