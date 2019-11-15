use std::sync::Arc;

use crate::{ast::Selection, executor::{ExecutionResult, Executor, FieldError, ValuesResultStream}, parser::Spanning, value::{Object, ScalarRefValue, ScalarValue, Value}, FieldResult};

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
#[async_trait::async_trait]
pub trait GraphQLTypeAsync<S>: GraphQLType<S> + Send + Sync
where
    Self::Context: Send + Sync,
    Self::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
    for<'b> &'b S: ScalarRefValue<'b>,
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
    async fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<'a, S>]>,
        executor: &'a Executor<'a, Self::Context, S>,
    ) -> Value<S> {
        if let Some(selection_set) = selection_set {
            resolve_selection_set_into_async(self, info, selection_set, executor).await
        } else {
            panic!("resolve() must be implemented by non-object output types");
        }
    }

    /// This method is similar to `GraphQLType::resolve_field`, but it returns
    /// future that resolves into value instead of value.
    ///
    /// The default implementation panics.
    async fn resolve_field_async<'a>(
        &'a self,
        _: &'a Self::TypeInfo,   // this query's type info
        _: &'a str,              // field's type name
        _: &'a Arguments<'a, S>, // field's arguments
        _: &'a Executor<'a, Self::Context, S>, // field's executor (query's sub-executor
                                 // with current field's selection set)
    ) -> ExecutionResult<S> {
        panic!("resolve_field must be implemented by object types");
    }

    /// This method is similar to `GraphQLType::resolve_into_type`, but it
    /// returns future that resolves into value instead of value.
    ///
    /// Default implementation resolves fragments with the same type as `Self`
    /// and panics otherwise.
    async fn resolve_into_type_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        type_name: &str,
        selection_set: Option<&'a [Selection<'a, S>]>,
        executor: &'a Executor<'a, Self::Context, S>,
    ) -> ExecutionResult<S> {
        if Self::name(info).unwrap() == type_name {
            Ok(self.resolve_async(info, selection_set, executor).await)
        } else {
            panic!("resolve_into_type_async must be implemented by unions and interfaces");
        }
    }
}

