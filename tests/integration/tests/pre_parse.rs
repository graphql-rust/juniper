use std::pin::Pin;

use futures::{Stream, StreamExt, TryFutureExt};
use juniper::{
    executor::{execute_validated_query_async, get_operation, resolve_validated_subscription},
    graphql_object, graphql_subscription, graphql_vars,
    parser::parse_document_source,
    validation::{validate_input_values, visit_all_rules, ValidatorContext},
    EmptyMutation, FieldError, OperationType, RootNode,
};

pub struct Context;

impl juniper::Context for Context {}

pub type UserStream = Pin<Box<dyn Stream<Item = Result<User, FieldError>> + Send>>;

pub struct Query;

#[graphql_object(context = Context)]
impl Query {
    fn users() -> Vec<User> {
        vec![User]
    }
}

pub struct Subscription;

#[graphql_subscription(context = Context)]
impl Subscription {
    async fn users() -> UserStream {
        Box::pin(futures::stream::iter(vec![Ok(User)]))
    }
}

#[derive(Clone)]
pub struct User;

#[graphql_object(context = Context)]
impl User {
    fn id() -> i32 {
        1
    }
}

type Schema = RootNode<Query, EmptyMutation<Context>, Subscription>;

#[tokio::test]
async fn query_document_can_be_pre_parsed() {
    let root_node = &Schema::new(Query, EmptyMutation::<Context>::new(), Subscription);

    let document_source = r#"query { users { id } }"#;
    let document = parse_document_source(document_source, &root_node.schema).unwrap();

    {
        let mut ctx = ValidatorContext::new(&root_node.schema, &document);
        visit_all_rules(&mut ctx, &document);
        let errors = ctx.into_errors();
        assert!(errors.is_empty());
    }

    let operation = get_operation(&document, None).unwrap();
    assert!(operation.item.operation_type == OperationType::Query);

    let errors = validate_input_values(&graphql_vars! {}, operation, &root_node.schema);
    assert!(errors.is_empty());

    let (_, errors) =
        execute_validated_query_async(&document, operation, root_node, &graphql_vars! {}, &Context)
            .await
            .unwrap();

    assert!(errors.is_empty());
}

#[tokio::test]
async fn subscription_document_can_be_pre_parsed() {
    let root_node = &Schema::new(Query, EmptyMutation::<Context>::new(), Subscription);

    let document_source = r#"subscription { users { id } }"#;
    let document = parse_document_source(document_source, &root_node.schema).unwrap();

    let operation = get_operation(&document, None).unwrap();
    assert!(operation.item.operation_type == OperationType::Subscription);

    let mut stream = resolve_validated_subscription(
        &document,
        operation,
        root_node,
        &graphql_vars! {},
        &Context,
    )
    .map_ok(|(stream, errors)| juniper_subscriptions::Connection::from_stream(stream, errors))
    .await
    .unwrap();

    let _ = stream.next().await.unwrap();
}
