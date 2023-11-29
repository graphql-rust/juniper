use std::{
    mem::{self, MaybeUninit},
    ptr,
};

use crate::{
    ast::{FromInputValue, InputValue, Selection, ToInputValue},
    executor::{ExecutionResult, Executor, FieldError, IntoFieldError, Registry},
    schema::meta::MetaType,
    types::{
        async_await::GraphQLValueAsync,
        base::{GraphQLType, GraphQLValue},
    },
    value::{ScalarValue, Value},
};

impl<S, T> GraphQLType<S> for Option<T>
where
    T: GraphQLType<S>,
    S: ScalarValue,
{
    fn name(_: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_nullable_type::<T>(info).into_meta()
    }
}

impl<S, T> GraphQLValue<S> for Option<T>
where
    S: ScalarValue,
    T: GraphQLValue<S>,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, _: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        match *self {
            Some(ref obj) => executor.resolve(info, obj),
            None => Ok(Value::null()),
        }
    }
}

impl<S, T> GraphQLValueAsync<S> for Option<T>
where
    T: GraphQLValueAsync<S>,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> crate::BoxFuture<'a, ExecutionResult<S>> {
        let f = async move {
            let value = match self {
                Some(obj) => executor.resolve_into_value_async(info, obj).await,
                None => Value::null(),
            };
            Ok(value)
        };
        Box::pin(f)
    }
}

impl<S, T: FromInputValue<S>> FromInputValue<S> for Option<T> {
    type Error = T::Error;

    fn from_input_value(v: &InputValue<S>) -> Result<Self, Self::Error> {
        match v {
            InputValue::Null => Ok(None),
            v => v.convert().map(Some),
        }
    }
}

impl<S, T: ToInputValue<S>> ToInputValue<S> for Option<T> {
    fn to_input_value(&self) -> InputValue<S> {
        match self {
            Some(v) => v.to_input_value(),
            None => InputValue::Null,
        }
    }
}

impl<S, T> GraphQLType<S> for Vec<T>
where
    T: GraphQLType<S>,
    S: ScalarValue,
{
    fn name(_: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_list_type::<T>(info, None).into_meta()
    }
}

impl<S, T> GraphQLValue<S> for Vec<T>
where
    T: GraphQLValue<S>,
    S: ScalarValue,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, _: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        resolve_into_list(executor, info, self.iter())
    }
}

impl<S, T> GraphQLValueAsync<S> for Vec<T>
where
    T: GraphQLValueAsync<S>,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> crate::BoxFuture<'a, ExecutionResult<S>> {
        let f = resolve_into_list_async(executor, info, self.iter());
        Box::pin(f)
    }
}

impl<S: ScalarValue, T: FromInputValue<S>> FromInputValue<S> for Vec<T> {
    type Error = FromInputValueVecError<T, S>;

    fn from_input_value(v: &InputValue<S>) -> Result<Self, Self::Error> {
        match v {
            InputValue::List(l) => l
                .iter()
                .map(|i| i.item.convert().map_err(FromInputValueVecError::Item))
                .collect(),
            // See "Input Coercion" on List types:
            // https://spec.graphql.org/October2021#sec-Combining-List-and-Non-Null
            InputValue::Null => Err(FromInputValueVecError::Null),
            other => other
                .convert()
                .map(|e| vec![e])
                .map_err(FromInputValueVecError::Item),
        }
    }
}

impl<T, S> ToInputValue<S> for Vec<T>
where
    T: ToInputValue<S>,
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        InputValue::list(self.iter().map(T::to_input_value).collect())
    }
}

/// Possible errors of converting [`InputValue`] into [`Vec`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FromInputValueVecError<T, S>
where
    T: FromInputValue<S>,
    S: ScalarValue,
{
    /// [`InputValue`] cannot be [`Null`].
    ///
    /// See ["Combining List and Non-Null" section of spec][1].
    ///
    /// [`Null`]: [`InputValue::Null`]
    /// [1]: https://spec.graphql.org/October2021#sec-Combining-List-and-Non-Null
    Null,

    /// Error of converting [`InputValue::List`]'s item.
    Item(T::Error),
}

