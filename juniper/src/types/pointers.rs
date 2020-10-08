use std::sync::Arc;

use crate::{
    ast::{FromInputValue, InputValue, Selection, ToInputValue},
    executor::{ExecutionResult, Executor, Registry},
    schema::meta::MetaType,
    types::{
        async_await::GraphQLValueAsync,
        base::{Arguments, GraphQLType, GraphQLValue},
    },
    BoxFuture,
};

impl<T> GraphQLType for Box<T>
where
    T: GraphQLType + ?Sized,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        T::name(info)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r> {
        T::meta(info, registry)
    }
}

impl<T> GraphQLValue for Box<T>
where
    T: GraphQLValue + ?Sized,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        (**self).type_name(info)
    }

    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        name: &str,
        selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        (**self).resolve_into_type(info, name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        args: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        (**self).resolve_field(info, field, args, executor)
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        (**self).resolve(info, selection_set, executor)
    }
}

impl<T> GraphQLValueAsync for Box<T>
where
    T: GraphQLValueAsync + Sync + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection]>,
        executor: &'a Executor<Self::Context>,
    ) -> BoxFuture<'a, ExecutionResult> {
        (**self).resolve_async(info, selection_set, executor)
    }
}

impl<T> FromInputValue for Box<T>
where
    T: FromInputValue,
{
    fn from_input_value(v: &InputValue) -> Option<Box<T>> {
        match <T as FromInputValue>::from_input_value(v) {
            Some(v) => Some(Box::new(v)),
            None => None,
        }
    }
}

impl<T> ToInputValue for Box<T>
where
    T: ToInputValue,
{
    fn to_input_value(&self) -> InputValue {
        (**self).to_input_value()
    }
}

impl<'e, T> GraphQLType for &'e T
where
    T: GraphQLType + ?Sized,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        T::name(info)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r> {
        T::meta(info, registry)
    }
}

impl<'e, T> GraphQLValue for &'e T
where
    T: GraphQLValue + ?Sized,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        (**self).type_name(info)
    }

    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        name: &str,
        selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        (**self).resolve_into_type(info, name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        args: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        (**self).resolve_field(info, field, args, executor)
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        (**self).resolve(info, selection_set, executor)
    }
}

impl<'e, T> GraphQLValueAsync for &'e T
where
    T: GraphQLValueAsync + Sync + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
{
    fn resolve_field_async<'b>(
        &'b self,
        info: &'b Self::TypeInfo,
        field_name: &'b str,
        arguments: &'b Arguments,
        executor: &'b Executor<Self::Context>,
    ) -> BoxFuture<'b, ExecutionResult> {
        (**self).resolve_field_async(info, field_name, arguments, executor)
    }

    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection]>,
        executor: &'a Executor<Self::Context>,
    ) -> BoxFuture<'a, ExecutionResult> {
        (**self).resolve_async(info, selection_set, executor)
    }
}

impl<'a, T> ToInputValue for &'a T
where
    T: ToInputValue,
{
    fn to_input_value(&self) -> InputValue {
        (**self).to_input_value()
    }
}

impl<T> GraphQLType for Arc<T>
where
    T: GraphQLType + ?Sized,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        T::name(info)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r> {
        T::meta(info, registry)
    }
}

impl<T> GraphQLValue for Arc<T>
where
    T: GraphQLValue + ?Sized,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        (**self).type_name(info)
    }

    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        name: &str,
        selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        (**self).resolve_into_type(info, name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        args: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        (**self).resolve_field(info, field, args, executor)
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        (**self).resolve(info, selection_set, executor)
    }
}

impl<'e, T> GraphQLValueAsync for Arc<T>
where
    T: GraphQLValueAsync + Send + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection]>,
        executor: &'a Executor<Self::Context>,
    ) -> BoxFuture<'a, ExecutionResult> {
        (**self).resolve_async(info, selection_set, executor)
    }
}

impl<T> ToInputValue for Arc<T>
where
    T: ToInputValue,
{
    fn to_input_value(&self) -> InputValue {
        (**self).to_input_value()
    }
}
