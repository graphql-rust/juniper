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

pub trait ScalarToken<S = DefaultScalarValue> {
    fn parse_scalar_token(token: parser::ScalarToken<'_>) -> Result<S, ParseError<'_>>;
}

pub trait InputValue<'input, S: 'input = DefaultScalarValue>: Sized {
    type Error;

    fn try_from_input_value(v: &'input graphql::InputValue<S>) -> Result<Self, Self::Error>;

    fn try_from_implicit_null() -> Result<Self, Self::Error> {
        Self::try_from_input_value(&graphql::InputValue::<S>::Null)
    }
}

pub trait InputValueOwned<S = DefaultScalarValue>: for<'i> InputValue<'i, S> {}

impl<T, S> InputValueOwned<S> for T where T: for<'i> InputValue<'i, S> {}

pub trait InputValueAsRef<S = DefaultScalarValue> {
    type Error;

    fn try_from_input_value(v: &graphql::InputValue<S>) -> Result<&Self, Self::Error>;

    fn try_from_implicit_null() -> Result<&'static Self, Self::Error>
    where S: 'static
    {
        Self::try_from_input_value(&graphql::InputValue::<S>::Null)
    }
}