impl<T, S> IntoFieldError<S> for FromInputValueVecError<T, S>
where
    T: FromInputValue<S>,
    T::Error: IntoFieldError<S>,
    S: ScalarValue,
{
    fn into_field_error(self) -> FieldError<S> {
        match self {
            Self::Null => "Failed to convert into `Vec`: Value cannot be `null`".into(),
            Self::Item(s) => s.into_field_error(),
        }
    }
}

impl<S, T> GraphQLType<S> for [T]
where
    S: ScalarValue,
    T: GraphQLType<S>,
{
    fn name(_: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_list_type::<T>(info, None).into_meta()
    }
}

impl<S, T> GraphQLValue<S> for [T]
where
    S: ScalarValue,
    T: GraphQLValue<S>,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, _: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        resolve_into_list(executor, info, self.iter())
    }
}

impl<S, T> GraphQLValueAsync<S> for [T]
where
    T: GraphQLValueAsync<S>,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> crate::BoxFuture<'a, ExecutionResult<S>> {
        let f = resolve_into_list_async(executor, info, self.iter());
        Box::pin(f)
    }
}

impl<'a, T, S> ToInputValue<S> for &'a [T]
where
    T: ToInputValue<S>,
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        InputValue::list(self.iter().map(T::to_input_value).collect())
    }
}

impl<S, T, const N: usize> GraphQLType<S> for [T; N]
where
    S: ScalarValue,
    T: GraphQLType<S>,
{
    fn name(_: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_list_type::<T>(info, Some(N)).into_meta()
    }
}

impl<S, T, const N: usize> GraphQLValue<S> for [T; N]
where
    S: ScalarValue,
    T: GraphQLValue<S>,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, _: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        resolve_into_list(executor, info, self.iter())
    }
}

impl<S, T, const N: usize> GraphQLValueAsync<S> for [T; N]
where
    T: GraphQLValueAsync<S>,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> crate::BoxFuture<'a, ExecutionResult<S>> {
        let f = resolve_into_list_async(executor, info, self.iter());
        Box::pin(f)
    }
}

