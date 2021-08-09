use std::{collections::HashMap, pin::Pin};

use futures::{stream, StreamExt as _};

use crate::{
    graphql_object, graphql_subscription, graphql_value, resolve_into_stream, DefaultScalarValue,
    EmptyMutation, Executor, RootNode, Value,
};

#[derive(Default)]
struct Context {
    flag1: bool,
}

impl crate::Context for Context {}

struct WithLifetime<'a> {
    value: &'a str,
}

#[graphql_object(context = Context)]
impl<'a> WithLifetime<'a> {
    fn value(&self) -> &str {
        self.value
    }
}

struct WithContext;

#[graphql_object(context = Context)]
impl WithContext {
    fn ctx(ctx: &Context) -> bool {
        ctx.flag1
    }
}

#[derive(Default)]
struct Query;

#[graphql_object(context = Context)]
impl Query {
    fn empty() -> bool {
        true
    }
}

#[derive(Default)]
struct Mutation;

#[graphql_object(context = Context)]
impl Mutation {
    fn empty() -> bool {
        true
    }
}

type Stream<I> = Pin<Box<dyn futures::Stream<Item = I> + Send>>;

#[derive(Default)]
struct Subscription {
    b: bool,
}

#[graphql_subscription(
    name = "Subscription",
    context = Context,
    scalar = DefaultScalarValue,
)]
/// Subscription Description.
impl Subscription {
    async fn with_executor(_executor: &Executor<'_, '_, Context>) -> Stream<bool> {
        Box::pin(stream::once(async { true }))
    }

    async fn with_executor_and_self(&self, _executor: &Executor<'_, '_, Context>) -> Stream<bool> {
        Box::pin(stream::once(async { true }))
    }

    async fn with_context_child(&self) -> Stream<WithContext> {
        Box::pin(stream::once(async { WithContext }))
    }

    async fn with_implicit_lifetime_child(&self) -> Stream<WithLifetime<'static>> {
        Box::pin(stream::once(async { WithLifetime { value: "blub" } }))
    }
}
