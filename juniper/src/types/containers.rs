use crate::{
    ast::{FromInputValue, InputValue, Selection, ToInputValue},
    executor::{ExecutionResult, Executor, Registry},
    schema::meta::MetaType,
    types::base::GraphQLType,
    value::{ScalarValue, Value},
};
use core::ops::{Range, RangeInclusive};

impl<S, T, CtxT> GraphQLType<S> for Option<T>
where
    S: ScalarValue,
    T: GraphQLType<S, Context = CtxT>,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(_: &T::TypeInfo) -> Option<&str> {
        None
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_nullable_type::<T>(info).into_meta()
    }

    fn resolve(
        &self,
        info: &T::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> ExecutionResult<S> {
        match *self {
            Some(ref obj) => executor.resolve(info, obj),
            None => Ok(Value::null()),
        }
    }
}

impl<S, T> FromInputValue<S> for Option<T>
where
    T: FromInputValue<S>,
    S: ScalarValue,
{
    fn from_input_value<'a>(v: &'a InputValue<S>) -> Option<Option<T>> {
        match v {
            &InputValue::Null => Some(None),
            v => match v.convert() {
                Some(x) => Some(Some(x)),
                None => None,
            },
        }
    }
}

impl<S, T> ToInputValue<S> for Option<T>
where
    T: ToInputValue<S>,
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        match *self {
            Some(ref v) => v.to_input_value(),
            None => InputValue::null(),
        }
    }
}

impl<S, T, CtxT> GraphQLType<S> for Vec<T>
where
    T: GraphQLType<S, Context = CtxT>,
    S: ScalarValue,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(_: &T::TypeInfo) -> Option<&str> {
        None
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_list_type::<T>(info).into_meta()
    }

    fn resolve(
        &self,
        info: &T::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> ExecutionResult<S> {
        resolve_into_list(executor, info, self.iter())
    }
}

impl<T, S> FromInputValue<S> for Vec<T>
where
    T: FromInputValue<S>,
    S: ScalarValue,
{
    fn from_input_value<'a>(v: &'a InputValue<S>) -> Option<Vec<T>> {
        match *v {
            InputValue::List(ref ls) => {
                let v: Vec<_> = ls.iter().filter_map(|i| i.item.convert()).collect();

                if v.len() == ls.len() {
                    Some(v)
                } else {
                    None
                }
            }
            ref other => {
                if let Some(e) = other.convert() {
                    Some(vec![e])
                } else {
                    None
                }
            }
        }
    }
}

impl<T, S> ToInputValue<S> for Vec<T>
where
    T: ToInputValue<S>,
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        InputValue::list(self.iter().map(|v| v.to_input_value()).collect())
    }
}

impl<'a, S, T, CtxT> GraphQLType<S> for &'a [T]
where
    S: ScalarValue,
    T: GraphQLType<S, Context = CtxT>,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(_: &T::TypeInfo) -> Option<&str> {
        None
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_list_type::<T>(info).into_meta()
    }

    fn resolve(
        &self,
        info: &T::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> ExecutionResult<S> {
        resolve_into_list(executor, info, self.iter())
    }
}

impl<'a, T, S> ToInputValue<S> for &'a [T]
where
    T: ToInputValue<S>,
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        InputValue::list(self.iter().map(|v| v.to_input_value()).collect())
    }
}

fn resolve_into_list<S, T, I>(
    executor: &Executor<T::Context, S>,
    info: &T::TypeInfo,
    iter: I,
) -> ExecutionResult<S>
where
    S: ScalarValue,
    I: Iterator<Item = T> + ExactSizeIterator,
    T: GraphQLType<S>,
{
    let stop_on_null = executor
        .current_type()
        .list_contents()
        .expect("Current type is not a list type")
        .is_non_null();
    let mut result = Vec::with_capacity(iter.len());

    for o in iter {
        match executor.resolve(info, &o) {
            Ok(value) => {
                if stop_on_null && value.is_null() {
                    return Ok(value);
                } else {
                    result.push(value)
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok(Value::list(result))
}

async fn resolve_into_list_async<'a, S, T, I>(
    executor: &'a Executor<'a, 'a, T::Context, S>,
    info: &'a T::TypeInfo,
    items: I,
) -> ExecutionResult<S>
where
    S: ScalarValue + Send + Sync,
    I: Iterator<Item = T> + ExactSizeIterator,
    T: crate::GraphQLTypeAsync<S>,
    T::TypeInfo: Send + Sync,
    T::Context: Send + Sync,
{
    use futures::stream::{FuturesOrdered, StreamExt as _};
    use std::iter::FromIterator;

    let stop_on_null = executor
        .current_type()
        .list_contents()
        .expect("Current type is not a list type")
        .is_non_null();

    let iter =
        items.map(|item| async move { executor.resolve_into_value_async(info, &item).await });
    let mut futures = FuturesOrdered::from_iter(iter);

    let mut values = Vec::with_capacity(futures.len());
    while let Some(value) = futures.next().await {
        if stop_on_null && value.is_null() {
            return Ok(value);
        }
        values.push(value);
    }

    Ok(Value::list(values))
}

impl<S, T, CtxT> crate::GraphQLTypeAsync<S> for Vec<T>
where
    T: crate::GraphQLTypeAsync<S, Context = CtxT>,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
    CtxT: Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> crate::BoxFuture<'a, ExecutionResult<S>> {
        let f = resolve_into_list_async(executor, info, self.iter());
        Box::pin(f)
    }
}

impl<S, T, CtxT> crate::GraphQLTypeAsync<S> for &[T]
where
    T: crate::GraphQLTypeAsync<S, Context = CtxT>,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
    CtxT: Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> crate::BoxFuture<'a, ExecutionResult<S>> {
        let f = resolve_into_list_async(executor, info, self.iter());
        Box::pin(f)
    }
}

