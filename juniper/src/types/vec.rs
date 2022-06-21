//! GraphQL implementation for [`Vec`].

use crate::{
    behavior,
    executor::{ExecutionResult, Executor, Registry},
    graphql, reflect, resolve,
    schema::meta::MetaType,
    BoxFuture, FieldError, IntoFieldError, Selection,
};

use super::iter;

impl<T, TI, SV, BH> resolve::Type<TI, SV, BH> for Vec<T>
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

impl<T, TI, CX, SV, BH> resolve::Value<TI, CX, SV, BH> for Vec<T>
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

impl<T, TI, CX, SV, BH> resolve::ValueAsync<TI, CX, SV, BH> for Vec<T>
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

impl<T, SV, BH> resolve::ToInputValue<SV, BH> for Vec<T>
where
    T: resolve::ToInputValue<SV, BH>,
    BH: ?Sized,
{
    fn to_input_value(&self) -> graphql::InputValue<SV> {
        graphql::InputValue::list(self.iter().map(T::to_input_value))
    }
}

impl<'i, T, SV, BH> resolve::InputValue<'i, SV, BH> for Vec<T>
where
    T: resolve::InputValue<'i, SV, BH>,
    SV: 'i,
    BH: ?Sized,
{
    type Error = TryFromInputValueError<T::Error>;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Self, Self::Error> {
        match v {
            graphql::InputValue::List(l) => l
                .iter()
                .map(|i| T::try_from_input_value(&i.item).map_err(TryFromInputValueError::Item))
                .collect(),
            // See "Input Coercion" on List types:
            // https://spec.graphql.org/October2021#sec-Combining-List-and-Non-Null
            graphql::InputValue::Null => Err(TryFromInputValueError::IsNull),
            other => T::try_from_input_value(other)
                .map(|e| vec![e])
                .map_err(TryFromInputValueError::Item),
        }
    }
}

impl<'i, T, TI, SV, BH> graphql::InputType<'i, TI, SV, BH> for Vec<T>
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

impl<T, TI, CX, SV, BH> graphql::OutputType<TI, CX, SV, BH> for Vec<T>
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

impl<T, BH> reflect::BaseType<BH> for Vec<T>
where
    T: reflect::BaseType<BH>,
    BH: ?Sized,
{
    const NAME: reflect::Type = T::NAME;
}

impl<T, BH> reflect::BaseSubTypes<BH> for Vec<T>
where
    T: reflect::BaseSubTypes<BH>,
    BH: ?Sized,
{
    const NAMES: reflect::Types = T::NAMES;
}

impl<T, BH> reflect::WrappedType<BH> for Vec<T>
where
    T: reflect::WrappedType<BH>,
    BH: ?Sized,
{
    const VALUE: reflect::WrappedValue = reflect::wrap::list(T::VALUE);
}

/// Possible errors of converting a [`graphql::InputValue`] into a [`Vec`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TryFromInputValueError<E> {
    /// [`graphql::InputValue`] cannot be [`Null`].
    ///
    /// See ["Combining List and Non-Null" section of spec][0].
    ///
    /// [`Null`]: [`InputValue::Null`]
    /// [0]: https://spec.graphql.org/October2021#sec-Combining-List-and-Non-Null
    IsNull,

    /// Error of converting a [`graphql::InputValue::List`]'s item.
    Item(E),
}

impl<E, SV> IntoFieldError<SV> for TryFromInputValueError<E>
where
    E: IntoFieldError<SV>,
{
    fn into_field_error(self) -> FieldError<SV> {
        match self {
            Self::IsNull => "Failed to convert into `Vec`: Value cannot be `null`".into(),
            Self::Item(e) => e.into_field_error(),
        }
    }
}

// See "Input Coercion" examples on List types:
// https://spec.graphql.org/October2021#sec-List.Input-Coercion
#[cfg(test)]
mod coercion {
    use crate::{graphql, resolve::InputValue as _, IntoFieldError as _};

    use super::TryFromInputValueError;

    type V = graphql::InputValue;

