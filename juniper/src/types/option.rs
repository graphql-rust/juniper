//! GraphQL implementation for [`Option`].

use futures::future;

use crate::{
    executor::{ExecutionResult, Executor, Registry},
    graphql, resolve,
    schema::meta::MetaType,
    BoxFuture, Selection,
};

impl<T, Info, S> resolve::Type<Info, S> for Option<T>
where
    T: resolve::Type<Info, S>,
    Info: ?Sized,
{
    fn meta<'r>(registry: &mut Registry<'r, S>, info: &Info) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_nullable_type_new::<T, _>(info).into_meta()
    }
}

impl<T, Info, Ctx, S> resolve::Value<Info, Ctx, S> for Option<T>
where
    T: resolve::Value<Info, Ctx, S>,
    Info: ?Sized,
    Ctx: ?Sized,
{
    fn resolve_value(
        &self,
        selection_set: Option<&[Selection<'_, S>]>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S> {
        match self {
            Some(v) => v.resolve_value(selection_set, info, executor),
            None => Ok(graphql::Value::Null),
        }
    }
}

impl<T, Info, Ctx, S> resolve::ValueAsync<Info, Ctx, S> for Option<T>
where
    T: resolve::ValueAsync<Info, Ctx, S>,
    Info: ?Sized,
    Ctx: ?Sized,
    S: Send,
{
    fn resolve_value_async<'r>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, S>]>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>> {
        match self {
            Some(v) => v.resolve_value_async(selection_set, info, executor),
            None => Box::pin(future::ok(graphql::Value::Null)),
        }
    }
}

impl<T, S> graphql::InputType<S> for Option<T>
where
    T: graphql::InputType<S>,
{
    fn assert_input_type() {
        T::assert_input_type()
    }
}

impl<T, S> graphql::OutputType<S> for Option<T>
where
    T: graphql::OutputType<S>,
{
    fn assert_output_type() {
        T::assert_output_type()
    }
}
