use std::convert::TryFrom;

use crate::{
    graphql,
    meta::MetaType,
    parser::{self, ParseError},
    Arguments, BoxFuture, DefaultScalarValue, ExecutionResult, Executor, Registry, Selection,
};

pub trait Type<Info: ?Sized, S = DefaultScalarValue> {
    fn meta<'r>(registry: &mut Registry<'r, S>, info: &Info) -> MetaType<'r, S>
    where
        S: 'r; // TODO: remove?
}

pub trait TypeName<Info: ?Sized> {
    fn type_name(info: &Info) -> &str;
}

pub trait ConcreteTypeName<Info: ?Sized> {
    fn concrete_type_name<'i>(&self, info: &'i Info) -> &'i str;
}

pub trait Value<Info: ?Sized, Ctx: ?Sized, S = DefaultScalarValue> {
    fn resolve_value(
        &self,
        selection_set: Option<&[Selection<'_, S>]>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>;
}

pub trait ValueAsync<Info: ?Sized, Ctx: ?Sized, S = DefaultScalarValue> {
    fn resolve_value_async<'r>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, S>]>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>>;
}

pub trait ConcreteValue<Info: ?Sized, Ctx: ?Sized, S = DefaultScalarValue> {
    fn resolve_concrete_value(
        &self,
        type_name: &str,
        selection_set: Option<&[Selection<'_, S>]>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>;
}

pub trait ConcreteValueAsync<Info: ?Sized, Ctx: ?Sized, S = DefaultScalarValue> {
    fn resolve_concrete_value_async<'r>(
        &'r self,
        type_name: &str,
        selection_set: Option<&'r [Selection<'_, S>]>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>>;
}

pub trait Field<Info: ?Sized, Ctx: ?Sized, S = DefaultScalarValue> {
    fn resolve_field(
        &self,
        field_name: &str,
        arguments: &Arguments<S>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>;
}

pub trait FieldAsync<Info: ?Sized, Ctx: ?Sized, S = DefaultScalarValue> {
    fn resolve_field_async<'r>(
        &'r self,
        field_name: &'r str,
        arguments: &'r Arguments<S>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>>;
}

pub trait InputValue<'inp, S: 'inp>: TryFrom<&'inp graphql::InputValue<S>> {
    fn try_from_implicit_null() -> Result<Self, Self::Error> {
        Self::try_from(&graphql::InputValue::<S>::Null)
    }
}

pub trait InputValueOwned<S>: for<'inp> InputValue<'inp, S> {}

impl<T, S> InputValueOwned<S> for T where T: for<'inp> InputValue<'inp, S> {}

pub trait ValidateInputValue<S>: Sized {
    fn validate_input_value<'inp>(
        v: &'inp graphql::InputValue<S>,
    ) -> Result<(), crate::FieldError<S>>
    where
        Self: TryFrom<&'inp graphql::InputValue<S>>,
        <Self as TryFrom<&'inp graphql::InputValue<S>>>::Error: crate::IntoFieldError<S>;
}

impl<T, S> ValidateInputValue<S> for T {
    fn validate_input_value<'inp>(
        v: &'inp graphql::InputValue<S>,
    ) -> Result<(), crate::FieldError<S>>
    where
        Self: TryFrom<&'inp graphql::InputValue<S>>,
        <Self as TryFrom<&'inp graphql::InputValue<S>>>::Error: crate::IntoFieldError<S>,
    {
        Self::try_from(v)
            .map(drop)
            .map_err(crate::IntoFieldError::<S>::into_field_error)
    }
}

pub trait ScalarToken<S = DefaultScalarValue> {
    fn parse_scalar_token(token: parser::ScalarToken<'_>) -> Result<S, ParseError<'_>>;
}
