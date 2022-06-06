//! GraphQL implementation for [`Vec`].

use crate::{
    executor::{ExecutionResult, Executor, Registry},
    graphql, reflect, resolve,
    schema::meta::MetaType,
    BoxFuture, Selection,
};

use super::iter;

impl<T, Info, S> resolve::Type<Info, S> for Vec<T>
where
    T: resolve::Type<Info, S>,
    Info: ?Sized,
{
    fn meta<'r>(registry: &mut Registry<'r, S>, info: &Info) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_list_type_new::<T, _>(info, None).into_meta()
    }
}

impl<T, Info, Ctx, S> resolve::Value<Info, Ctx, S> for Vec<T>
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
        iter::resolve_list(self.iter(), selection_set, info, executor)
    }
}

impl<T, Info, Ctx, S> resolve::ValueAsync<Info, Ctx, S> for Vec<T>
where
    T: resolve::ValueAsync<Info, Ctx, S> + Sync,
    Info: Sync + ?Sized,
    Ctx: Sync + ?Sized,
    S: Send + Sync,
{
    fn resolve_value_async<'r>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, S>]>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>> {
        Box::pin(iter::resolve_list_async(
            self.iter(),
            selection_set,
            info,
            executor,
        ))
    }
}

impl<T, S> resolve::ToInputValue<S> for Vec<T>
where
    T: resolve::ToInputValue<S>,
{
    fn to_input_value(&self) -> graphql::InputValue<S> {
        graphql::InputValue::list(self.iter().map(T::to_input_value))
    }
}

/*
impl<'i, T, Info, S> graphql::InputType<'i, Info, S> for Vec<T>
where
    T: graphql::InputType<'i, Info, S>,
    Info: ?Sized,
{
    fn assert_input_type() {
        T::assert_input_type()
    }
}
 */

impl<T, S> graphql::OutputType<S> for Vec<T>
where
    T: graphql::OutputType<S>,
{
    fn assert_output_type() {
        T::assert_output_type()
    }
}

impl<T, S> reflect::BaseType<S> for Vec<T>
where
    T: reflect::BaseType<S>,
{
    const NAME: reflect::Type = T::NAME;
}

impl<T, S> reflect::BaseSubTypes<S> for Vec<T>
where
    T: reflect::BaseSubTypes<S>,
{
    const NAMES: reflect::Types = T::NAMES;
}

impl<T, S> reflect::WrappedType<S> for Vec<T>
where
    T: reflect::WrappedType<S>,
{
    const VALUE: reflect::WrappedValue = reflect::wrap::list(T::VALUE);
}
