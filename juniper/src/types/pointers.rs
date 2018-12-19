use ast::{FromInputValue, InputValue, Selection, ToInputValue};
use std::fmt::Debug;
use std::sync::Arc;

use executor::{ExecutionResult, Executor, Registry};
use schema::meta::MetaType;
use types::base::{Arguments, GraphQLType};
use value::{ScalarRefValue, ScalarValue, Value};

impl<S, T, CtxT> GraphQLType<S> for Box<T>
where
    S: ScalarValue,
    T: GraphQLType<S, Context = CtxT>,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(info: &T::TypeInfo) -> Option<&str> {
        T::name(info)
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        T::meta(info, registry)
    }

    fn resolve_into_type(
        &self,
        info: &T::TypeInfo,
        name: &str,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_into_type(info, name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        info: &T::TypeInfo,
        field: &str,
        args: &Arguments<S>,
        executor: &Executor<CtxT, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_field(info, field, args, executor)
    }

    fn resolve(
        &self,
        info: &T::TypeInfo,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> Value<S> {
        (**self).resolve(info, selection_set, executor)
    }
}

impl<T, S> FromInputValue<S> for Box<T>
where
    S: ScalarValue,
    T: FromInputValue<S>,
{
    fn from_input_value<'a>(v: &'a InputValue<S>) -> Option<Box<T>>
    where
        for<'b> &'b S: ScalarRefValue<'b>,
    {
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

impl<'a, S, T, CtxT> GraphQLType<S> for &'a T
where
    S: ScalarValue,
    T: GraphQLType<S, Context = CtxT>,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(info: &T::TypeInfo) -> Option<&str> {
        T::name(info)
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        T::meta(info, registry)
    }

    fn resolve_into_type(
        &self,
        info: &T::TypeInfo,
        name: &str,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_into_type(info, name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        info: &T::TypeInfo,
        field: &str,
        args: &Arguments<S>,
        executor: &Executor<CtxT, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_field(info, field, args, executor)
    }

    fn resolve(
        &self,
        info: &T::TypeInfo,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> Value<S> {
        (**self).resolve(info, selection_set, executor)
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
    T: GraphQLType<S>,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn name(info: &T::TypeInfo) -> Option<&str> {
        T::name(info)
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        T::meta(info, registry)
    }

    fn resolve_into_type(
        &self,
        info: &T::TypeInfo,
        name: &str,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<T::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_into_type(info, name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        info: &T::TypeInfo,
        field: &str,
        args: &Arguments<S>,
        executor: &Executor<T::Context, S>,
    ) -> ExecutionResult<S> {
        (**self).resolve_field(info, field, args, executor)
    }

    fn resolve(
        &self,
        info: &T::TypeInfo,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<T::Context, S>,
    ) -> Value<S> {
        (**self).resolve(info, selection_set, executor)
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
