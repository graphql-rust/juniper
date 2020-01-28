use crate::{
    ast::Selection,
    executor::{ExecutionResult, Executor},
    parser::Spanning,
    value::{Object, ScalarValue, Value},
};

#[cfg(feature = "async")]
use crate::BoxFuture;

use super::base::{is_excluded, merge_key_into, Arguments, GraphQLType};

/**
This trait extends `GraphQLType` with asynchronous queries/mutations resolvers.

Convenience macros related to asynchronous queries/mutations expand into an
implementation of this trait and `GraphQLType` for the given type.

This trait's execution logic is similar to `GraphQLType`.

## Manual implementation example

```rust
use async_trait::async_trait;
use juniper::{
    meta::MetaType, DefaultScalarValue, FieldError,
    GraphQLType, GraphQLTypeAsync, Registry, Value, ValuesIterator,
};

#[derive(Debug)]
struct User {
    id: String,
    name: String,
    friend_ids: Vec<String>,
}

#[juniper::object]
impl User {}

struct Query;

// GraphQLType should be implemented in order to add `Query`
// to `juniper::RootNode`. In this example it is implemented manually
// to show that only `name` and `meta` methods are used, not the ones
// containing execution logic.
impl GraphQLType for Query {
    type Context = ();
    type TypeInfo = ();

    fn name(_: &Self::TypeInfo) -> Option<&str> {
        Some("Query")
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        juniper::DefaultScalarValue: 'r,
    {
        let fields = vec![registry.field_convert::<User, _, Self::Context>("users", info)];
        let meta = registry.build_object_type::<Query>(info, &fields);
        meta.into_meta()
    }
}

// async_trait[1] is used in this example for convenience, though this trait
// can be implemented without async_trait (subscription macros do not
// use async_trait, for example)
// [1](https://github.com/dtolnay/async-trait)
#[async_trait]
impl GraphQLTypeAsync<DefaultScalarValue> for Query {
    // This function is called every time a field is found
    async fn resolve_field_async<'a>(
        &'a self,
        type_info: &'a <Self as GraphQLType>::TypeInfo,
        field_name: &'a str,
        args: &'a juniper::Arguments<'a>,
        executor: &'a juniper::Executor<'a, <Self as GraphQLType>::Context>,
    ) -> juniper::ExecutionResult {
        match field_name {
            "users" => {
                let user = User {
                    id: "1".to_string(),
                    name: "user".to_string(),
                    friend_ids: vec!["2".to_string(), "3".to_string(), "4".to_string()],
                };
                // Pass returned object as context and keep resolving query to
                // filter out unnecessary fields.
                let res: Result<Option<(&<Self as GraphQLType>::Context, _)>, _> =
                    juniper::IntoResolvable::into(user, executor.context());
                match res {
                    Ok(Some((ctx, r))) => {
                        let sub = executor.replaced_context(ctx);
                        sub.resolve_with_ctx_async::<<Self as GraphQLType>::Context, _>(&(), &r)
                            .await
                    }
                    Ok(None) => Ok(juniper::Value::null()),
                    Err(e) => Err(e),
                }
            },
            // panicking here because juniper should return field does not exist
            // error while parsing subscription query
            _ => panic!("Field {:?} not found on type Query"),
        }
    }
}
```
*/
pub trait GraphQLTypeAsync<S>: GraphQLType<S> + Send + Sync
where
    Self::Context: Send + Sync,
    Self::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
{
    /// Resolve the provided selection set against the current object.
    /// This method is called by executor and should call other methods on this
    /// trait (if needed).
    ///
    /// It is similar to `GraphQLType::resolve` except that it
    /// returns a future which resolves into a single value.
    ///
    ///  ## Default implementation
    ///
    /// For object types, the default implementation calls `resolve_field_async`
    /// on each field and `resolve_into_type_async` for each fragment in
    /// provided selection set.
    ///
    /// For non-object types, the selection set will be `None` and default
    /// implementation will panic.
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

    /// This method is similar to `GraphQLType::resolve_field`, but it returns
    /// future that resolves into value instead of value.
    ///
    /// The default implementation panics.
    fn resolve_field_async<'a>(
        &'a self,
        _: &'a Self::TypeInfo,   // this query's type info
        _: &'a str,              // field's type name
        _: &'a Arguments<'a, S>, // field's arguments
        _: &'a Executor<'a, 'a, Self::Context, S>, // field's executor (query's sub-executor
                                 // with current field's selection set)
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        panic!("resolve_field must be implemented by object types");
    }

    /// This method is similar to `GraphQLType::resolve_into_type`, but it
    /// returns future that resolves into value instead of value.
    ///
    /// Default implementation resolves fragments with the same type as `Self`
    /// and panics otherwise.
    fn resolve_into_type_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        type_name: &str,
        selection_set: Option<&'a [Selection<'a, S>]>,
        executor: &'a Executor<'a, 'a, Self::Context, S>,
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
fn resolve_selection_set_into_async<'a, 'e, T, CtxT, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    selection_set: &'e [Selection<'e, S>],
    executor: &'e Executor<'e, 'e, CtxT, S>,
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
async fn resolve_selection_set_into_async_recursive<'a, T, CtxT, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    selection_set: &'a [Selection<'a, S>],
    executor: &'a Executor<'a, 'a, CtxT, S>,
) -> Value<S>
where
    T: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
    CtxT: Send + Sync,
{
    use futures::stream::{FuturesOrdered, StreamExt as _};

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
