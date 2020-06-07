use crate::{
    ast::{FromInputValue, InputValue, Selection, ToInputValue},
    executor::{ExecutionResult, Executor, Registry},
    schema::meta::MetaType,
    types::base::GraphQLType,
    value::{ScalarValue, Value},
    BoxFuture,
};
use futures::stream::{FuturesOrdered, StreamExt};
use std::iter::FromIterator;

impl<S, T, CtxT> GraphQLType<S> for Option<T>
where
    T: GraphQLType<S, Context = CtxT>,
    T::TypeInfo: Send + Sync,
    S: ScalarValue,
    CtxT: Send + Sync,
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

    fn resolve<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        _selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        let f = async move {
            let value = match *self {
                Some(ref obj) => executor.resolve_into_value(info, obj).await,
                None => Value::null(),
            };
            Ok(value)
        };

        Box::pin(f)
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
    T: GraphQLType<S, Context = CtxT> + Send + Sync,
    T::TypeInfo: Send + Sync,
    S: ScalarValue,
    CtxT: Send + Sync,
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

    fn resolve<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        _selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        Box::pin(resolve_into_list(executor, info, self.iter()))
    }
}

impl<T, S> FromInputValue<S> for Vec<T>
where
    T: FromInputValue<S>,
    S: ScalarValue,
{
    fn from_input_value<'a>(v: &'a InputValue<S>) -> Option<Vec<T>>
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
    T: GraphQLType<S, Context = CtxT>,
    T::TypeInfo: Send + Sync,
    S: ScalarValue,
    CtxT: Send + Sync,
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

    fn resolve<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        _selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        Box::pin(resolve_into_list(executor, info, self.iter()))
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

async fn resolve_into_list<'ref_err, 'err, 'ty, S, T, I>(
    executor: &'ref_err Executor<'ref_err, 'err, T::Context, S>,
    info: &'ty T::TypeInfo,
    items: I,
) -> ExecutionResult<S>
where
    S: ScalarValue,
    I: Iterator<Item = T> + ExactSizeIterator,
    T: GraphQLType<S> + Send + Sync,
    T::TypeInfo: Send + Sync,
    T::Context: Send + Sync,
{
    let stop_on_null = executor
        .current_type()
        .list_contents()
        .expect("Current type is not a list type")
        .is_non_null();

    let iter = items.map(|item| executor.resolve_into_value(info, item));
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