impl<T, S, const N: usize> FromInputValue<S> for [T; N]
where
    T: FromInputValue<S>,
    S: ScalarValue,
{
    type Error = FromInputValueArrayError<T, S>;

    fn from_input_value(v: &InputValue<S>) -> Result<Self, Self::Error> {
        struct PartiallyInitializedArray<T, const N: usize> {
            arr: [MaybeUninit<T>; N],
            init_len: usize,
            no_drop: bool,
        }

        impl<T, const N: usize> Drop for PartiallyInitializedArray<T, N> {
            fn drop(&mut self) {
                if self.no_drop {
                    return;
                }
                // Dropping a `MaybeUninit` does nothing, thus we need to drop
                // the initialized elements manually, otherwise we may introduce
                // a memory/resource leak if `T: Drop`.
                for elem in &mut self.arr[0..self.init_len] {
                    // SAFETY: This is safe, because `self.init_len` represents
                    //         exactly the number of initialized elements.
                    unsafe {
                        ptr::drop_in_place(elem.as_mut_ptr());
                    }
                }
            }
        }

        match *v {
            InputValue::List(ref ls) => {
                if ls.len() != N {
                    return Err(FromInputValueArrayError::WrongCount {
                        actual: ls.len(),
                        expected: N,
                    });
                }
                if N == 0 {
                    // TODO: Use `mem::transmute` instead of
                    //       `mem::transmute_copy` below, once it's allowed
                    //       for const generics:
                    //       https://github.com/rust-lang/rust/issues/61956
                    // SAFETY: `mem::transmute_copy` is safe here, because we
                    //         check `N` to be `0`. It's no-op, actually.
                    return Ok(unsafe { mem::transmute_copy::<[T; 0], Self>(&[]) });
                }

                // SAFETY: The reason we're using a wrapper struct implementing
                //         `Drop` here is to be panic safe:
                //         `T: FromInputValue<S>` implementation is not
                //         controlled by us, so calling `i.item.convert()` below
                //         may cause a panic when our array is initialized only
                //         partially. In such situation we need to drop already
                //         initialized values to avoid possible memory/resource
                //         leaks if `T: Drop`.
                let mut out = PartiallyInitializedArray::<T, N> {
                    // SAFETY: The `.assume_init()` here is safe, because the
                    //         type we are claiming to have initialized here is
                    //         a bunch of `MaybeUninit`s, which do not require
                    //         any initialization.
                    arr: unsafe { MaybeUninit::uninit().assume_init() },
                    init_len: 0,
                    no_drop: false,
                };

                let mut items = ls.iter().map(|i| i.item.convert());
                for elem in &mut out.arr[..] {
                    if let Some(i) = items
                        .next()
                        .transpose()
                        .map_err(FromInputValueArrayError::Item)?
                    {
                        *elem = MaybeUninit::new(i);
                        out.init_len += 1;
                    }
                }

                // Do not drop collected `items`, because we're going to return
                // them.
                out.no_drop = true;

                // TODO: Use `mem::transmute` instead of `mem::transmute_copy`
                //       below, once it's allowed for const generics:
                //       https://github.com/rust-lang/rust/issues/61956
                // SAFETY: `mem::transmute_copy` is safe here, because we have
                //         exactly `N` initialized `items`.
                //         Also, despite `mem::transmute_copy` copies the value,
                //         we won't have a double-free when `T: Drop` here,
                //         because original array elements are `MaybeUninit`, so
                //         do nothing on `Drop`.
                Ok(unsafe { mem::transmute_copy::<_, Self>(&out.arr) })
            }
            // See "Input Coercion" on List types:
            // https://spec.graphql.org/October2021#sec-Combining-List-and-Non-Null
            InputValue::Null => Err(FromInputValueArrayError::Null),
            ref other => {
                other
                    .convert()
                    .map_err(FromInputValueArrayError::Item)
                    .and_then(|e: T| {
                        // TODO: Use `mem::transmute` instead of
                        //       `mem::transmute_copy` below, once it's allowed
                        //       for const generics:
                        //       https://github.com/rust-lang/rust/issues/61956
                        if N == 1 {
                            // SAFETY: `mem::transmute_copy` is safe here,
                            //         because we check `N` to be `1`.
                            //         Also, despite `mem::transmute_copy`
                            //         copies the value, we won't have a
                            //         double-free when `T: Drop` here, because
                            //         original `e: T` value is wrapped into
                            //         `mem::ManuallyDrop`, so does nothing on
                            //         `Drop`.
                            Ok(unsafe {
                                mem::transmute_copy::<_, Self>(&[mem::ManuallyDrop::new(e)])
                            })
                        } else {
                            Err(FromInputValueArrayError::WrongCount {
                                actual: 1,
                                expected: N,
                            })
                        }
                    })
            }
        }
    }
}

impl<T, S, const N: usize> ToInputValue<S> for [T; N]
where
    T: ToInputValue<S>,
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        InputValue::list(self.iter().map(T::to_input_value).collect())
    }
}

/// Error converting [`InputValue`] into exact-size [`array`](prim@array).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FromInputValueArrayError<T, S>
where
    T: FromInputValue<S>,
    S: ScalarValue,
{
    /// [`InputValue`] cannot be [`Null`].
    ///
    /// See ["Combining List and Non-Null" section of spec][1].
    ///
    /// [`Null`]: [`InputValue::Null`]
    /// [1]: https://spec.graphql.org/October2021#sec-Combining-List-and-Non-Null
    Null,

    /// Wrong count of items.
    WrongCount {
        /// Actual count of items.
        actual: usize,

        /// Expected count of items.
        expected: usize,
    },

    /// Error of converting [`InputValue::List`]'s item.
    Item(T::Error),
}

