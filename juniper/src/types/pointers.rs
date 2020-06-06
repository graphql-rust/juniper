use crate::{
    ast::{FromInputValue, InputValue, Selection, ToInputValue},
    executor::{ExecutionResult, Executor, Registry},
    schema::meta::MetaType,
    types::base::{Arguments, GraphQLType},
    value::ScalarValue,
    BoxFuture,
};
use std::{fmt::Debug, sync::Arc};

impl<S, T, CtxT> GraphQLType<S> for Box<T>
where
    S: ScalarValue,
    T: GraphQLType<S, Context = CtxT> + Send + Sync,
    CtxT: Send + Sync,
    T::TypeInfo: Send + Sync,
    T::Context: Send + Sync,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(info: &T::TypeInfo) -> Option<&str> {
        T::name(info)
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        T::meta(info, registry)
    }

    fn resolve_into_type<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        type_name: &'name str,
        selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        Box::pin((**self).resolve_into_type(info, type_name, selection_set, executor))
    }

    fn resolve_field<'me, 'ty, 'field, 'args, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        field_name: &'field str,
        arguments: &'args Arguments<'args, S>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'args: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
        'field: 'fut,
        S: 'fut,
    {
        Box::pin((**self).resolve_field(info, field_name, arguments, executor))
    }

    fn resolve<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        Box::pin((**self).resolve(info, selection_set, executor))
    }
}

impl<T, S> FromInputValue<S> for Box<T>
where
    S: ScalarValue,
    T: FromInputValue<S>,
{
    fn from_input_value<'a>(v: &'a InputValue<S>) -> Option<Box<T>> {
        match <T as FromInputValue<S>>::from_input_value(v) {
            Some(v) => Some(Box::new(v)),
            None => None,
        }
    }
}

impl<T, S> ToInputValue<S> for Box<T>
where
    S: Debug,
    T: ToInputValue<S>,
{
    fn to_input_value(&self) -> InputValue<S> {
        (**self).to_input_value()
    }
}

impl<'e, S, T, CtxT> GraphQLType<S> for &'e T
where
    S: ScalarValue,
    T: GraphQLType<S, Context = CtxT> + Send + Sync,
    T::TypeInfo: Send + Sync,
    T::Context: Send + Sync,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(info: &T::TypeInfo) -> Option<&str> {
        T::name(info)
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        T::meta(info, registry)
    }

    #[allow(unused_variables)]
    fn resolve_into_type<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        type_name: &'name str,
        selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        Box::pin((**self).resolve_into_type(info, type_name, selection_set, executor))
    }

    fn resolve_field<'me, 'ty, 'field, 'args, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        field_name: &'field str,
        arguments: &'args Arguments<'args, S>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'args: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
        'field: 'fut,
        S: 'fut,
    {
        Box::pin((**self).resolve_field(info, field_name, arguments, executor))
    }

    fn resolve<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        Box::pin((**self).resolve(info, selection_set, executor))
    }
}

impl<'a, T, S> ToInputValue<S> for &'a T
where
    S: Debug,
    T: ToInputValue<S>,
{
    fn to_input_value(&self) -> InputValue<S> {
        (**self).to_input_value()
    }
}

impl<S, T> GraphQLType<S> for Arc<T>
where
    S: ScalarValue,
    T: GraphQLType<S> + Send + Sync,
    T::TypeInfo: Send + Sync,
    T::Context: Send + Sync,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn name(info: &T::TypeInfo) -> Option<&str> {
        T::name(info)
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        T::meta(info, registry)
    }

    fn resolve_into_type<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        type_name: &'name str,
        selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        Box::pin((**self).resolve_into_type(info, type_name, selection_set, executor))
    }

    fn resolve_field<'me, 'ty, 'field, 'args, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        field_name: &'field str,
        arguments: &'args Arguments<'args, S>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'args: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
        'field: 'fut,
        S: 'fut,
    {
        Box::pin((**self).resolve_field(info, field_name, arguments, executor))
    }

    fn resolve<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        Box::pin((**self).resolve(info, selection_set, executor))
    }
}

impl<T, S> ToInputValue<S> for Arc<T>
where
    S: Debug,
    T: ToInputValue<S>,
{
    fn to_input_value(&self) -> InputValue<S> {
        (**self).to_input_value()
    }
}
