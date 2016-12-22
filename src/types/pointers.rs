use ast::{Selection, InputValue, ToInputValue, FromInputValue};
use value::Value;

use schema::meta::MetaType;
use executor::{Executor, Registry, ExecutionResult, IntoFieldResult, FieldResult};
use types::base::{Arguments, GraphQLType};

impl<T, CtxT> GraphQLType for Box<T> where T: GraphQLType<Context=CtxT> {
    type Context = CtxT;

    fn name() -> Option<&'static str> {
        T::name()
    }

    fn meta(registry: &mut Registry) -> MetaType {
        T::meta(registry)
    }

    fn resolve_into_type(&self, name: &str, selection_set: Option<Vec<Selection>>, executor: &Executor<CtxT>) -> ExecutionResult {
        (**self).resolve_into_type(name, selection_set, executor)
    }

    fn resolve_field(&self, field: &str, args: &Arguments, executor: &Executor<CtxT>) -> ExecutionResult
    {
        (**self).resolve_field(field, args, executor)
    }

    fn resolve(&self, selection_set: Option<Vec<Selection>>, executor: &Executor<CtxT>) -> Value {
        (**self).resolve(selection_set, executor)
    }
}

impl<T> FromInputValue for Box<T> where T: FromInputValue {
    fn from(v: &InputValue) -> Option<Box<T>> {
        match <T as FromInputValue>::from(v) {
            Some(v) => Some(Box::new(v)),
            None => None,
        }
    }
}

impl<T> ToInputValue for Box<T> where T: ToInputValue {
    fn to(&self) -> InputValue {
        (**self).to()
    }
}

impl<T> IntoFieldResult<Box<T>> for Box<T> {
    fn into(self) -> FieldResult<Box<T>> {
        Ok(self)
    }
}

impl<'a, T, CtxT> GraphQLType for &'a T where T: GraphQLType<Context=CtxT> {
    type Context = CtxT;

    fn name() -> Option<&'static str> {
        T::name()
    }

    fn meta(registry: &mut Registry) -> MetaType {
        T::meta(registry)
    }

    fn resolve_into_type(&self, name: &str, selection_set: Option<Vec<Selection>>, executor: &Executor<CtxT>) -> ExecutionResult {
        (**self).resolve_into_type(name, selection_set, executor)
    }

    fn resolve_field(&self, field: &str, args: &Arguments, executor: &Executor<CtxT>) -> ExecutionResult
    {
        (**self).resolve_field(field, args, executor)
    }

    fn resolve(&self, selection_set: Option<Vec<Selection>>, executor: &Executor<CtxT>) -> Value {
        (**self).resolve(selection_set, executor)
    }
}

impl<'a, T> ToInputValue for &'a T where T: ToInputValue {
    fn to(&self) -> InputValue {
        (**self).to()
    }
}

impl<'a, T> IntoFieldResult<&'a T> for &'a T {
    fn into(self) -> FieldResult<&'a T> {
        Ok(self)
    }
}