impl<T, S> IntoFieldError<S> for FromInputValueArrayError<T, S>
where
    T: FromInputValue<S>,
    T::Error: IntoFieldError<S>,
    S: ScalarValue,
{
    fn into_field_error(self) -> FieldError<S> {
        const ERROR_PREFIX: &str = "Failed to convert into exact-size array";
        match self {
            Self::Null => format!("{ERROR_PREFIX}: Value cannot be `null`").into(),
            Self::WrongCount { actual, expected } => {
                format!("{ERROR_PREFIX}: wrong elements count: {actual} instead of {expected}",)
                    .into()
            }
            Self::Item(s) => s.into_field_error(),
        }
    }
}

fn resolve_into_list<'t, S, T, I>(
    executor: &Executor<T::Context, S>,
    info: &T::TypeInfo,
    iter: I,
) -> ExecutionResult<S>
where
    S: ScalarValue,
    I: Iterator<Item = &'t T> + ExactSizeIterator,
    T: GraphQLValue<S> + ?Sized + 't,
{
    let stop_on_null = executor
        .current_type()
        .list_contents()
        .expect("Current type is not a list type")
        .is_non_null();
    let mut result = Vec::with_capacity(iter.len());

    for o in iter {
        let val = executor.resolve(info, o)?;
        if stop_on_null && val.is_null() {
            return Ok(val);
        } else {
            result.push(val)
        }
    }

    Ok(Value::list(result))
}

async fn resolve_into_list_async<'a, 't, S, T, I>(
    executor: &'a Executor<'a, 'a, T::Context, S>,
    info: &'a T::TypeInfo,
    items: I,
) -> ExecutionResult<S>
where
    I: Iterator<Item = &'t T> + ExactSizeIterator,
    T: GraphQLValueAsync<S> + ?Sized + 't,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    use futures::stream::{FuturesOrdered, StreamExt as _};

    let stop_on_null = executor
        .current_type()
        .list_contents()
        .expect("Current type is not a list type")
        .is_non_null();

    let mut futures = items
        .map(|it| async move { executor.resolve_into_value_async(info, it).await })
        .collect::<FuturesOrdered<_>>();

    let mut values = Vec::with_capacity(futures.len());
    while let Some(value) = futures.next().await {
        if stop_on_null && value.is_null() {
            return Ok(value);
        }
        values.push(value);
    }

    Ok(Value::list(values))
}

#[cfg(test)]
mod coercion {
    use crate::{graphql_input_value, FromInputValue as _, InputValue, IntoFieldError as _};

    use super::{FromInputValueArrayError, FromInputValueVecError};

    type V = InputValue;

    #[test]
    fn option() {
        let v: V = graphql_input_value!(null);
        assert_eq!(<Option<i32>>::from_input_value(&v), Ok(None));

        let v: V = graphql_input_value!(1);
        assert_eq!(<Option<i32>>::from_input_value(&v), Ok(Some(1)));
    }

