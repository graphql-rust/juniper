use std::sync::Arc;

use arcstr::ArcStr;

use crate::{
    BoxFuture,
    ast::{FromInputValue, InputValue, Selection, ToInputValue},
    executor::{ExecutionResult, Executor, Registry},
    schema::meta::MetaType,
    types::{
        async_await::GraphQLValueAsync,
        base::{Arguments, GraphQLType, GraphQLValue},
    },
    value::{FromScalarValue, ScalarValue, ToScalarValue},
};

impl<S, T> GraphQLType<S> for Box<T>
where
    T: GraphQLType<S> + ?Sized,
    S: ScalarValue,
{
    fn name(info: &Self::TypeInfo) -> Option<ArcStr> {
        T::name(info)
    }

    fn meta(info: &Self::TypeInfo, registry: &mut Registry<S>) -> MetaType<S> {
        T::meta(info, registry)
    }
}

impl<S, T> GraphQLValue<S> for Box<T>
where
    T: GraphQLValue<S> + ?Sized,
    S: ScalarValue,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        (**self).type_name(info)
    }

    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        name: &str,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_into_type(info, name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        args: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_field(info, field, args, executor)
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve(info, selection_set, executor)
    }
}

impl<S, T> GraphQLValueAsync<S> for Box<T>
where
    T: GraphQLValueAsync<S> + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        (**self).resolve_async(info, selection_set, executor)
    }
}

impl<'s, T, S> FromScalarValue<'s, S> for Box<T>
where
    S: ScalarValue,
    T: FromScalarValue<'s, S> + 's,
{
    type Error = T::Error;

    fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error> {
        T::from_scalar_value(v).map(Self::new)
    }
}

impl<T, S> ToScalarValue<S> for Box<T>
where
    T: ToScalarValue<S> + ?Sized,
{
    fn to_scalar_value(&self) -> S {
        (**self).to_scalar_value()
    }
}

impl<T, S> FromInputValue<S> for Box<T>
where
    S: ScalarValue,
    T: FromInputValue<S>,
{
    type Error = T::Error;

    fn from_input_value(v: &InputValue<S>) -> Result<Box<T>, Self::Error> {
        <T as FromInputValue<S>>::from_input_value(v).map(Box::new)
    }
}

impl<T, S> ToInputValue<S> for Box<T>
where
    T: ToInputValue<S> + ?Sized,
{
    fn to_input_value(&self) -> InputValue<S> {
        (**self).to_input_value()
    }
}

impl<S, T> GraphQLType<S> for &T
where
    T: GraphQLType<S> + ?Sized,
    S: ScalarValue,
{
    fn name(info: &Self::TypeInfo) -> Option<ArcStr> {
        T::name(info)
    }

    fn meta(info: &Self::TypeInfo, registry: &mut Registry<S>) -> MetaType<S> {
        T::meta(info, registry)
    }
}

impl<S, T> GraphQLValue<S> for &T
where
    S: ScalarValue,
    T: GraphQLValue<S> + ?Sized,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        (**self).type_name(info)
    }

    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        name: &str,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_into_type(info, name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        args: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_field(info, field, args, executor)
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve(info, selection_set, executor)
    }
}

impl<S, T> GraphQLValueAsync<S> for &T
where
    T: GraphQLValueAsync<S> + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_field_async<'b>(
        &'b self,
        info: &'b Self::TypeInfo,
        field_name: &'b str,
        arguments: &'b Arguments<S>,
        executor: &'b Executor<Self::Context, S>,
    ) -> BoxFuture<'b, ExecutionResult<S>> {
        (**self).resolve_field_async(info, field_name, arguments, executor)
    }

    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        (**self).resolve_async(info, selection_set, executor)
    }
}

impl<T, S> ToScalarValue<S> for &T
where
    T: ToScalarValue<S> + ?Sized,
{
    fn to_scalar_value(&self) -> S {
        (**self).to_scalar_value()
    }
}

impl<T, S> ToInputValue<S> for &T
where
    T: ToInputValue<S> + ?Sized,
{
    fn to_input_value(&self) -> InputValue<S> {
        (**self).to_input_value()
    }
}

impl<S, T> GraphQLType<S> for Arc<T>
where
    S: ScalarValue,
    T: GraphQLType<S> + ?Sized,
{
    fn name(info: &Self::TypeInfo) -> Option<ArcStr> {
        T::name(info)
    }

    fn meta(info: &Self::TypeInfo, registry: &mut Registry<S>) -> MetaType<S> {
        T::meta(info, registry)
    }
}

impl<S, T> GraphQLValue<S> for Arc<T>
where
    S: ScalarValue,
    T: GraphQLValue<S> + ?Sized,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        (**self).type_name(info)
    }

    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        name: &str,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_into_type(info, name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        args: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_field(info, field, args, executor)
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve(info, selection_set, executor)
    }
}

impl<S, T> GraphQLValueAsync<S> for Arc<T>
where
    T: GraphQLValueAsync<S> + Send + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        (**self).resolve_async(info, selection_set, executor)
    }
}

impl<'s, T, S> FromScalarValue<'s, S> for Arc<T>
where
    S: ScalarValue,
    T: FromScalarValue<'s, S> + 's,
{
    type Error = T::Error;

    fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error> {
        T::from_scalar_value(v).map(Self::new)
    }
}

impl<T, S> ToScalarValue<S> for Arc<T>
where
    T: ToScalarValue<S> + ?Sized,
{
    fn to_scalar_value(&self) -> S {
        (**self).to_scalar_value()
    }
}

impl<T, S> FromInputValue<S> for Arc<T>
where
    S: ScalarValue,
    T: FromInputValue<S>,
{
    type Error = T::Error;

    fn from_input_value(v: &InputValue<S>) -> Result<Arc<T>, Self::Error> {
        <T as FromInputValue<S>>::from_input_value(v).map(Arc::new)
    }
}

impl<T, S> ToInputValue<S> for Arc<T>
where
    T: ToInputValue<S> + ?Sized,
{
    fn to_input_value(&self) -> InputValue<S> {
        (**self).to_input_value()
    }
}
