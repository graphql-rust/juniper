use crate::{
    executor::{ExecutionResult, Executor, Registry},
    graphql, resolve,
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

impl<T, S> graphql::InputType<S> for Vec<T>
where
    T: graphql::InputType<S>,
{
    fn assert_input_type() {
        T::assert_input_type()
    }
}

impl<T, S> graphql::OutputType<S> for Vec<T>
where
    T: graphql::OutputType<S>,
{
    fn assert_output_type() {
        T::assert_output_type()
    }
}