/**
*
* This trait replaces GraphQLType`'s resolver logic with asynchronous subscription
* execution logic. It should be used with `GraphQLType` in order to implement
* subscription GraphQL objects.
*
* Asynchronous subscription related convenience macros expand into an
* implementation of this trait and `GraphQLType` for the given type.
*
* See trait methods for more detailed explanation on how this trait works.
*
* Convenience macros related to asynchronous subscriptions expand into an
* implementation of this trait and `GraphQLType` for the given type.
*
* See trait methods descriptions for more details.
*
* ## Manual implementation example
*
* The following example demonstrates how to implement `GraphQLSubscriptionType`
* with default resolver logic (without overwriting `resolve_into_stream`) manually.
*
* Juniper's subscription macros use similar execution logic by default.
*
*
* ```rust
* use async_trait::async_trait;
* use juniper::{
*     GraphQLType, GraphQLSubscriptionType, Value, ValuesStream,
*     FieldError, Registry, meta::MetaType, DefaultScalarValue,
*     FieldResult,
* };
*
* #[derive(Debug)]
* struct User { id: String, name: String, friend_ids: Vec<String> }
*
* #[juniper::object]
* impl User {}
*
* struct Subscription;
*
* // `GraphQLType` should be implemented in order to use this type in `juniper::RootNode`.
* // In this example it is implemented manually to show that only `name` and `meta` methods
* // are used, not the ones containing execution logic.
*
* // Note: `juniper::GraphQLTypeAsync` should not be implemented for asynchronous
* // subscriptions, as it only contains asynchronous query/mutation execution logic.
* impl GraphQLType for Subscription {
*     type Context = ();
*     type TypeInfo = ();
*
*     fn name(_: &Self::TypeInfo) -> Option<&str> {
*         Some("Subscription")
*     }
*
*     fn meta<'r>(
*         info: &Self::TypeInfo,
*         registry: &mut Registry<'r>
*     ) -> MetaType<'r>
*         where DefaultScalarValue: 'r,
*     {
*         let fields = vec![
*             registry.field_convert::<User, _, Self::Context>("users", info),
*         ];
*         let meta = registry.build_object_type::<Subscription>(info, &fields);
*         meta.into_meta()
*     }
* }
*
* // async_trait[1] is used in this example for convenience, though this trait
* // can be implemented without async_trait (subscription macros do not
* // use async_trait, for example)
* // [1](https://github.com/dtolnay/async-trait)
* #[async_trait]
* impl GraphQLSubscriptionType<DefaultScalarValue> for Subscription {
*     // This function will be called for every field by default
*     async fn resolve_field_into_stream<'args, 'e, 'res>(
*         &self,
*         info: &<Self as GraphQLType>::TypeInfo,
*         field_name: &str,
*         arguments: &juniper::Arguments<'args>,
*         executor: std::rc::Rc<juniper::Executor<'e, <Self as GraphQLType>::Context>>,
*     ) -> Result<Value<ValuesStream<'res>>, FieldError>
*     where 'args: 'res,
*           'e: 'res,
*     {
*         use futures::stream::StreamExt as _;
*         match field_name {
*             "users" => {
*                 let users_stream = futures::stream::once(async {
*                     User {
*                         id: "1".to_string(),
*                         name: "stream user".to_string(),
*                         friend_ids: vec!["2".to_string(), "3".to_string(), "4".to_string()]
*                     }
*                 });
*
*                 // Each `User` that is returned from the stream should be resolved
*                 // to filter out fields that were not requested.
*                 // This could be done by treating each returned `User` as
*                 // a separate asychronous query, which was executed up to returning `User`
*                 let stream = users_stream.then(move |res| {
*                     // check if executor's context can be replaced
*                     let res2: FieldResult<_, DefaultScalarValue> =
*                         juniper::IntoResolvable::into(res, executor.context());
*                     // clone executor here to use it in returned future
*                     let ex = executor.clone();
*                     async move {
*                         // if context could be replaced...
*                         match res2 {
*                             Ok(Some((ctx, r))) => {
*                                 let sub = ex.replaced_context(ctx);
*                                 match sub.resolve_with_ctx_async(&(), &r).await {
*                                     //... filter out not requested fields and return resolved value
*                                     Ok(v) => v,
*                                     Err(_) => Value::Null,
*                                 }
*                             }
*                             //... or return Null since value could not be resolved till the end
*                             Ok(None) => Value::Null,
*                             Err(e) => Value::Null,
*                         }
*                     }
*                 });
*                 // `Value::Scalar` is returned here because we resolved subscription
*                 // into a single stream.
*                 Ok(Value::Scalar(Box::pin(stream)))
*             },
*             _ => {
*                 // panicking here because juniper should return field does not exist
*                 // error while parsing subscription query
*                 panic!("Field {} not found on type GraphQLSubscriptionType", &field_name);
*             }
*         }
*     }
* }
*
* ```
*/
#[async_trait::async_trait]
pub trait GraphQLSubscriptionType<S>: GraphQLType<S> + Send + Sync
where
    Self::Context: Send + Sync,
    Self::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    /// Resolve the provided selection set asynchronously against
    /// the current object. This method is called by subscriptions executor
    /// and should call other trait methods if needed.
    ///
    /// ## Default implementation
    ///
    /// In order to resolve selection set on object types, default
    /// implementation calls `resolve_field_into_stream` every time a field
    /// needs to be resolved and `resolve_into_type_stream` every time a
    /// fragment needs to be resolved.
    ///
    /// For non-object types, the selection set will be `None`
    /// and default implementation will panic.
    async fn resolve_into_stream<'s, 'i, 'ss, 'ref_e, 'e, 'res>(
        &'s self,
        info: &'i Self::TypeInfo,
        selection_set: Option<&'ss [Selection<'_, S>]>,
        executor: &'ref_e Executor<'e, Self::Context, S>,
        // TODO: decide if this should be a result or not
    ) -> Value<ValuesResultStream<'res, S>>
    where
        's: 'res,
        'i: 'res,
        'ss: 'res,
        'ref_e: 'e,
        'e: 'res,
    {
        if let Some(selection_set) = selection_set {
            resolve_selection_set_into_stream(
                self,
                info,
                selection_set,
                executor
            ).await
        } else {
            panic!("resolve_into_stream() must be implemented");
        }
    }

    /// This method is called by Self's `resolve_into_stream` default
    /// implementation every time any field is found in selection set.
    ///
    /// It replaces `GraphQLType::resolve_field`.
    /// Unlike `resolve_field`, which resolves each field into a single
    /// `Value<S>`, this method resolves each field into
    /// `Value<ValuesStream<S>>`.
    ///
    /// The default implementation panics.
    async fn resolve_field_into_stream<'args, 'e, 'res>(
        &self,
        _: &Self::TypeInfo,     // this subscription's type info
        _: &str,                // field's type name
        _: Arguments<'args, S>, // field's arguments
        _: Arc<Executor<'e, Self::Context, S>>, // field's executor (subscription's sub-executor
                                // with current field's selection set)
    ) -> Result<Value<ValuesResultStream<'res, S>>, FieldError<S>>
    where
        'args: 'res,
        'e: 'res,
    {
        panic!("resolve_field must be implemented by object types");
    }

    /// It is called by Self's `resolve_into_stream` default implementation
    /// every time any fragment is found in selection set.
    ///
    /// It replaces `GraphQLType::resolve_into_type`.
    /// Unlike `resolve_into_type`, which resolves each fragment
    /// a single `Value<S>`, this method resolves each fragment into
    /// `Value<ValuesStream<S>>`.
    ///
    /// The default implementation panics.
    async fn resolve_into_type_stream<'s, 'i, 'tn, 'ss, 'e, 'res>(
        &'s self,
        _: &'i Self::TypeInfo,              // this subscription's type info
        _: &'tn str,                        // fragment's type name
        _: Option<&'ss [Selection<'_, S>]>, // fragment's arguments
        _: Arc<Executor<'e, Self::Context, S>>, // fragment's executor (subscription's sub-executor
                                            // with current field's selection set)
    ) -> Result<Value<ValuesResultStream<'res, S>>, FieldError<S>>
    where
        's: 'res,
        'i: 'res,
        'tn: 'res,
        'ss: 'res,
        'e: 'res,
    {
        // TODO: cannot resolve by default (cannot return value referencing function parameter `self`)
        // if Self::name(info).unwrap() == type_name {
        //      let stream = self.resolve_into_stream(info, selection_set, executor).await;
        //      Ok(stream)
        // } else {
        panic!("stream_resolve_into_type must be implemented by unions and interfaces");
        //        }
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
    for<'b> &'b S: ScalarRefValue<'b>,
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


struct AsyncResultField<'a, S> {
    name: String,
    value: FieldResult<Value<ValuesResultStream<'a, S>>, S>,
}

