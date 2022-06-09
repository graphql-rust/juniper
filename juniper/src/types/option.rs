//! GraphQL implementation for [`Option`].

use futures::future;

use crate::{
    executor::{ExecutionResult, Executor, Registry},
    graphql, reflect, resolve,
    schema::meta::MetaType,
    BoxFuture, Selection,
};

impl<T, TI, SV, BH> resolve::Type<TI, SV, BH> for Option<T>
where
    T: resolve::Type<TI, SV, BH>,
    TI: ?Sized,
    BH: ?Sized,
{
    fn meta<'r>(registry: &mut Registry<'r, SV>, type_info: &TI) -> MetaType<'r, SV>
    where
        SV: 'r,
    {
        registry
            .build_nullable_type_reworked::<T, BH, _>(type_info)
            .into_meta()
    }
}

impl<T, TI, CX, SV, BH> resolve::Value<TI, CX, SV, BH> for Option<T>
where
    T: resolve::Value<TI, CX, SV, BH>,
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
        match self {
            Some(v) => v.resolve_value(selection_set, type_info, executor),
            None => Ok(graphql::Value::Null),
        }
    }
}

impl<T, TI, CX, SV, BH> resolve::ValueAsync<TI, CX, SV, BH> for Option<T>
where
    T: resolve::ValueAsync<TI, CX, SV, BH>,
    TI: ?Sized,
    CX: ?Sized,
    SV: Send,
    BH: ?Sized,
{
    fn resolve_value_async<'r>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, SV>]>,
        type_info: &'r TI,
        executor: &'r Executor<CX, SV>,
    ) -> BoxFuture<'r, ExecutionResult<SV>> {
        match self {
            Some(v) => v.resolve_value_async(selection_set, type_info, executor),
            None => Box::pin(future::ok(graphql::Value::Null)),
        }
    }
}

impl<T, SV, BH> resolve::ToInputValue<SV, BH> for Option<T>
where
    T: resolve::ToInputValue<SV, BH>,
    BH: ?Sized,
{
    fn to_input_value(&self) -> graphql::InputValue<SV> {
        match self {
            Some(v) => v.to_input_value(),
            None => graphql::InputValue::Null,
        }
    }
}

impl<'i, T, SV, BH> resolve::InputValue<'i, SV, BH> for Option<T>
where
    T: resolve::InputValue<'i, SV, BH>,
    SV: 'i,
    BH: ?Sized,
{
    type Error = T::Error;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Self, Self::Error> {
        if v.is_null() {
            Ok(None)
        } else {
            T::try_from_input_value(v).map(Some)
        }
    }
}

impl<'i, T, TI, SV, BH> graphql::InputType<'i, TI, SV, BH> for Option<T>
where
    T: graphql::InputType<'i, TI, SV, BH>,
    TI: ?Sized,
    SV: 'i,
    BH: ?Sized,
{
    fn assert_input_type() {
        T::assert_input_type()
    }
}

impl<T, TI, CX, SV, BH> graphql::OutputType<TI, CX, SV, BH> for Option<T>
where
    T: graphql::OutputType<TI, CX, SV, BH>,
    TI: ?Sized,
    CX: ?Sized,
    BH: ?Sized,
    Self: resolve::ValueAsync<TI, CX, SV, BH>,
{
    fn assert_output_type() {
        T::assert_output_type()
    }
}

impl<T, BH> reflect::BaseType<BH> for Option<T>
where
    T: reflect::BaseType<BH>,
    BH: ?Sized,
{
    const NAME: reflect::Type = T::NAME;
}

impl<T, BH> reflect::BaseSubTypes<BH> for Option<T>
where
    T: reflect::BaseSubTypes<BH>,
    BH: ?Sized,
{
    const NAMES: reflect::Types = T::NAMES;
}

impl<T, BH> reflect::WrappedType<BH> for Option<T>
where
    T: reflect::WrappedType<BH>,
    BH: ?Sized,
{
    const VALUE: reflect::WrappedValue = reflect::wrap::nullable(T::VALUE);
}