    // See "Input Coercion" examples on List types:
    // https://spec.graphql.org/October2021/#sec-List.Input-Coercion
    #[test]
    fn vec() {
        let v: V = graphql_input_value!(null);
        assert_eq!(
            <Vec<i32>>::from_input_value(&v),
            Err(FromInputValueVecError::Null),
        );
        assert_eq!(
            <Vec<Option<i32>>>::from_input_value(&v),
            Err(FromInputValueVecError::Null),
        );
        assert_eq!(<Option<Vec<i32>>>::from_input_value(&v), Ok(None));
        assert_eq!(<Option<Vec<Option<i32>>>>::from_input_value(&v), Ok(None));
        assert_eq!(
            <Vec<Vec<i32>>>::from_input_value(&v),
            Err(FromInputValueVecError::Null),
        );
        assert_eq!(
            <Option<Vec<Option<Vec<Option<i32>>>>>>::from_input_value(&v),
            Ok(None),
        );

        let v: V = graphql_input_value!(1);
        assert_eq!(<Vec<i32>>::from_input_value(&v), Ok(vec![1]));
        assert_eq!(<Vec<Option<i32>>>::from_input_value(&v), Ok(vec![Some(1)]));
        assert_eq!(<Option<Vec<i32>>>::from_input_value(&v), Ok(Some(vec![1])));
        assert_eq!(
            <Option<Vec<Option<i32>>>>::from_input_value(&v),
            Ok(Some(vec![Some(1)])),
        );
        assert_eq!(<Vec<Vec<i32>>>::from_input_value(&v), Ok(vec![vec![1]]));
        assert_eq!(
            <Option<Vec<Option<Vec<Option<i32>>>>>>::from_input_value(&v),
            Ok(Some(vec![Some(vec![Some(1)])])),
        );

        let v: V = graphql_input_value!([1, 2, 3]);
        assert_eq!(<Vec<i32>>::from_input_value(&v), Ok(vec![1, 2, 3]));
        assert_eq!(
            <Option<Vec<i32>>>::from_input_value(&v),
            Ok(Some(vec![1, 2, 3])),
        );
        assert_eq!(
            <Vec<Option<i32>>>::from_input_value(&v),
            Ok(vec![Some(1), Some(2), Some(3)]),
        );
        assert_eq!(
            <Option<Vec<Option<i32>>>>::from_input_value(&v),
            Ok(Some(vec![Some(1), Some(2), Some(3)])),
        );
        assert_eq!(
            <Vec<Vec<i32>>>::from_input_value(&v),
            Ok(vec![vec![1], vec![2], vec![3]]),
        );
        // Looks like the spec ambiguity.
        // See: https://github.com/graphql/graphql-spec/pull/515
        assert_eq!(
            <Option<Vec<Option<Vec<Option<i32>>>>>>::from_input_value(&v),
            Ok(Some(vec![
                Some(vec![Some(1)]),
                Some(vec![Some(2)]),
                Some(vec![Some(3)]),
            ])),
        );

        let v: V = graphql_input_value!([1, 2, null]);
        assert_eq!(
            <Vec<i32>>::from_input_value(&v),
            Err(FromInputValueVecError::Item(
                "Expected `Int`, found: null".into_field_error(),
            )),
        );
        assert_eq!(
            <Option<Vec<i32>>>::from_input_value(&v),
            Err(FromInputValueVecError::Item(
                "Expected `Int`, found: null".into_field_error(),
            )),
        );
        assert_eq!(
            <Vec<Option<i32>>>::from_input_value(&v),
            Ok(vec![Some(1), Some(2), None]),
        );
        assert_eq!(
            <Option<Vec<Option<i32>>>>::from_input_value(&v),
            Ok(Some(vec![Some(1), Some(2), None])),
        );
        assert_eq!(
            <Vec<Vec<i32>>>::from_input_value(&v),
            Err(FromInputValueVecError::Item(FromInputValueVecError::Null)),
        );
        // Looks like the spec ambiguity.
        // See: https://github.com/graphql/graphql-spec/pull/515
        assert_eq!(
            <Option<Vec<Option<Vec<Option<i32>>>>>>::from_input_value(&v),
            Ok(Some(vec![Some(vec![Some(1)]), Some(vec![Some(2)]), None])),
        );
    }

