use crate::{
    ast::{FromInputValue, InputValue, Selection, ToInputValue},
    executor::{ExecutionResult, Executor, Registry},
    schema::meta::MetaType,
    types::{
        async_await::GraphQLValueAsync,
        base::{GraphQLType, GraphQLValue},
    },
    value::Value,
};

impl<T> GraphQLType for Option<T>
where
    T: GraphQLType,
{
    fn name(_: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r> {
        registry.build_nullable_type::<T>(info).into_meta()
    }
}

impl<T> GraphQLValue for Option<T>
where
    T: GraphQLValue,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, _: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        _: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        match *self {
            Some(ref obj) => executor.resolve(info, obj),
            None => Ok(Value::null()),
        }
    }
}

impl<T> GraphQLValueAsync for Option<T>
where
    T: GraphQLValueAsync,
    T::TypeInfo: Sync,
    T::Context: Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _: Option<&'a [Selection]>,
        executor: &'a Executor<Self::Context>,
    ) -> crate::BoxFuture<'a, ExecutionResult> {
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

impl<T> FromInputValue for Option<T>
where
    T: FromInputValue,
{
    fn from_input_value(v: &InputValue) -> Option<Option<T>> {
        match v {
            &InputValue::Null => Some(None),
            v => v.convert().map(Some),
        }
    }
}

impl<T> ToInputValue for Option<T>
where
    T: ToInputValue,
{
    fn to_input_value(&self) -> InputValue {
        match *self {
            Some(ref v) => v.to_input_value(),
            None => InputValue::null(),
        }
    }
}

impl<T> GraphQLType for Vec<T>
where
    T: GraphQLType,
{
    fn name(_: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r> {
        registry.build_list_type::<T>(info).into_meta()
    }
}

impl<T> GraphQLValue for Vec<T>
where
    T: GraphQLValue,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, _: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        _: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        resolve_into_list(executor, info, self.iter())
    }
}

impl<T> GraphQLValueAsync for Vec<T>
where
    T: GraphQLValueAsync,
    T::TypeInfo: Sync,
    T::Context: Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _: Option<&'a [Selection]>,
        executor: &'a Executor<Self::Context>,
    ) -> crate::BoxFuture<'a, ExecutionResult> {
        let f = resolve_into_list_async(executor, info, self.iter());
        Box::pin(f)
    }
}

impl<T> FromInputValue for Vec<T>
where
    T: FromInputValue,
{
    fn from_input_value(v: &InputValue) -> Option<Vec<T>>
where {
        match *v {
            InputValue::List(ref ls) => {
                let v: Vec<_> = ls.iter().filter_map(|i| i.item.convert()).collect();

                if v.len() == ls.len() {
                    Some(v)
                } else {
                    None
                }
            }
            ref other => other.convert().map(|e| vec![e]),
        }
    }
}

impl<T> ToInputValue for Vec<T>
where
    T: ToInputValue,
{
    fn to_input_value(&self) -> InputValue {
        InputValue::list(self.iter().map(T::to_input_value).collect())
    }
}

impl<T> GraphQLType for [T]
where
    T: GraphQLType,
{
    fn name(_: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r> {
        registry.build_list_type::<T>(info).into_meta()
    }
}

impl<T> GraphQLValue for [T]
where
    T: GraphQLValue,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, _: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        _: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        resolve_into_list(executor, info, self.iter())
    }
}

impl<T> GraphQLValueAsync for [T]
where
    T: GraphQLValueAsync,
    T::TypeInfo: Sync,
    T::Context: Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _: Option<&'a [Selection]>,
        executor: &'a Executor<Self::Context>,
    ) -> crate::BoxFuture<'a, ExecutionResult> {
        let f = resolve_into_list_async(executor, info, self.iter());
        Box::pin(f)
    }
}

impl<'a, T> ToInputValue for &'a [T]
where
    T: ToInputValue,
{
    fn to_input_value(&self) -> InputValue {
        InputValue::list(self.iter().map(T::to_input_value).collect())
    }
}

fn resolve_into_list<'t, T, I>(
    executor: &Executor<T::Context>,
    info: &T::TypeInfo,
    iter: I,
) -> ExecutionResult
where
    I: Iterator<Item = &'t T> + ExactSizeIterator,
    T: GraphQLValue + ?Sized + 't,
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

async fn resolve_into_list_async<'a, 't, T, I>(
    executor: &'a Executor<'a, 'a, T::Context>,
    info: &'a T::TypeInfo,
    items: I,
) -> ExecutionResult
where
    I: Iterator<Item = &'t T> + ExactSizeIterator,
    T: GraphQLValueAsync + ?Sized + 't,
    T::TypeInfo: Sync,
    T::Context: Sync,
{
    use futures::stream::{FuturesOrdered, StreamExt as _};
    use std::iter::FromIterator;

    let stop_on_null = executor
        .current_type()
        .list_contents()
        .expect("Current type is not a list type")
        .is_non_null();

    let iter = items.map(|it| async move { executor.resolve_into_value_async(info, it).await });
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