    #[test]
    fn from_null() {
        let v: V = graphql::input_value!(null);
        assert_eq!(
            <Vec<i32>>::try_from_input_value(&v),
            Err(TryFromInputValueError::IsNull),
        );
        assert_eq!(
            <Vec<Option<i32>>>::try_from_input_value(&v),
            Err(TryFromInputValueError::IsNull),
        );
        assert_eq!(<Option<Vec<i32>>>::try_from_input_value(&v), Ok(None));
        assert_eq!(
            <Option<Vec<Option<i32>>>>::try_from_input_value(&v),
            Ok(None),
        );
        assert_eq!(
            <Vec<Vec<i32>>>::try_from_input_value(&v),
            Err(TryFromInputValueError::IsNull),
        );
        assert_eq!(
            <Option<Vec<Option<Vec<Option<i32>>>>>>::try_from_input_value(&v),
            Ok(None),
        );
    }

    #[test]
    fn from_value() {
        let v: V = graphql::input_value!(1);
        assert_eq!(<Vec<i32>>::try_from_input_value(&v), Ok(vec![1]));
        assert_eq!(
            <Vec<Option<i32>>>::try_from_input_value(&v),
            Ok(vec![Some(1)]),
        );
        assert_eq!(
            <Option<Vec<i32>>>::try_from_input_value(&v),
            Ok(Some(vec![1])),
        );
        assert_eq!(
            <Option<Vec<Option<i32>>>>::try_from_input_value(&v),
            Ok(Some(vec![Some(1)])),
        );
        assert_eq!(<Vec<Vec<i32>>>::try_from_input_value(&v), Ok(vec![vec![1]]));
        assert_eq!(
            <Option<Vec<Option<Vec<Option<i32>>>>>>::try_from_input_value(&v),
            Ok(Some(vec![Some(vec![Some(1)])])),
        );
    }

    #[test]
    fn from_list() {
        let v: V = graphql::input_value!([1, 2, 3]);
        assert_eq!(<Vec<i32>>::try_from_input_value(&v), Ok(vec![1, 2, 3]));
        assert_eq!(
            <Option<Vec<i32>>>::try_from_input_value(&v),
            Ok(Some(vec![1, 2, 3])),
        );
        assert_eq!(
            <Vec<Option<i32>>>::try_from_input_value(&v),
            Ok(vec![Some(1), Some(2), Some(3)]),
        );
        assert_eq!(
            <Option<Vec<Option<i32>>>>::try_from_input_value(&v),
            Ok(Some(vec![Some(1), Some(2), Some(3)])),
        );
        assert_eq!(
            <Vec<Vec<i32>>>::try_from_input_value(&v),
            Ok(vec![vec![1], vec![2], vec![3]]),
        );
        // Looks like the spec ambiguity.
        // See: https://github.com/graphql/graphql-spec/pull/515
        assert_eq!(
            <Option<Vec<Option<Vec<Option<i32>>>>>>::try_from_input_value(&v),
            Ok(Some(vec![
                Some(vec![Some(1)]),
                Some(vec![Some(2)]),
                Some(vec![Some(3)]),
            ])),
        );
    }

    #[test]
    fn from_list_with_null() {
        let v: V = graphql::input_value!([1, 2, null]);
        assert_eq!(
            <Vec<i32>>::try_from_input_value(&v),
            Err(TryFromInputValueError::Item(
                "Expected `Int`, found: null".into_field_error(),
            )),
        );
        assert_eq!(
            <Option<Vec<i32>>>::try_from_input_value(&v),
            Err(TryFromInputValueError::Item(
                "Expected `Int`, found: null".into_field_error(),
            )),
        );
        assert_eq!(
            <Vec<Option<i32>>>::try_from_input_value(&v),
            Ok(vec![Some(1), Some(2), None]),
        );
        assert_eq!(
            <Option<Vec<Option<i32>>>>::try_from_input_value(&v),
            Ok(Some(vec![Some(1), Some(2), None])),
        );
        assert_eq!(
            <Vec<Vec<i32>>>::try_from_input_value(&v),
            Err(TryFromInputValueError::Item(TryFromInputValueError::IsNull)),
        );
        // Looks like the spec ambiguity.
        // See: https://github.com/graphql/graphql-spec/pull/515
        assert_eq!(
            <Option<Vec<Option<Vec<Option<i32>>>>>>::try_from_input_value(&v),
            Ok(Some(vec![Some(vec![Some(1)]), Some(vec![Some(2)]), None])),
        );
    }
}
