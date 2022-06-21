//! GraphQL implementation for [slice].
//!
//! [slice]: prim@slice

use std::{rc::Rc, sync::Arc};

use crate::{
    behavior,
    executor::{ExecutionResult, Executor, Registry},
    graphql, reflect, resolve,
    schema::meta::MetaType,
    BoxFuture, Selection,
};

use super::{iter, vec::TryFromInputValueError};

impl<T, TI, SV, BH> resolve::Type<TI, SV, BH> for [T]
where
    T: resolve::Type<TI, SV, BH>,
    TI: ?Sized,
    BH: ?Sized,
{
    fn meta<'r, 'ti: 'r>(registry: &mut Registry<'r, SV>, type_info: &'ti TI) -> MetaType<'r, SV>
    where
        SV: 'r,
    {
        registry.wrap_list::<behavior::Coerce<T, BH>, _>(type_info, None)
    }
}

impl<T, TI, CX, SV, BH> resolve::Value<TI, CX, SV, BH> for [T]
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
        iter::resolve_list(self.iter(), selection_set, type_info, executor)
    }
}

impl<T, TI, CX, SV, BH> resolve::ValueAsync<TI, CX, SV, BH> for [T]
where
    T: resolve::ValueAsync<TI, CX, SV, BH> + Sync,
    TI: Sync + ?Sized,
    CX: Sync + ?Sized,
    SV: Send + Sync,
    BH: ?Sized + 'static, // TODO: Lift `'static` bound if possible.
{
    fn resolve_value_async<'r>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, SV>]>,
        type_info: &'r TI,
        executor: &'r Executor<CX, SV>,
    ) -> BoxFuture<'r, ExecutionResult<SV>> {
        Box::pin(iter::resolve_list_async(
            self.iter(),
            selection_set,
            type_info,
            executor,
        ))
    }
}

impl<T, SV, BH> resolve::ToInputValue<SV, BH> for [T]
where
    T: resolve::ToInputValue<SV, BH>,
    BH: ?Sized,
{
    fn to_input_value(&self) -> graphql::InputValue<SV> {
        graphql::InputValue::list(self.iter().map(T::to_input_value))
    }
}

impl<'i, T, SV, BH> resolve::InputValueAs<'i, Box<Self>, SV, BH> for [T]
where
    Vec<T>: resolve::InputValue<'i, SV, BH>,
    SV: 'i,
    BH: ?Sized,
{
    type Error = <Vec<T> as resolve::InputValue<'i, SV, BH>>::Error;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Box<Self>, Self::Error> {
        <Vec<T> as resolve::InputValue<'i, SV, BH>>::try_from_input_value(v)
            .map(Vec::into_boxed_slice)
    }
}

impl<'i, T, SV, BH> resolve::InputValueAs<'i, Rc<Self>, SV, BH> for [T]
where
    T: resolve::InputValue<'i, SV, BH>,
    SV: 'i,
    BH: ?Sized,
{
    type Error = TryFromInputValueError<T::Error>;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Rc<Self>, Self::Error> {
        // We don't want to reuse `Vec<T>` implementation in the same way we do
        // for `Box<[T]>`, because `impl From<Vec<T>> for Rc<[T]>` reallocates.
        match v {
            graphql::InputValue::List(l) => l
                .iter()
                .map(|i| T::try_from_input_value(&i.item).map_err(TryFromInputValueError::Item))
                .collect(),
            // See "Input Coercion" on List types:
            // https://spec.graphql.org/October2021#sec-Combining-List-and-Non-Null
            graphql::InputValue::Null => Err(TryFromInputValueError::IsNull),
            // TODO: Use `.into_iter()` after upgrade to 2021 Rust edition.
            other => T::try_from_input_value(other)
                .map(|e| std::iter::once(e).collect())
                .map_err(TryFromInputValueError::Item),
        }
    }
}

impl<'i, T, SV, BH> resolve::InputValueAs<'i, Arc<Self>, SV, BH> for [T]
where
    T: resolve::InputValue<'i, SV, BH>,
    SV: 'i,
    BH: ?Sized,
{
    type Error = TryFromInputValueError<T::Error>;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Arc<Self>, Self::Error> {
        // We don't want to reuse `Vec<T>` implementation in the same way we do
        // for `Box<[T]>`, because `impl From<Vec<T>> for Arc<[T]>` reallocates.
        match v {
            graphql::InputValue::List(l) => l
                .iter()
                .map(|i| T::try_from_input_value(&i.item).map_err(TryFromInputValueError::Item))
                .collect(),
            // See "Input Coercion" on List types:
            // https://spec.graphql.org/October2021#sec-Combining-List-and-Non-Null
            graphql::InputValue::Null => Err(TryFromInputValueError::IsNull),
            // TODO: Use `.into_iter()` after upgrade to 2021 Rust edition.
            other => T::try_from_input_value(other)
                .map(|e| std::iter::once(e).collect())
                .map_err(TryFromInputValueError::Item),
        }
    }
}

impl<'i, T, TI, SV, BH> graphql::InputTypeAs<'i, Box<Self>, TI, SV, BH> for [T]
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

impl<'i, T, TI, SV, BH> graphql::InputTypeAs<'i, Rc<Self>, TI, SV, BH> for [T]
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

impl<'i, T, TI, SV, BH> graphql::InputTypeAs<'i, Arc<Self>, TI, SV, BH> for [T]
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

impl<T, TI, CX, SV, BH> graphql::OutputType<TI, CX, SV, BH> for [T]
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

impl<T, BH> reflect::BaseType<BH> for [T]
where
    T: reflect::BaseType<BH>,
    BH: ?Sized,
{
    const NAME: reflect::Type = T::NAME;
}

impl<T, BH> reflect::BaseSubTypes<BH> for [T]
where
    T: reflect::BaseSubTypes<BH>,
    BH: ?Sized,
{
    const NAMES: reflect::Types = T::NAMES;
}

impl<T, BH> reflect::WrappedType<BH> for [T]
where
    T: reflect::WrappedType<BH>,
    BH: ?Sized,
{
    const VALUE: reflect::WrappedValue = reflect::wrap::list(T::VALUE);
}
