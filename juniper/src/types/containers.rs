use ast::{FromInputValue, InputValue, Selection, ToInputValue};
use schema::meta::MetaType;
use value::{ScalarRefValue, ScalarValue, Value};

use executor::{Executor, Registry};
use types::base::GraphQLType;

impl<S, T, CtxT> GraphQLType<S> for Option<T>
where
    S: ScalarValue,
    T: GraphQLType<S, Context = CtxT>,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(_: &T::TypeInfo) -> Option<&str> {
        None
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        registry.build_nullable_type::<T>(info).into_meta()
    }

    fn resolve(
        &self,
        info: &T::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> Value<S> {
        match *self {
            Some(ref obj) => executor.resolve_into_value(info, obj),
            None => Value::null(),
        }
    }
}

impl<S, T> FromInputValue<S> for Option<T>
where
    T: FromInputValue<S>,
    S: ScalarValue,
{
    fn from_input_value<'a>(v: &'a InputValue<S>) -> Option<Option<T>>
    where
        for<'b> &'b S: ScalarRefValue<'b>,
    {
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
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(_: &T::TypeInfo) -> Option<&str> {
        None
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        registry.build_list_type::<T>(info).into_meta()
    }

    fn resolve(
        &self,
        info: &T::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> Value<S> {
        resolve_into_list(executor, info, self.iter())
    }
}

impl<T, S> FromInputValue<S> for Vec<T>
where
    T: FromInputValue<S>,
    S: ScalarValue,
{
    fn from_input_value<'a>(v: &'a InputValue<S>) -> Option<Vec<T>>
    where
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        match *v {
            InputValue::List(ref ls) => {
                let v: Vec<_> = ls.iter().filter_map(|i| i.item.convert()).collect();

                if v.len() == ls.len() {
                    Some(v)
                } else {
                    None
                }
            }
            ref other => if let Some(e) = other.convert() {
                Some(vec![e])
            } else {
                None
            },
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
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = CtxT;
    type TypeInfo = T::TypeInfo;

    fn name(_: &T::TypeInfo) -> Option<&str> {
        None
    }

    fn meta<'r>(info: &T::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        registry.build_list_type::<T>(info).into_meta()
    }

    fn resolve(
        &self,
        info: &T::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<CtxT, S>,
    ) -> Value<S> {
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
) -> Value<S>
where
    S: ScalarValue,
    I: Iterator<Item = T> + ExactSizeIterator,
    T: GraphQLType<S>,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    let stop_on_null = executor
        .current_type()
        .list_contents()
        .expect("Current type is not a list type")
        .is_non_null();

    let mut result = Vec::with_capacity(iter.len());

    for o in iter {
        let value = executor.resolve_into_value(info, &o);
        if stop_on_null && value.is_null() {
            return value;
        }

        result.push(value);
    }

    Value::list(result)
}
