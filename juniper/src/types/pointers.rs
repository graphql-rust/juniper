use ast::{FromInputValue, InputValue, Selection, ToInputValue};
use value::Value;

use schema::meta::MetaType;
use executor::{ExecutionResult, Executor, Registry};
use types::base::{Arguments, GraphQLType};

impl<T, CtxT> GraphQLType for Box<T>
where
    T: GraphQLType<Context = CtxT>,
{
    type Context = CtxT;

    fn name() -> Option<&'static str> {
        T::name()
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        T::meta(registry)
    }

    fn resolve_into_type(
        &self,
        name: &str,
        selection_set: Option<&[Selection]>,
        executor: &Executor<CtxT>,
    ) -> ExecutionResult {
        (**self).resolve_into_type(name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        field: &str,
        args: &Arguments,
        executor: &Executor<CtxT>,
    ) -> ExecutionResult {
        (**self).resolve_field(field, args, executor)
    }

    fn resolve(&self, selection_set: Option<&[Selection]>, executor: &Executor<CtxT>) -> Value {
        (**self).resolve(selection_set, executor)
    }
}

impl<T> FromInputValue for Box<T>
where
    T: FromInputValue,
{
    fn from(v: &InputValue) -> Option<Box<T>> {
        match <T as FromInputValue>::from(v) {
            Some(v) => Some(Box::new(v)),
            None => None,
        }
    }
}

impl<T> ToInputValue for Box<T>
where
    T: ToInputValue,
{
    fn to(&self) -> InputValue {
        (**self).to()
    }
}

impl<'a, T, CtxT> GraphQLType for &'a T
where
    T: GraphQLType<Context = CtxT>,
{
    type Context = CtxT;

    fn name() -> Option<&'static str> {
        T::name()
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        T::meta(registry)
    }

    fn resolve_into_type(
        &self,
        name: &str,
        selection_set: Option<&[Selection]>,
        executor: &Executor<CtxT>,
    ) -> ExecutionResult {
        (**self).resolve_into_type(name, selection_set, executor)
    }

    fn resolve_field(
        &self,
        field: &str,
        args: &Arguments,
        executor: &Executor<CtxT>,
    ) -> ExecutionResult {
        (**self).resolve_field(field, args, executor)
    }

    fn resolve(&self, selection_set: Option<&[Selection]>, executor: &Executor<CtxT>) -> Value {
        (**self).resolve(selection_set, executor)
    }
}

impl<'a, T> ToInputValue for &'a T
where
    T: ToInputValue,
{
    fn to(&self) -> InputValue {
        (**self).to()
    }
}
