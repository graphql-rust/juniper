//! GraphQL implementation for mutable [reference].
//!
//! [reference]: primitive@std::reference

use crate::{
    graphql,
    meta::MetaType,
    parser::{ParseError, ScalarToken},
    reflect, resolve, Arguments, BoxFuture, ExecutionResult, Executor, FieldResult, Registry,
    Selection,
};

impl<'me, T, TI, SV, BH> resolve::Type<TI, SV, BH> for &'me mut T
where
    T: resolve::Type<TI, SV, BH> + ?Sized,
    TI: ?Sized,
    BH: ?Sized,
{
    fn meta<'r, 'ti: 'r>(registry: &mut Registry<'r, SV>, type_info: &'ti TI) -> MetaType<'r, SV>
    where
        SV: 'r,
    {
        T::meta(registry, type_info)
    }
}

impl<'me, T, TI, BH> resolve::TypeName<TI, BH> for &'me mut T
where
    T: resolve::TypeName<TI, BH> + ?Sized,
    TI: ?Sized,
    BH: ?Sized,
{
    fn type_name(type_info: &TI) -> &str {
        T::type_name(type_info)
    }
}

impl<'me, T, TI, BH> resolve::ConcreteTypeName<TI, BH> for &'me mut T
where
    T: resolve::ConcreteTypeName<TI, BH> + ?Sized,
    TI: ?Sized,
    BH: ?Sized,
{
    fn concrete_type_name<'i>(&self, type_info: &'i TI) -> &'i str {
        (**self).concrete_type_name(type_info)
    }
}

impl<'me, T, TI, CX, SV, BH> resolve::Value<TI, CX, SV, BH> for &'me mut T
where
    T: resolve::Value<TI, CX, SV, BH> + ?Sized,
    TI: ?Sized,
    CX: ?Sized,
    BH: ?Sized,
{
    fn resolve_value(
        &self,
        selection_set: Option<&[Selection<'_, SV>]>,
        type_info: &TI,
        executor: &Executor<CX, SV>,
    ) -> ExecutionResult<SV> {
        (**self).resolve_value(selection_set, type_info, executor)
    }
}

impl<'me, T, TI, CX, SV, BH> resolve::ValueAsync<TI, CX, SV, BH> for &'me mut T
where
    T: resolve::ValueAsync<TI, CX, SV, BH> + ?Sized,
    TI: ?Sized,
    CX: ?Sized,
    BH: ?Sized,
{
    fn resolve_value_async<'r>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, SV>]>,
        type_info: &'r TI,
        executor: &'r Executor<CX, SV>,
    ) -> BoxFuture<'r, ExecutionResult<SV>> {
        (**self).resolve_value_async(selection_set, type_info, executor)
    }
}

impl<'me, T, SV, BH> resolve::Resolvable<SV, BH> for &'me mut T
where
    T: ?Sized,
    BH: ?Sized,
{
    type Value = Self;

    fn into_value(self) -> FieldResult<Self, SV> {
        Ok(self)
    }
}

impl<'me, T, TI, CX, SV, BH> resolve::ConcreteValue<TI, CX, SV, BH> for &'me mut T
where
    T: resolve::ConcreteValue<TI, CX, SV, BH> + ?Sized,
    TI: ?Sized,
    CX: ?Sized,
    BH: ?Sized,
{
    fn resolve_concrete_value(
        &self,
        type_name: &str,
        selection_set: Option<&[Selection<'_, SV>]>,
        type_info: &TI,
        executor: &Executor<CX, SV>,
    ) -> ExecutionResult<SV> {
        (**self).resolve_concrete_value(type_name, selection_set, type_info, executor)
    }
}

impl<'me, T, TI, CX, SV, BH> resolve::ConcreteValueAsync<TI, CX, SV, BH> for &'me mut T
where
    T: resolve::ConcreteValueAsync<TI, CX, SV, BH> + ?Sized,
    TI: ?Sized,
    CX: ?Sized,
    BH: ?Sized,
{
    fn resolve_concrete_value_async<'r>(
        &'r self,
        type_name: &str,
        selection_set: Option<&'r [Selection<'_, SV>]>,
        type_info: &'r TI,
        executor: &'r Executor<CX, SV>,
    ) -> BoxFuture<'r, ExecutionResult<SV>> {
        (**self).resolve_concrete_value_async(type_name, selection_set, type_info, executor)
    }
}

impl<'me, T, TI, CX, SV, BH> resolve::Field<TI, CX, SV, BH> for &'me mut T
where
    T: resolve::Field<TI, CX, SV, BH> + ?Sized,
    TI: ?Sized,
    CX: ?Sized,
    BH: ?Sized,
{
    fn resolve_field(
        &self,
        field_name: &str,
        arguments: &Arguments<SV>,
        type_info: &TI,
        executor: &Executor<CX, SV>,
    ) -> ExecutionResult<SV> {
        (**self).resolve_field(field_name, arguments, type_info, executor)
    }
}

impl<'me, T, TI, CX, SV, BH> resolve::FieldAsync<TI, CX, SV, BH> for &'me mut T
where
    T: resolve::FieldAsync<TI, CX, SV, BH> + ?Sized,
    TI: ?Sized,
    CX: ?Sized,
    BH: ?Sized,
{
    fn resolve_field_async<'r>(
        &'r self,
        field_name: &'r str,
        arguments: &'r Arguments<SV>,
        type_info: &'r TI,
        executor: &'r Executor<CX, SV>,
    ) -> BoxFuture<'r, ExecutionResult<SV>> {
        (**self).resolve_field_async(field_name, arguments, type_info, executor)
    }
}

impl<'me, T, SV, BH> resolve::ToInputValue<SV, BH> for &'me mut T
where
    T: resolve::ToInputValue<SV, BH> + ?Sized,
    BH: ?Sized,
{
    fn to_input_value(&self) -> graphql::InputValue<SV> {
        (**self).to_input_value()
    }
}

impl<'me, T, SV, BH> resolve::ScalarToken<SV, BH> for &'me mut T
where
    T: resolve::ScalarToken<SV, BH> + ?Sized,
    BH: ?Sized,
{
    fn parse_scalar_token(token: ScalarToken<'_>) -> Result<SV, ParseError> {
        T::parse_scalar_token(token)
    }
}

impl<'me, T, TI, CX, SV, BH> graphql::OutputType<TI, CX, SV, BH> for &'me mut T
where
    T: graphql::OutputType<TI, CX, SV, BH> + ?Sized,
    TI: ?Sized,
    CX: ?Sized,
    BH: ?Sized,
{
    fn assert_output_type() {
        T::assert_output_type()
    }
}

impl<'me, T, BH> reflect::BaseType<BH> for &'me mut T
where
    T: reflect::BaseType<BH> + ?Sized,
    BH: ?Sized,
{
    const NAME: reflect::Type = T::NAME;
}

impl<'me, T, BH> reflect::BaseSubTypes<BH> for &'me mut T
where
    T: reflect::BaseSubTypes<BH> + ?Sized,
    BH: ?Sized,
{
    const NAMES: reflect::Types = T::NAMES;
}

impl<'me, T, BH> reflect::WrappedType<BH> for &'me mut T
where
    T: reflect::WrappedType<BH> + ?Sized,
    BH: ?Sized,
{
    const VALUE: reflect::WrappedValue = T::VALUE;
}
