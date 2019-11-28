use crate::parser::Spanning;
use crate::types::base::{is_excluded, merge_key_into};
use crate::Arguments;
use crate::{
    BoxFuture, Executor, FieldError, GraphQLType, Object,
    ScalarRefValue, ScalarValue, Selection, Value, ValuesResultStream,
};

// TODO#433: update this after `async-await` will be refactored
/**

 This trait replaces GraphQLType`'s resolver logic with asynchronous subscription
 execution logic. It should be used with `GraphQLType` in order to implement
 subscription GraphQL objects.

 Asynchronous subscription related convenience macros expand into an
 implementation of this trait and `GraphQLType` for the given type.

 See trait methods for more detailed explanation on how this trait works.

 ## Manual implementation example

 The following example demonstrates how to implement `GraphQLSubscriptionType`
 with default resolver logic (without overwriting `resolve_into_stream`) manually.

 Juniper's subscription macros do not overwrite `resolve_into_stream`


 ```rust
 use async_trait::async_trait;
 use juniper::{
     GraphQLType, GraphQLSubscriptionType, Value, ValuesResultStream,
     FieldError, Registry, meta::MetaType, DefaultScalarValue,
     FieldResult, Executor,
 };

 #[derive(Debug)]
 struct User { id: String, name: String, friend_ids: Vec<String> }

 #[juniper::object]
 impl User {}

 struct Subscription;

 // `GraphQLType` should be implemented in order to use this type in `juniper::RootNode`.
 // In this example it is implemented manually to show that only `name` and `meta` methods
 // are used, not the ones containing execution logic.

 // Note: `juniper::GraphQLTypeAsync` should not be implemented for asynchronous
 // subscriptions, as it only contains asynchronous query/mutation execution logic.
 impl GraphQLType for Subscription {
     type Context = ();
     type TypeInfo = ();

     fn name(_: &Self::TypeInfo) -> Option<&str> {
         Some("Subscription")
     }

     fn meta<'r>(
         info: &Self::TypeInfo,
         registry: &mut Registry<'r>
     ) -> MetaType<'r>
         where DefaultScalarValue: 'r,
     {
         let fields = vec![
             registry.field_convert::<User, _, Self::Context>("users", info),
         ];
         let meta = registry.build_object_type::<Subscription>(info, &fields);
         meta.into_meta()
     }
 }

 // async_trait[1] is used in this example for convenience, though this trait
 // can be implemented without async_trait (subscription macros do not
 // use async_trait, for example)
 // [1](https://github.com/dtolnay/async-trait)
 #[async_trait]
 impl GraphQLSubscriptionType<DefaultScalarValue> for Subscription {
     // This function will be called for every field by default
     async fn resolve_field_into_stream<'args, 'e, 'ref_e, 'res>(
         &self,
         info: &<Self as GraphQLType>::TypeInfo,
         field_name: &str,
         arguments: &juniper::Arguments<'args>,
         executor: &'ref_e Executor<'ref_e, 'e, Self::Context, S>,
     ) -> Result<Value<ValuesResultStream<'res>>, FieldError>
     where 'e: 'res,
     {
         use futures::stream::StreamExt as _;
         match field_name {
             "users" => {
                 let users_stream = futures::stream::once(async {
                     User {
                         id: "1".to_string(),
                         name: "stream user".to_string(),
                         friend_ids: vec!["2".to_string(), "3".to_string(), "4".to_string()]
                     }
                 });

                 // Each `User` that is returned from the stream should be
                 // resolved in order to to filter out fields that were not requested.
                 // This could be done by treating each returned `User` as
                 // a separate asychronous query, which was executed up to returning `User`

                 // Executor should be converted to OwnedExecutor since this stream
                 // will be returned
                 let executor= executor.as_owned_executor();

                 let f = res.then(|res| {
                    // OwnedExecutor should be cloned here since this closure is FnMut
                    let exec = executor.clone();

                    // Check if returned object can be replaced by `exec`'s context...
                    let res2: juniper::FieldResult<_, juniper::DefaultScalarValue> =
                        juniper::IntoResolvable::into(res, exec.context());

                    async move {
                        let ex = exec.as_executor();
                        match res2 {
                            // ...if managed to replace then resolve
                            //    and return resolved item
                            Ok(Some((ctx, r))) => {
                                let sub = ex.replaced_context(ctx);
                                sub.resolve_with_ctx_async(&(), &r).await
                            }
                            Ok(None) => Ok(Value::null()),
                            // ...otherwise return the error
                            Err(e) => Err(e),
                        }
                    }
                });

                // Return Value::Scalar since we have only one stream
                Ok(Value::Scalar::<ValuesResultStream>(
                    Box::pin(f),
                ))
             },
             _ => {
                 // panicking here because should be safe since juniper returns
                 // 'field does not exist' error while parsing subscription query
                 // if such field does not exist on type
                 panic!("Field {} not found on type GraphQLSubscriptionType", &field_name);
             }
         }
     }
 }
 ```
*/
#[async_trait::async_trait]
pub trait GraphQLSubscriptionType<S>: GraphQLType<S> + Send + Sync
where
    Self::Context: Send + Sync,
    Self::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    /// In order to resolve selection set on object types, default
    /// implementation calls `resolve_field_into_stream` every time a field
    /// needs to be resolved and `resolve_into_type_stream` every time a
    /// fragment needs to be resolved.
    ///
    /// For non-object types, the selection set will be `None`
    /// and default implementation will panic.
    async fn resolve_into_stream<'s, 'i, 'ref_e, 'e, 'res>(
        &'s self,
        info: &'i Self::TypeInfo,
        executor: &'ref_e Executor<'ref_e, 'e, Self::Context, S>,
    ) -> Value<ValuesResultStream<'res, S>>
    where
        'i: 'res,
        'e: 'res,
    {
        if executor.current_selection_set().is_some() {
            resolve_selection_set_into_stream(self, info, executor).await
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
    async fn resolve_field_into_stream<'args, 'e, 'ref_e, 'res>(
        &self,
        _: &Self::TypeInfo,     // this subscription's type info
        _: &str,                // field's type name
        _: Arguments<'args, S>, // field's arguments
        _: &'ref_e Executor<'ref_e, 'e, Self::Context, S>, // field's executor (subscription's sub-executor
                                                           // with current field's selection set)
    ) -> Result<Value<ValuesResultStream<'res, S>>, FieldError<S>>
    where
        'e: 'res,
    {
        panic!("resolve_field_into_stream must be implemented");
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
    async fn resolve_into_type_stream<'s, 'i, 'tn, 'e, 'ref_e, 'res>(
        &'s self,
        info: &'i Self::TypeInfo, // this subscription's type info
        type_name: &'tn str,      // fragment's type name
        executor: &'ref_e Executor<'ref_e, 'e, Self::Context, S>, // fragment's executor (subscription's sub-executor
                                                                  // with current field's selection set)
    ) -> Result<Value<ValuesResultStream<'res, S>>, FieldError<S>>
    where
        'i: 'res,
        'e: 'res,
    {
        if Self::name(info) == Some(type_name) {
            Ok(self.resolve_into_stream(info, executor).await)
        } else {
            panic!("resolve_into_type_stream must be implemented");
        }
    }
}

// Wrapper function around `resolve_selection_set_into_stream_recursive`.
// This wrapper is necessary because async fns can not be recursive.
// Panics if executor's current selection set is None
#[cfg(feature = "async")]
fn resolve_selection_set_into_stream<'i, 'inf, 'ref_e, 'e, 'res, 'fut, T, CtxT, S>(
    instance: &'i T,
    info: &'inf T::TypeInfo,
    executor: &'ref_e Executor<'ref_e, 'e, CtxT, S>,
) -> BoxFuture<'fut, Value<ValuesResultStream<'res, S>>>
where
    'inf: 'res,
    'e: 'res,
    'i: 'fut,
    'e: 'fut,
    'ref_e: 'fut,
    'res: 'fut,
    T: GraphQLSubscriptionType<S, Context = CtxT>,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync + 'static,
    CtxT: Send + Sync,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    Box::pin(resolve_selection_set_into_stream_recursive(
        instance, info, executor,
    ))
}

#[cfg(feature = "async")]
/// Selection set default resolver logic.
/// Returns `Value::Null` if cannot keep resolving. Otherwise pushes
/// errors to `Executor`.
async fn resolve_selection_set_into_stream_recursive<
    'i,
    'inf,
    'ref_e,
    'e,
    'res,
    T,
    CtxT,
    S,
>(
    instance: &'i T,
    info: &'inf T::TypeInfo,
    executor: &'ref_e Executor<'ref_e, 'e, CtxT, S>,
) -> Value<ValuesResultStream<'res, S>>
where
    T: GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync + 'static,
    CtxT: Send + Sync,
    for<'b> &'b S: ScalarRefValue<'b>,
    'inf: 'res,
    'e: 'res,
{
    let selection_set = executor
        .current_selection_set()
        .expect("Executor's selection set is none");

    let mut object: Object<ValuesResultStream<'res, S>> =
        Object::with_capacity(selection_set.len());
    let meta_type = executor
        .schema()
        .concrete_type_by_name(
            T::name(info)
                .expect("Resolving named type's selection set")
                .as_ref(),
        )
        .expect("Type not found in schema");

    for selection in selection_set {
        match selection {
            Selection::Field(Spanning {
                item: ref f,
                start: ref start_pos,
                ..
            }) => {
                if is_excluded(&f.directives, &executor.variables()) {
                    continue;
                }

                let response_name = f.alias.as_ref().unwrap_or(&f.name).item;

                if f.name.item == "__typename" {
                    let typename =
                        Value::scalar(instance.concrete_type_name(executor.context(), info));
                    object.add_field(
                        response_name,
                        Value::Scalar(Box::pin(futures::stream::once(async { Ok(typename) }))),
                    );
                    continue;
                }

                let meta_field = meta_type
                    .field_by_name(f.name.item)
                    .unwrap_or_else(|| {
                        panic!(format!(
                            "Field {} not found on type {:?}",
                            f.name.item,
                            meta_type.name()
                        ))
                    })
                    .clone();

                let exec_vars = executor.variables();

                let sub_exec = executor.field_sub_executor(
                    response_name,
                    f.name.item,
                    start_pos.clone(),
                    f.selection_set.as_ref().map(|x| &x[..]),
                );

                let args = Arguments::new(
                    f.arguments.as_ref().map(|m| {
                        m.item
                            .iter()
                            .map(|&(ref k, ref v)| (k.item, v.item.clone().into_const(&exec_vars)))
                            .collect()
                    }),
                    &meta_field.arguments,
                );

                let is_non_null = meta_field.field_type.is_non_null();

                let res = instance
                    .resolve_field_into_stream(info, f.name.item, args, &sub_exec)
                    .await;

                match res {
                    Ok(Value::Null) if is_non_null => {
                        return Value::Null;
                    }
                    Ok(v) => merge_key_into(&mut object, response_name, v),
                    Err(e) => {
                        sub_exec.push_error_at(e, start_pos.clone());

                        if meta_field.field_type.is_non_null() {
                            return Value::Null;
                        }

                        object.add_field(f.name.item, Value::Null);
                    }
                }
            }

            Selection::FragmentSpread(Spanning {
                item: ref spread,
                start: ref start_pos,
                ..
            }) => {
                if is_excluded(&spread.directives, &executor.variables()) {
                    continue;
                }

                let fragment = executor
                    .fragment_by_name(spread.name.item)
                    .expect("Fragment could not be found");

                let sub_exec = executor.type_sub_executor(
                    Some(fragment.type_condition.item),
                    Some(&fragment.selection_set[..]),
                );

                let obj = instance
                    .resolve_into_type_stream(info, fragment.type_condition.item, &sub_exec)
                    .await;

                match obj {
                    Ok(val) => {
                        match val {
                            Value::Object(o) => {
                                for (k, v) in o {
                                    merge_key_into(&mut object, &k, v);
                                }
                            }
                            // since this was a wrapper of current function,
                            // we'll rather get an object or nothing
                            _ => unreachable!(),
                        }
                    }
                    Err(e) => sub_exec.push_error_at(e, start_pos.clone()),
                }
            }
            Selection::InlineFragment(Spanning {
                item: ref fragment,
                start: ref start_pos,
                ..
            }) => {
                if is_excluded(&fragment.directives, &executor.variables()) {
                    continue;
                }

                let sub_exec = executor.type_sub_executor(
                    fragment.type_condition.as_ref().map(|c| c.item),
                    Some(&fragment.selection_set[..]),
                );

                if let Some(ref type_condition) = fragment.type_condition {
                    let sub_result = instance
                        .resolve_into_type_stream(info, type_condition.item, &sub_exec)
                        .await;

                    if let Ok(Value::Object(obj)) = sub_result {
                        for (k, v) in obj {
                            merge_key_into(&mut object, &k, v);
                        }
                    } else if let Err(e) = sub_result {
                        sub_exec.push_error_at(e, start_pos.clone());
                    }
                } else {
                    if let Some(type_name) = meta_type.name() {
                        let sub_result = instance
                            .resolve_into_type_stream(info, type_name.clone(), &sub_exec)
                            .await;

                        if let Ok(Value::Object(obj)) = sub_result {
                            for (k, v) in obj {
                                merge_key_into(&mut object, &k, v);
                            }
                        } else if let Err(e) = sub_result {
                            sub_exec.push_error_at(e, start_pos.clone());
                        }
                    } else {
                        return Value::Null;
                    }
                }
            }
        }
    }

    Value::Object(object)
}
