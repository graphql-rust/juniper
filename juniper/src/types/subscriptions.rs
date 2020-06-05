use crate::{
    http::{GraphQLRequest, GraphQLResponse},
    parser::Spanning,
    types::base::{is_excluded, merge_key_into},
    Arguments, BoxFuture, Executor, FieldError, GraphQLType, Object, ScalarValue, Selection, Value,
    ValuesStream,
};

/// Global subscription coordinator trait.
///
/// With regular queries we could get away with not having some in-between
/// layer, but for subscriptions it is needed, otherwise the integration crates
/// can become really messy and cumbersome to maintain. Subscriptions are also
/// quite a bit more stability sensitive than regular queries, they provide a
/// great vector for DOS attacks and can bring down a server easily if not
/// handled right.
///
/// This trait implementation might include the following features:
///  - contains the schema
///  - keeps track of subscription connections
///  - handles subscription start, maintains a global subscription id
///  - max subscription limits / concurrency limits
///  - subscription de-duplication
///  - reconnection on connection loss / buffering / re-synchronisation
///
///
/// `'a` is how long spawned connections live for.
pub trait SubscriptionCoordinator<'a, CtxT, S>
where
    S: ScalarValue,
{
    /// Type of [`SubscriptionConnection`]s this [`SubscriptionCoordinator`]
    /// returns
    type Connection: SubscriptionConnection<'a, S>;

    /// Type of error while trying to spawn [`SubscriptionConnection`]
    type Error;

    /// Return [`SubscriptionConnection`] based on given [`GraphQLRequest`]
    fn subscribe(
        &'a self,
        _: &'a GraphQLRequest<S>,
        _: &'a CtxT,
    ) -> BoxFuture<'a, Result<Self::Connection, Self::Error>>;
}

/// Single subscription connection.
///
/// This trait implementation might:
/// - hold schema + context
/// - process subscribe, unsubscribe
/// - unregister from coordinator upon close/shutdown
/// - connection-local + global de-duplication, talk to coordinator
/// - concurrency limits
/// - machinery with coordinator to allow reconnection
///
/// It can be treated as [`futures::Stream`] yielding [`GraphQLResponse`]s in
/// server integration crates.
pub trait SubscriptionConnection<'a, S>: futures::Stream<Item = GraphQLResponse<'a, S>> {}