enum AsyncResultValue<'a, S> {
    Field(AsyncResultField<'a, S>),
    Nested(FieldResult<Value<ValuesResultStream<'a, S>>, S>),
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
    for<'b> &'b S: ScalarRefValue<'b>,
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

// Wrapper function around `resolve_selection_set_into_stream_recursive`.
// This wrapper is necessary because async fns can not be recursive.
#[cfg(feature = "async")]
pub(crate) fn resolve_selection_set_into_stream<'i, 'inf, 'ss, 'ref_e, 'e, 'res, T, CtxT, S>(
    instance: &'i T,
    info: &'inf T::TypeInfo,
    selection_set: &'ss [Selection<'ss, S>],
    executor: &'ref_e Executor<'e, CtxT, S>,
) -> BoxFuture<'res, FieldResult<Value<ValuesResultStream<'res, S>>, S>>
where
    'i: 'res,
    'inf: 'res,
    'ss: 'res,
    'e: 'res,
    'ref_e: 'e,
    T: GraphQLSubscriptionType<S, Context = CtxT>,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync + 'static,
    CtxT: Send + Sync,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    Box::pin(resolve_selection_set_into_stream_recursive(
        instance,
        info,
        selection_set,
        executor,
    ))
}

#[cfg(feature = "async")]
/// Selection set resolver logic
pub(crate) async fn resolve_selection_set_into_stream_recursive<'a, T, CtxT, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    selection_set: &'a [Selection<'a, S>],
    executor: &'a Executor<'a, CtxT, S>,
) -> FieldResult<Value<ValuesResultStream<'a, S>>, S>
where
    T: GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync + 'static,
    CtxT: Send + Sync,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    use futures::stream::{FuturesOrdered, StreamExt as _};

    let mut object: Object<ValuesResultStream<S>> = Object::with_capacity(selection_set.len());

    let mut async_values = FuturesOrdered::<
        BoxFuture<'a, AsyncResultValue<'a, S>>
    >::new();

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
                    let typename =
                        Value::scalar(
                            instance.concrete_type_name(
                                executor.context(),
                                info
                            )
                        );
                    object.add_field(
                        response_name,
                        Value::Scalar(
                            Box::pin(
                                futures::stream::once(async { Ok(typename) })
                            )
                        ),
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

                let sub_exec = Arc::new(executor.field_sub_executor(
                    &response_name,
                    f.name.item,
                    start_pos.clone(),
                    f.selection_set.as_ref().map(|v| &v[..]),
                ));

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
                        .resolve_field_into_stream(
                            info,
                            f.name.item,
                            args,
                            sub_exec
                        ).await;

                    let value = match res {
                        //todo: custom error type
                        Ok(Value::Null) if is_non_null => Err(FieldError::new(
                            "null value on non null field",
                            Value::null()
                            )),
                        Ok(v) => Ok(v),
                        Err(e) => Err(e),
                    };
                    AsyncResultValue::Field(AsyncResultField {
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
                    let value = resolve_selection_set_into_stream(
                        instance,
                        info,
                        &fragment.selection_set[..],
                        executor,
                    )
                    .await;
                    AsyncResultValue::Nested(value)
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

                let sub_exec = Arc::new(executor.type_sub_executor(
                    fragment.type_condition.as_ref().map(|c| c.item),
                    Some(&fragment.selection_set[..]),
                ));

                let sub_exec2 = Arc::clone(&sub_exec);

                if let Some(ref type_condition) = fragment.type_condition {
                    let sub_result = instance
                        .resolve_into_type_stream(
                            info,
                            type_condition.item,
                            Some(&fragment.selection_set[..]),
                            sub_exec,
                        )
                        .await;

                    if let Ok(Value::Object(obj)) = sub_result {
                        for (k, v) in obj {
                            merge_key_into(&mut object, &k, v);
                        }
                    } else if let Err(e) = sub_result {
                        //todo
//                        sub_exec2.push_error_at(e, start_pos.clone());
                    }
                } else {
                    if let Some(type_name) = meta_type.name() {
                        let sub_result = instance
                            .resolve_into_type_stream(
                                info,
                                type_name,
                                Some(&fragment.selection_set[..]),
                                sub_exec,
                            )
                            .await;

                        if let Ok(Value::Object(obj)) = sub_result {
                            for (k, v) in obj {
                                merge_key_into(&mut object, &k, v);
                            }
                        } else if let Err(e) = sub_result {
                            //todo
//                            sub_exec2.push_error_at(e, start_pos.clone());
                        }
                    } else {
                        return Value::Null;
                    }
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
                    return Value::Null;
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