impl<S, T, CtxT> crate::GraphQLTypeAsync<S> for Option<T>
where
    T: crate::GraphQLTypeAsync<S, Context = CtxT>,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
    CtxT: Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> crate::BoxFuture<'a, ExecutionResult<S>> {
        let f = async move {
            let value = match *self {
                Some(ref obj) => executor.resolve_into_value_async(info, obj).await,
                None => Value::null(),
            };
            Ok(value)
        };
        Box::pin(f)
    }
}

macro_rules! impl_range {
    ($range:ident, $range_new:path, $range_start:path, $range_end:path) => {
        impl<S, T, CtxT> GraphQLType<S> for $range<T>
        where
            T: GraphQLType<S, Context = CtxT, TypeInfo = ()>,
            S: ScalarValue,
        {
            type Context = CtxT;
            type TypeInfo = T::TypeInfo;
        
            fn name(_: &T::TypeInfo) -> Option<&str> {
                None
            }
        
            fn meta<'r>(i: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
            where
                S: 'r,
            {
                let fields = &[
                    registry.field::<i32>("start", i),
                    registry.field::<i32>("end", i),
                ];
                registry.build_object_type::<Self>(i, fields).into_meta()
            }
        
            fn resolve(
                &self,
                i: &T::TypeInfo,
                _: Option<&[Selection<S>]>,
                executor: &Executor<CtxT, S>,
            ) -> ExecutionResult<S> {
                let start = executor.resolve(i, $range_start(self))?;
                let end = executor.resolve(i, $range_end(self))?;
                Ok(Value::object(
                    vec![("start", start), ("end", end)].into_iter().collect(),
                ))
            }
        }
        
        impl<T, S> FromInputValue<S> for $range<T>
        where
            T: FromInputValue<S>,
            S: ScalarValue,
        {
        fn from_input_value<'a>(v: &'a InputValue<S>) -> Option<$range<T>> {
                match *v {
                    InputValue::Object(ref o) => {
                        let start = if let Some(Some(i)) = o.get(0).map(|tuple| tuple.1.item.convert()) {
                            i
                        } else {
                            return None;
                        };
                        let end = if let Some(Some(i)) = o.get(1).map(|tuple| tuple.1.item.convert()) {
                            i
                        } else {
                            return None;
                        };
                        Some($range_new(start, end))
                    }
                    ref other => {
                        if let Some(r) = other.convert() {
                            Some(r)
                        } else {
                            None
                        }
                    }
                }
            }
        }
        
        impl<T, S> ToInputValue<S> for $range<T>
        where
            T: ToInputValue<S>,
            S: ScalarValue,
        {
            fn to_input_value(&self) -> InputValue<S> {
                InputValue::object(
                    vec![
                        ("start", $range_start(self).to_input_value()),
                        ("end", $range_end(self).to_input_value()),
                    ]
                    .into_iter()
                    .collect(),
                )
            }
        }        
    };
}

impl_range!(Range, range_new, range_start, range_end);
impl_range!(RangeInclusive, RangeInclusive::new, RangeInclusive::start, RangeInclusive::end);

fn range_new<T>(start: T, end: T) -> Range<T> {
    Range { start, end }
}

fn range_start<T>(range: &Range<T>) -> &T {
    &range.start
}

fn range_end<T>(range: &Range<T>) -> &T {
    &range.end
}