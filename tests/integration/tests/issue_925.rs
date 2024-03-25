//! Checks that `FieldError` doesn't lose its extensions while being implicitly
//! converted from user defined subscription errors.
//! See [#925](https://github.com/graphql-rust/juniper/issues/925) for details.

use futures::stream::BoxStream;
use juniper::{
    graphql_object, graphql_subscription, graphql_value, graphql_vars, EmptyMutation, FieldError,
    GraphQLObject, IntoFieldError, Object, ScalarValue, Value,
};

#[derive(GraphQLObject)]
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
        users_stream()
    }
}

fn users_stream() -> Result<BoxStream<'static, User>, Error> {
    Err(Error)
}

struct Query;

#[graphql_object]
impl Query {
    fn users() -> Vec<User> {
        vec![]
    }
}

type Schema = juniper::RootNode<Query, EmptyMutation, SubscriptionsRoot>;

#[tokio::test]
async fn error_extensions() {
    let sub = r#"
        subscription Users {
            users {
                name
            }
        }
    "#;

    let schema = Schema::new(Query, EmptyMutation::new(), SubscriptionsRoot);
    let (_, errors) = juniper::resolve_into_stream(sub, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();

    assert_eq!(
        errors.first().unwrap().error().extensions(),
        &graphql_value!({ "a": 42 }),
    );
}
