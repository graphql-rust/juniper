use futures::{future, stream};
use serde::Serialize;

use crate::{
    http::GraphQLRequest,
    parser::Spanning,
    types::base::{is_excluded, merge_key_into, GraphQLType, GraphQLValue},
    Arguments, BoxFuture, ExecutionError, Executor, FieldError, Object, Selection, Value,
    ValuesStream,
};

/// Represents the result of executing a GraphQL operation (after parsing and validating has been
/// done).
#[derive(Debug, Serialize)]
pub struct ExecutionOutput {
    /// The output data.
    pub data: Value,

    /// The errors that occurred. Note that the presence of errors does not mean there is no data.
    /// The output can have both data and errors.
    pub errors: Vec<ExecutionError>,
}

impl ExecutionOutput {
    /// Creates execution output from data, with no errors.
    pub fn from_data(data: Value) -> Self {
        Self {
            data,
            errors: vec![],
        }
    }
}

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
pub trait SubscriptionCoordinator<'a, CtxT> {
    /// Type of [`SubscriptionConnection`]s this [`SubscriptionCoordinator`]
    /// returns
    type Connection: SubscriptionConnection;

    /// Type of error while trying to spawn [`SubscriptionConnection`]
    type Error;

    /// Return [`SubscriptionConnection`] based on given [`GraphQLRequest`]
    fn subscribe(
        &'a self,
        _: &'a GraphQLRequest,
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
pub trait SubscriptionConnection: futures::Stream<Item = ExecutionOutput> {}

/// Extension of [`GraphQLValue`] trait with asynchronous [subscription][1] execution logic.
/// It should be used with [`GraphQLValue`] in order to implement [subscription][1] resolvers on
/// [GraphQL objects][2].
///
/// [Subscription][1]-related convenience macros expand into an implementation of this trait and
/// [`GraphQLValue`] for the given type.
///
/// See trait methods for more detailed explanation on how this trait works.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Subscription
/// [2]: https://spec.graphql.org/June2018/#sec-Objects
pub trait GraphQLSubscriptionValue: GraphQLValue + Sync
where
    Self::TypeInfo: Sync,
    Self::Context: Sync,
{
    /// Resolves into `Value<ValuesStream>`.
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
        executor: &'ref_e Executor<'ref_e, 'e, Self::Context>,
    ) -> BoxFuture<'f, Result<Value<ValuesStream<'res>>, FieldError>>
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
    /// It replaces `GraphQLValue::resolve_field`.
    /// Unlike `resolve_field`, which resolves each field into a single
    /// `Value`, this method resolves each field into
    /// `Value<ValuesStream>`.
    ///
    /// The default implementation panics.
    fn resolve_field_into_stream<'s, 'i, 'ft, 'args, 'e, 'ref_e, 'res, 'f>(
        &'s self,
        _: &'i Self::TypeInfo, // this subscription's type info
        _: &'ft str,           // field's type name
        _: Arguments<'args>,   // field's arguments
        _: &'ref_e Executor<'ref_e, 'e, Self::Context>, // field's executor (subscription's sub-executor
                                                        // with current field's selection set)
    ) -> BoxFuture<'f, Result<Value<ValuesStream<'res>>, FieldError>>
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
    /// It replaces `GraphQLValue::resolve_into_type`.
    /// Unlike `resolve_into_type`, which resolves each fragment
    /// a single `Value`, this method resolves each fragment into
    /// `Value<ValuesStream>`.
    ///
    /// The default implementation panics.
    fn resolve_into_type_stream<'s, 'i, 'tn, 'e, 'ref_e, 'res, 'f>(
        &'s self,
        info: &'i Self::TypeInfo, // this subscription's type info
        type_name: &'tn str,      // fragment's type name
        executor: &'ref_e Executor<'ref_e, 'e, Self::Context>, // fragment's executor (subscription's sub-executor
                                                               // with current field's selection set)
    ) -> BoxFuture<'f, Result<Value<ValuesStream<'res>>, FieldError>>
    where
        'i: 'res,
        'e: 'res,
        's: 'f,
        'tn: 'f,
        'ref_e: 'f,
        'res: 'f,
    {
        Box::pin(async move {
            if self.type_name(info) == Some(type_name) {
                self.resolve_into_stream(info, executor).await
            } else {
                panic!("resolve_into_type_stream must be implemented");
            }
        })
    }
}

crate::sa::assert_obj_safe!(GraphQLSubscriptionValue<Context = (), TypeInfo = ()>);

/// Extension of [`GraphQLType`] trait with asynchronous [subscription][1] execution logic.
///
/// It's automatically implemented for [`GraphQLSubscriptionValue`] and [`GraphQLType`]
/// implementers, so doesn't require manual or code-generated implementation.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Subscription
pub trait GraphQLSubscriptionType: GraphQLSubscriptionValue + GraphQLType
where
    Self::Context: Sync,
    Self::TypeInfo: Sync,
{
}

impl<T> GraphQLSubscriptionType for T
where
    T: GraphQLSubscriptionValue + GraphQLType + ?Sized,
    T::Context: Sync,
    T::TypeInfo: Sync,
{
}

/// Wrapper function around `resolve_selection_set_into_stream_recursive`.
/// This wrapper is necessary because async fns can not be recursive.
/// Panics if executor's current selection set is None.
pub(crate) fn resolve_selection_set_into_stream<'i, 'inf, 'ref_e, 'e, 'res, 'fut, T>(
    instance: &'i T,
    info: &'inf T::TypeInfo,
    executor: &'ref_e Executor<'ref_e, 'e, T::Context>,
) -> BoxFuture<'fut, Value<ValuesStream<'res>>>
where
    'inf: 'res,
    'e: 'res,
    'i: 'fut,
    'e: 'fut,
    'ref_e: 'fut,
    'res: 'fut,
    T: GraphQLSubscriptionValue + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
{
    Box::pin(resolve_selection_set_into_stream_recursive(
        instance, info, executor,
    ))
}

/// Selection set default resolver logic.
/// Returns `Value::Null` if cannot keep resolving. Otherwise pushes errors to
/// `Executor`.
async fn resolve_selection_set_into_stream_recursive<'i, 'inf, 'ref_e, 'e, 'res, T>(
    instance: &'i T,
    info: &'inf T::TypeInfo,
    executor: &'ref_e Executor<'ref_e, 'e, T::Context>,
) -> Value<ValuesStream<'res>>
where
    T: GraphQLSubscriptionValue + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
    'inf: 'res,
    'e: 'res,
{
    let selection_set = executor
        .current_selection_set()
        .expect("Executor's selection set is none");

    let mut object: Object<ValuesStream<'res>> = Object::with_capacity(selection_set.len());
    let meta_type = executor
        .schema()
        .concrete_type_by_name(
            instance
                .type_name(info)
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
                        Value::Scalar(Box::pin(stream::once(future::ok(typename)))),
                    );
                    continue;
                }

                let meta_field = meta_type
                    .field_by_name(f.name.item)
                    .unwrap_or_else(|| {
                        panic!(
                            "Field {} not found on type {:?}",
                            f.name.item,
                            meta_type.name(),
                        )
                    })
                    .clone();

                let exec_vars = executor.variables();

                let sub_exec = executor.field_sub_executor(
                    response_name,
                    f.name.item,
                    *start_pos,
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
                        sub_exec.push_error_at(e, *start_pos);

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
                    Err(e) => sub_exec.push_error_at(e, *start_pos),
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
                        sub_exec.push_error_at(e, *start_pos);
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
                        sub_exec.push_error_at(e, *start_pos);
                    }
                } else {
                    return Value::Null;
                }
            }
        }
    }

    Value::Object(object)
}
