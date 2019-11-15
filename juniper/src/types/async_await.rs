use crate::{
    ast::{Directive, FromInputValue, InputValue, Selection},
    value::{Object, ScalarValue, Value},
};

use crate::{
    executor::{ExecutionResult, Executor},
    parser::Spanning,
};

use crate::BoxFuture;

use super::base::{is_excluded, merge_key_into, Arguments, GraphQLType};

pub trait GraphQLTypeAsync<S>: GraphQLType<S> + Send + Sync
where
    Self::Context: Send + Sync,
    Self::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_field_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        field_name: &'a str,
        arguments: &'a Arguments<S>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        panic!("resolve_field must be implemented by object types");
    }

    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        if let Some(selection_set) = selection_set {
            Box::pin(async move {
                let value =
                    resolve_selection_set_into_async(self, info, selection_set, executor).await;
                Ok(value)
            })
        } else {
            panic!("resolve() must be implemented by non-object output types");
        }
    }

    fn resolve_into_type_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        type_name: &str,
        selection_set: Option<&'a [Selection<'a, S>]>,
        executor: &'a Executor<'a, Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        if Self::name(info).unwrap() == type_name {
            self.resolve_async(info, selection_set, executor)
        } else {
            panic!("resolve_into_type_async must be implemented by unions and interfaces");
        }
    }
}

// Wrapper function around resolve_selection_set_into_async_recursive.
// This wrapper is necessary because async fns can not be recursive.
#[cfg(feature = "async")]
pub(crate) fn resolve_selection_set_into_async<'a, 'e, T, CtxT, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    selection_set: &'e [Selection<'e, S>],
    executor: &'e Executor<'e, CtxT, S>,
) -> BoxFuture<'a, Value<S>>
where
    T: GraphQLTypeAsync<S, Context = CtxT>,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
    CtxT: Send + Sync,
    'e: 'a,
{
    Box::pin(resolve_selection_set_into_async_recursive(
        instance,
        info,
        selection_set,
        executor,
    ))
}

struct AsyncField<S> {
    name: String,
    value: Option<Value<S>>,
}

enum AsyncValue<S> {
    Field(AsyncField<S>),
    Nested(Value<S>),
}

#[cfg(feature = "async")]
pub(crate) async fn resolve_selection_set_into_async_recursive<'a, T, CtxT, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    selection_set: &'a [Selection<'a, S>],
    executor: &'a Executor<'a, CtxT, S>,
) -> Value<S>
where
    T: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
    CtxT: Send + Sync,
{
    use futures::stream::{FuturesOrdered, StreamExt};

    let mut object = Object::with_capacity(selection_set.len());

    let mut async_values = FuturesOrdered::<BoxFuture<'a, AsyncValue<S>>>::new();

    let meta_type = executor
        .schema()
        .concrete_type_by_name(
            T::name(info)
                .expect("Resolving named type's selection set")
                .as_ref(),
        )
        .expect("Type not found in schema");

    for selection in selection_set {
        match *selection {
            Selection::Field(Spanning {
                item: ref f,
                start: ref start_pos,
                ..
            }) => {
                if is_excluded(&f.directives, executor.variables()) {
                    continue;
                }

                let response_name = f.alias.as_ref().unwrap_or(&f.name).item;

                if f.name.item == "__typename" {
                    object.add_field(
                        response_name,
                        Value::scalar(instance.concrete_type_name(executor.context(), info)),
                    );
                    continue;
                }

                let meta_field = meta_type.field_by_name(f.name.item).unwrap_or_else(|| {
                    panic!(format!(
                        "Field {} not found on type {:?}",
                        f.name.item,
                        meta_type.name()
                    ))
                });

                let exec_vars = executor.variables();

                let sub_exec = executor.field_sub_executor(
                    &response_name,
                    f.name.item,
                    start_pos.clone(),
                    f.selection_set.as_ref().map(|v| &v[..]),
                );
                let args = Arguments::new(
                    f.arguments.as_ref().map(|m| {
                        m.item
                            .iter()
                            .map(|&(ref k, ref v)| (k.item, v.item.clone().into_const(exec_vars)))
                            .collect()
                    }),
                    &meta_field.arguments,
                );

                let pos = start_pos.clone();
                let is_non_null = meta_field.field_type.is_non_null();

                let response_name = response_name.to_string();
                let field_future = async move {
                    // TODO: implement custom future type instead of
                    //       two-level boxing.
                    let res = instance
                        .resolve_field_async(info, f.name.item, &args, &sub_exec)
                        .await;

                    let value = match res {
                        Ok(Value::Null) if is_non_null => None,
                        Ok(v) => Some(v),
                        Err(e) => {
                            sub_exec.push_error_at(e, pos);

                            if is_non_null {
                                None
                            } else {
                                Some(Value::null())
                            }
                        }
                    };
                    AsyncValue::Field(AsyncField {
                        name: response_name,
                        value,
                    })
                };
                async_values.push(Box::pin(field_future));
            }
            Selection::FragmentSpread(Spanning {
                item: ref spread, ..
            }) => {
                if is_excluded(&spread.directives, executor.variables()) {
                    continue;
                }

                // TODO: prevent duplicate boxing.
                let f = async move {
                    let fragment = &executor
                        .fragment_by_name(spread.name.item)
                        .expect("Fragment could not be found");
                    let value = resolve_selection_set_into_async(
                        instance,
                        info,
                        &fragment.selection_set[..],
                        executor,
                    )
                    .await;
                    AsyncValue::Nested(value)
                };
                async_values.push(Box::pin(f));
            }
            Selection::InlineFragment(Spanning {
                item: ref fragment,
                start: ref start_pos,
                ..
            }) => {
                if is_excluded(&fragment.directives, executor.variables()) {
                    continue;
                }

                let sub_exec = executor.type_sub_executor(
                    fragment.type_condition.as_ref().map(|c| c.item),
                    Some(&fragment.selection_set[..]),
                );

                if let Some(ref type_condition) = fragment.type_condition {
                    let sub_result = instance
                        .resolve_into_type_async(
                            info,
                            type_condition.item,
                            Some(&fragment.selection_set[..]),
                            &sub_exec,
                        )
                        .await;

                    if let Ok(Value::Object(obj)) = sub_result {
                        for (k, v) in obj {
                            merge_key_into(&mut object, &k, v);
                        }
                    } else if let Err(e) = sub_result {
                        sub_exec.push_error_at(e, start_pos.clone());
                    }
                } else {
                    let f = async move {
                        let value = resolve_selection_set_into_async(
                            instance,
                            info,
                            &fragment.selection_set[..],
                            &sub_exec,
                        )
                        .await;
                        AsyncValue::Nested(value)
                    };
                    async_values.push(Box::pin(f));
                }
            }
        }
    }

    while let Some(item) = async_values.next().await {
        match item {
            AsyncValue::Field(AsyncField { name, value }) => {
                if let Some(value) = value {
                    object.add_field(&name, value);
                } else {
                    return Value::null();
                }
            }
            AsyncValue::Nested(obj) => match obj {
                v @ Value::Null => {
                    return v;
                }
                Value::Object(obj) => {
                    for (k, v) in obj {
                        merge_key_into(&mut object, &k, v);
                    }
                }
                _ => unreachable!(),
            },
        }
    }

    Value::Object(object)
}