    // See "Input Coercion" examples on List types:
    // https://spec.graphql.org/October2021#sec-List.Input-Coercion
    #[test]
    fn array() {
        let v: V = graphql_input_value!(null);
        assert_eq!(
            <[i32; 0]>::from_input_value(&v),
            Err(FromInputValueArrayError::Null),
        );
        assert_eq!(
            <[i32; 1]>::from_input_value(&v),
            Err(FromInputValueArrayError::Null),
        );
        assert_eq!(
            <[Option<i32>; 0]>::from_input_value(&v),
            Err(FromInputValueArrayError::Null),
        );
        assert_eq!(
            <[Option<i32>; 1]>::from_input_value(&v),
            Err(FromInputValueArrayError::Null),
        );
        assert_eq!(<Option<[i32; 0]>>::from_input_value(&v), Ok(None));
        assert_eq!(<Option<[i32; 1]>>::from_input_value(&v), Ok(None));
        assert_eq!(<Option<[Option<i32>; 0]>>::from_input_value(&v), Ok(None));
        assert_eq!(<Option<[Option<i32>; 1]>>::from_input_value(&v), Ok(None));
        assert_eq!(
            <[[i32; 1]; 1]>::from_input_value(&v),
            Err(FromInputValueArrayError::Null),
        );
        assert_eq!(
            <Option<[Option<[Option<i32>; 1]>; 1]>>::from_input_value(&v),
            Ok(None),
        );

        let v: V = graphql_input_value!(1);
        assert_eq!(<[i32; 1]>::from_input_value(&v), Ok([1]));
        assert_eq!(
            <[i32; 0]>::from_input_value(&v),
            Err(FromInputValueArrayError::WrongCount {
                expected: 0,
                actual: 1,
            }),
        );
        assert_eq!(<[Option<i32>; 1]>::from_input_value(&v), Ok([Some(1)]));
        assert_eq!(<Option<[i32; 1]>>::from_input_value(&v), Ok(Some([1])));
        assert_eq!(
            <Option<[Option<i32>; 1]>>::from_input_value(&v),
            Ok(Some([Some(1)])),
        );
        assert_eq!(<[[i32; 1]; 1]>::from_input_value(&v), Ok([[1]]));
        assert_eq!(
            <Option<[Option<[Option<i32>; 1]>; 1]>>::from_input_value(&v),
            Ok(Some([Some([Some(1)])])),
        );

        let v: V = graphql_input_value!([1, 2, 3]);
        assert_eq!(<[i32; 3]>::from_input_value(&v), Ok([1, 2, 3]));
        assert_eq!(
            <Option<[i32; 3]>>::from_input_value(&v),
            Ok(Some([1, 2, 3])),
        );
        assert_eq!(
            <[Option<i32>; 3]>::from_input_value(&v),
            Ok([Some(1), Some(2), Some(3)]),
        );
        assert_eq!(
            <Option<[Option<i32>; 3]>>::from_input_value(&v),
            Ok(Some([Some(1), Some(2), Some(3)])),
        );
        assert_eq!(<[[i32; 1]; 3]>::from_input_value(&v), Ok([[1], [2], [3]]));
        // Looks like the spec ambiguity.
        // See: https://github.com/graphql/graphql-spec/pull/515
        assert_eq!(
            <Option<[Option<[Option<i32>; 1]>; 3]>>::from_input_value(&v),
            Ok(Some([Some([Some(1)]), Some([Some(2)]), Some([Some(3)]),])),
        );

        let v: V = graphql_input_value!([1, 2, null]);
        assert_eq!(
            <[i32; 3]>::from_input_value(&v),
            Err(FromInputValueArrayError::Item(
                "Expected `Int`, found: null".into_field_error(),
            )),
        );
        assert_eq!(
            <Option<[i32; 3]>>::from_input_value(&v),
            Err(FromInputValueArrayError::Item(
                "Expected `Int`, found: null".into_field_error(),
            )),
        );
        assert_eq!(
            <[Option<i32>; 3]>::from_input_value(&v),
            Ok([Some(1), Some(2), None]),
        );
        assert_eq!(
            <Option<[Option<i32>; 3]>>::from_input_value(&v),
            Ok(Some([Some(1), Some(2), None])),
        );
        assert_eq!(
            <[[i32; 1]; 3]>::from_input_value(&v),
            Err(FromInputValueArrayError::Item(
                FromInputValueArrayError::Null
            )),
        );
        // Looks like the spec ambiguity.
        // See: https://github.com/graphql/graphql-spec/pull/515
        assert_eq!(
            <Option<[Option<[Option<i32>; 1]>; 3]>>::from_input_value(&v),
            Ok(Some([Some([Some(1)]), Some([Some(2)]), None])),
        );
    }
}
