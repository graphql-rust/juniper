use juniper::*;

use futures::stream::BoxStream;

#[derive(juniper::GraphQLObject)]
struct User {
    name: String,
}

struct Error;

impl<S: ScalarValue> IntoFieldError<S> for Error {
    fn into_field_error(self) -> FieldError<S> {
        let a = Value::Scalar(S::from(42));
        let mut extensions = Object::with_capacity(1);
        let _ = extensions.add_field("a", a);
        FieldError::new("oops", Value::Object(extensions))
    }
}

struct SubscriptionsRoot;

#[graphql_subscription(name = "Subscription")]
impl SubscriptionsRoot {
    async fn users() -> Result<BoxStream<'static, User>, Error> {
        Ok(users_stream()?)
    }
}

fn users_stream() -> Result<BoxStream<'static, User>, Error> {
    Err(Error)
}

struct Query;

#[juniper::graphql_object]
impl Query {
    fn users() -> Vec<User> {
        vec![]
    }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation<()>, SubscriptionsRoot>;

#[tokio::test]
async fn test_error_extensions() {
    let sub = r#"
        subscription Users {
            users {
                name
            }
        }
    "#;

    let (_, errors) = juniper::resolve_into_stream(
        sub,
        None,
        &Schema::new(Query, EmptyMutation::new(), SubscriptionsRoot),
        &Variables::new(),
        &(),
    )
    .await
    .unwrap();

    assert_eq!(
        errors.first().unwrap().error().extensions(),
        &graphql_value!({ "a": 42 })
    );
}