/**
 This trait adds resolver logic with asynchronous subscription execution logic
 on GraphQL types. It should be used with `GraphQLType` in order to implement
 subscription resolvers on GraphQL objects.

 Subscription-related convenience macros expand into an implementation of this
 trait and `GraphQLType` for the given type.

 See trait methods for more detailed explanation on how this trait works.
*/
pub trait GraphQLSubscriptionType<S>: GraphQLType<S> + Send + Sync
where
    Self::Context: Send + Sync,
    Self::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
{
    /// Resolve into `Value<ValuesStream>`
    ///
    /// ## Default implementation
    ///
    /// In order to resolve selection set on object types, default
    /// implementation calls `resolve_field_into_stream` every time a field
    /// needs to be resolved and `resolve_into_type_stream` every time a
    /// fragment needs to be resolved.
    ///
    /// For non-object types, the selection set will be `None` and default
    /// implementation will panic.
    fn resolve_into_stream<'s, 'i, 'ref_e, 'e, 'res, 'f>(
        &'s self,
        info: &'i Self::TypeInfo,
        executor: &'ref_e Executor<'ref_e, 'e, Self::Context, S>,
    ) -> BoxFuture<'f, Result<Value<ValuesStream<'res, S>>, FieldError<S>>>
    where
        'e: 'res,
        'i: 'res,
        's: 'f,
        'ref_e: 'f,
        'res: 'f,
    {
        if executor.current_selection_set().is_some() {
            Box::pin(
                async move { Ok(resolve_selection_set_into_stream(self, info, executor).await) },
            )
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
    fn resolve_field_into_stream<'s, 'i, 'ft, 'args, 'e, 'ref_e, 'res, 'f>(
        &'s self,
        _: &'i Self::TypeInfo,  // this subscription's type info
        _: &'ft str,            // field's type name
        _: Arguments<'args, S>, // field's arguments
        _: &'ref_e Executor<'ref_e, 'e, Self::Context, S>, // field's executor (subscription's sub-executor
                                                           // with current field's selection set)
    ) -> BoxFuture<'f, Result<Value<ValuesStream<'res, S>>, FieldError<S>>>
    where
        's: 'f,
        'i: 'res,
        'ft: 'f,
        'args: 'f,
        'ref_e: 'f,
        'res: 'f,
        'e: 'res,
    {
        panic!("resolve_field_into_stream must be implemented");
    }

    /// This method is called by Self's `resolve_into_stream` default
    /// implementation every time any fragment is found in selection set.
    ///
    /// It replaces `GraphQLType::resolve_into_type`.
    /// Unlike `resolve_into_type`, which resolves each fragment
    /// a single `Value<S>`, this method resolves each fragment into
    /// `Value<ValuesStream<S>>`.
    ///
    /// The default implementation panics.
    fn resolve_into_type_stream<'s, 'i, 'tn, 'e, 'ref_e, 'res, 'f>(
        &'s self,
        info: &'i Self::TypeInfo, // this subscription's type info
        type_name: &'tn str,      // fragment's type name
        executor: &'ref_e Executor<'ref_e, 'e, Self::Context, S>, // fragment's executor (subscription's sub-executor
                                                                  // with current field's selection set)
    ) -> BoxFuture<'f, Result<Value<ValuesStream<'res, S>>, FieldError<S>>>
    where
        'i: 'res,
        'e: 'res,
        's: 'f,
        'tn: 'f,
        'ref_e: 'f,
        'res: 'f,
    {
        Box::pin(async move {
            if Self::name(info) == Some(type_name) {
                self.resolve_into_stream(info, executor).await
            } else {
                panic!("resolve_into_type_stream must be implemented");
            }
        })
    }
}

/// Wrapper function around `resolve_selection_set_into_stream_recursive`.
/// This wrapper is necessary because async fns can not be recursive.
/// Panics if executor's current selection set is None.
pub(crate) fn resolve_selection_set_into_stream<'i, 'inf, 'ref_e, 'e, 'res, 'fut, T, CtxT, S>(
    instance: &'i T,
    info: &'inf T::TypeInfo,
    executor: &'ref_e Executor<'ref_e, 'e, CtxT, S>,
) -> BoxFuture<'fut, Value<ValuesStream<'res, S>>>
where
    'inf: 'res,
    'e: 'res,
    'i: 'fut,
    'e: 'fut,
    'ref_e: 'fut,
    'res: 'fut,
    T: GraphQLSubscriptionType<S, Context = CtxT> + ?Sized,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
    CtxT: Send + Sync,
{
    Box::pin(resolve_selection_set_into_stream_recursive(
        instance, info, executor,
    ))
}

/// Selection set default resolver logic.
/// Returns `Value::Null` if cannot keep resolving. Otherwise pushes errors to
/// `Executor`.
async fn resolve_selection_set_into_stream_recursive<'i, 'inf, 'ref_e, 'e, 'res, T, CtxT, S>(
    instance: &'i T,
    info: &'inf T::TypeInfo,
    executor: &'ref_e Executor<'ref_e, 'e, CtxT, S>,
) -> Value<ValuesStream<'res, S>>
where
    T: GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync + ?Sized,
    T::TypeInfo: Send + Sync,
    S: ScalarValue + Send + Sync,
    CtxT: Send + Sync,
    'inf: 'res,
    'e: 'res,
{
    let selection_set = executor
        .current_selection_set()
        .expect("Executor's selection set is none");

    let mut object: Object<ValuesStream<'res, S>> = Object::with_capacity(selection_set.len());
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
                } else if let Some(type_name) = meta_type.name() {
                    let sub_result = instance
                        .resolve_into_type_stream(info, type_name, &sub_exec)
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

    Value::Object(object)
}
