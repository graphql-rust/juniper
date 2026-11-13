use std::{
    future, mem,
    ops::ControlFlow,
    pin::Pin,
    task::{Context, Poll, ready},
};

use arcstr::ArcStr;
use auto_enums::enum_derive;
use futures::stream::{FuturesUnordered, StreamExt as _};

use crate::{
    ast::Selection,
    executor::{ExecutionResult, Executor},
    parser::{SourcePosition, Spanning},
    schema::meta::{Field, MetaType},
    value::{DefaultScalarValue, Object, ScalarValue, Value},
};

use crate::BoxFuture;

use super::base::{
    Arguments, GraphQLType, GraphQLValue, complete_field, is_excluded, response_key,
    typename_response_key,
};

/// Extension of [`GraphQLValue`] trait with asynchronous queries/mutations resolvers.
///
/// Convenience macros related to asynchronous queries/mutations expand into an implementation of
/// this trait and [`GraphQLValue`] for the given type.
pub trait GraphQLValueAsync<S = DefaultScalarValue>: GraphQLValue<S> + Sync
where
    Self::TypeInfo: Sync,
    Self::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    /// Resolves the value of a single field on this [`GraphQLValueAsync`].
    ///
    /// The `arguments` object contains all the specified arguments, with default values being
    /// substituted for the ones not provided by the query.
    ///
    /// The `executor` can be used to drive selections into sub-[objects][3].
    ///
    /// The returned future owns the `arguments` and the `executor`. The executor polls the future
    /// once in place, so a field that resolves at once needs no other allocation.
    ///
    /// # Panics
    ///
    /// The default implementation panics.
    ///
    /// [3]: https://spec.graphql.org/October2021#sec-Objects
    fn resolve_field_async<'a>(
        &'a self,
        _info: &'a Self::TypeInfo,
        _field_name: &'a str,
        _arguments: Arguments<'a, S>,
        _executor: Executor<'a, 'a, Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        panic!(
            "GraphQLValueAsync::resolve_field_async() must be implemented by objects and \
             interfaces",
        );
    }

    /// Resolves this [`GraphQLValueAsync`] (being an [interface][1] or an [union][2]) into a
    /// concrete downstream [object][3] type.
    ///
    /// Tries to resolve this [`GraphQLValueAsync`] into the provided `type_name`. If the type
    /// matches, then passes the instance along to [`Executor::resolve`].
    ///
    /// # Panics
    ///
    /// The default implementation panics.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    /// [2]: https://spec.graphql.org/October2021#sec-Unions
    /// [3]: https://spec.graphql.org/October2021#sec-Objects
    fn resolve_into_type_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        type_name: &str,
        selection_set: Option<&'a [Selection<'a, S>]>,
        executor: &'a Executor<'a, 'a, Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        if self.type_name(info).unwrap() == type_name {
            self.resolve_async(info, selection_set, executor)
        } else {
            panic!(
                "GraphQLValueAsync::resolve_into_type_async() must be implemented by unions and \
                 interfaces",
            );
        }
    }

    /// Resolves the provided `selection_set` against this [`GraphQLValueAsync`].
    ///
    /// For non-[object][3] types, the `selection_set` will be [`None`] and the value should simply
    /// be returned.
    ///
    /// For [objects][3], all fields in the `selection_set` should be resolved. The default
    /// implementation uses [`GraphQLValueAsync::resolve_field_async`] to resolve all fields,
    /// including those through a fragment expansion.
    ///
    /// Since the [GraphQL spec specifies][0] that errors during field processing should result in
    /// a null-value, this might return `Ok(Null)` in case of a failure. Errors are recorded
    /// internally.
    ///
    /// # Panics
    ///
    /// The default implementation panics, if `selection_set` is [`None`].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Handling-Field-Errors
    /// [3]: https://spec.graphql.org/October2021#sec-Objects
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        if let Some(sel) = selection_set {
            Box::pin(async move {
                Ok(resolve_selection_set_into_async(self, info, sel, executor).await)
            })
        } else {
            panic!(
                "GraphQLValueAsync::resolve_async() must be implemented by non-object output types",
            );
        }
    }
}

/// Extension of [`GraphQLType`] trait with asynchronous queries/mutations resolvers.
///
/// It's automatically implemented for [`GraphQLValueAsync`] and [`GraphQLType`] implementers, so
/// doesn't require manual or code-generated implementation.
pub trait GraphQLTypeAsync<S = DefaultScalarValue>: GraphQLValueAsync<S> + GraphQLType<S>
where
    Self::Context: Sync,
    Self::TypeInfo: Sync,
    S: ScalarValue + Send + Sync,
{
}

impl<S, T> GraphQLTypeAsync<S> for T
where
    T: GraphQLValueAsync<S> + GraphQLType<S> + ?Sized,
    T::Context: Sync,
    T::TypeInfo: Sync,
    S: ScalarValue + Send + Sync,
{
}

/// Boxes the future of [`resolve_selection_set_into_async()`].
///
/// An inline fragment without a type condition resolves its selection set on the same instance.
/// The future of a selection set then contains itself, so the inner future needs a box.
fn resolve_selection_set_into_async_boxed<'a, 'e, T, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    selection_set: &'e [Selection<'e, S>],
    executor: &'e Executor<'e, 'e, T::Context, S>,
) -> BoxFuture<'a, Value<S>>
where
    T: GraphQLValueAsync<S> + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
    'e: 'a,
{
    Box::pin(resolve_selection_set_into_async(
        instance,
        info,
        selection_set,
        executor,
    ))
}

/// Selection of a selection set after its resolution.
enum ResolvedSelection<S> {
    /// Value of a field.
    ///
    /// [`None`] means a non-`null` field without a value. The parent object is then `null`.
    Field {
        response_key: ArcStr,
        value: Option<Value<S>>,
    },

    /// Fields of a fragment.
    ///
    /// [`None`] means a non-`null` field of the fragment without a value. The parent object is then
    /// `null`.
    Fragment(Option<Object<S>>),
}

impl<S> ResolvedSelection<S> {
    /// Merges this selection into the `object`.
    ///
    /// Returns [`ControlFlow::Break`] if this selection makes the `object` `null`.
    fn merge_into(self, object: &mut Object<S>) -> ControlFlow<()> {
        match self {
            Self::Field {
                response_key,
                value: Some(value),
            } => object.merge_field(response_key, value),
            Self::Fragment(Some(fields)) => object.merge(fields),
            Self::Field { value: None, .. } | Self::Fragment(None) => return ControlFlow::Break(()),
        }
        ControlFlow::Continue(())
    }
}

/// Future of a pending selection in a [`PartialObject`].
#[enum_derive(Future)]
enum SelectionFuture<Field, Fragment> {
    Field(Field),
    Fragment(Fragment),
}

/// Field with a pending resolver future.
///
/// The field completes with [`complete_field()`] when its resolver future completes.
struct PendingField<'a, CtxT, S>
where
    CtxT: 'a,
    S: 'a,
{
    resolver: BoxFuture<'a, ExecutionResult<S>>,
    executor: Executor<'a, 'a, CtxT, S>,
    parent_type: &'a MetaType<S>,
    meta_field: &'a Field<S>,
    response_key: ArcStr,
    location: SourcePosition,
}

impl<CtxT, S: ScalarValue> Future for PendingField<'_, CtxT, S> {
    type Output = ResolvedSelection<S>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let result = ready!(this.resolver.as_mut().poll(cx));
        Poll::Ready(ResolvedSelection::Field {
            response_key: mem::take(&mut this.response_key),
            value: complete_field(
                result,
                &this.executor,
                this.parent_type,
                this.meta_field,
                this.location,
            ),
        })
    }
}

/// Fragment with a pending resolver future.
struct PendingFragment<'a, S> {
    future: BoxFuture<'a, Value<S>>,
}

impl<S> Future for PendingFragment<'_, S> {
    type Output = ResolvedSelection<S>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let value = ready!(this.future.as_mut().poll(cx));
        Poll::Ready(ResolvedSelection::Fragment(value.into_object()))
    }
}

/// Pending future of a selection with the position of the selection in the queue of a
/// [`PartialObject`].
struct QueuedFuture<F> {
    position: usize,
    future: F,
}

impl<F: Future + Unpin> Future for QueuedFuture<F> {
    type Output = (usize, F::Output);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let position = self.position;
        Pin::new(&mut self.future)
            .poll(cx)
            .map(|output| (position, output))
    }
}

/// Object under construction from the selections of a selection set.
///
/// The fields of the object keep the order of the selection set. A selection that resolves at once
/// merges into the object, unless an earlier selection is still pending. The selection then waits
/// in a queue behind the pending selections. Only the futures of pending selections go to a
/// [`FuturesUnordered`] set.
struct PartialObject<S, F>
where
    F: Future<Output = ResolvedSelection<S>> + Unpin,
{
    object: Object<S>,

    /// Selections from the first pending selection on, in the order of the selection set.
    ///
    /// A pending selection is [`None`] until its future completes.
    queue: Vec<Option<ResolvedSelection<S>>>,

    /// Futures of the pending selections in the `queue`.
    pending: FuturesUnordered<QueuedFuture<F>>,
}

impl<S, F> PartialObject<S, F>
where
    F: Future<Output = ResolvedSelection<S>> + Unpin,
{
    fn with_capacity(capacity: usize) -> Self {
        Self {
            object: Object::with_capacity(capacity),
            queue: Vec::new(),
            pending: FuturesUnordered::new(),
        }
    }

    /// Adds a resolved `selection`.
    ///
    /// Returns [`ControlFlow::Break`] if the `selection` makes the object `null`.
    fn push_resolved(&mut self, selection: ResolvedSelection<S>) -> ControlFlow<()> {
        if self.queue.is_empty() {
            selection.merge_into(&mut self.object)
        } else {
            self.queue.push(Some(selection));
            ControlFlow::Continue(())
        }
    }

    /// Adds the `future` of a pending selection.
    fn push_pending(&mut self, future: F) {
        self.pending.push(QueuedFuture {
            position: self.queue.len(),
            future,
        });
        self.queue.push(None);
    }

    /// Awaits the pending selections and merges the queue into the object in order.
    ///
    /// Returns [`ControlFlow::Break`] if a selection makes the object `null`.
    async fn resolve_pending(&mut self) -> ControlFlow<()> {
        while let Some((position, selection)) = self.pending.next().await {
            self.queue[position] = Some(selection);
        }
        for selection in self.queue.drain(..) {
            selection
                .expect("all pending selections are resolved")
                .merge_into(&mut self.object)?;
        }
        ControlFlow::Continue(())
    }
}

/// Polls the `future` once with the context of the current task.
///
/// Most futures of fields and list items complete on their first poll. Polling them in place
/// avoids a task in a [`FuturesUnordered`] set for each of them.
pub(crate) async fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    future::poll_fn(|cx| Poll::Ready(Pin::new(&mut *future).poll(cx))).await
}

/// Resolves the `selection_set` on the `instance` into an object.
///
/// Returns `null` if a non-`null` field has no value.
async fn resolve_selection_set_into_async<'a, T, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    selection_set: &'a [Selection<'a, S>],
    executor: &'a Executor<'a, 'a, T::Context, S>,
) -> Value<S>
where
    T: GraphQLValueAsync<S> + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    let mut object = PartialObject::with_capacity(selection_set.len());

    let meta_type = executor
        .schema()
        .concrete_type_by_name(
            instance
                .type_name(info)
                .expect("Resolving named type's selection set"),
        )
        .expect("Type not found in schema");

    for selection in selection_set {
        let flow = match *selection {
            Selection::Field(Spanning {
                item: ref f,
                ref span,
            }) => {
                if is_excluded(&f.directives, executor.variables()) {
                    continue;
                }

                if f.name.item == "__typename" {
                    object.push_resolved(ResolvedSelection::Field {
                        response_key: typename_response_key(f),
                        value: Some(Value::Scalar(
                            instance.concrete_type_name(executor.context(), info).into(),
                        )),
                    })
                } else {
                    let meta_field = meta_type.field_by_name(f.name.item).unwrap_or_else(|| {
                        panic!(
                            "Field {} not found on type {:?}",
                            f.name.item,
                            meta_type.name(),
                        )
                    });

                    let exec_vars = executor.variables();

                    let sub_exec = executor.field_sub_executor(
                        f.alias.as_ref().unwrap_or(&f.name).item,
                        meta_field,
                        span.start,
                        f.selection_set.as_ref().map(|v| &v[..]),
                    );
                    let args = Arguments::new(
                        f.arguments.as_ref().map(|m| {
                            m.item
                                .iter()
                                .filter_map(|(k, v)| {
                                    let val = v.item.clone().into_const(exec_vars)?;
                                    Some((k.item, Spanning::new(v.span, val)))
                                })
                                .collect()
                        }),
                        &meta_field.arguments,
                    );
                    let response_key = response_key(f, meta_field);
                    let location = span.start;

                    if meta_field.synchronous
                        && sub_exec
                            .current_type()
                            .innermost_concrete()
                            .is_synchronous()
                    {
                        // The field resolves without awaiting, so it needs no future.
                        let result = instance.resolve_field(info, f.name.item, &args, &sub_exec);
                        object.push_resolved(ResolvedSelection::Field {
                            response_key,
                            value: complete_field(
                                result, &sub_exec, meta_type, meta_field, location,
                            ),
                        })
                    } else {
                        let mut resolver =
                            instance.resolve_field_async(info, f.name.item, args, sub_exec);
                        match poll_once(&mut resolver).await {
                            Poll::Ready(result) => object.push_resolved(ResolvedSelection::Field {
                                response_key,
                                value: complete_field(
                                    result, &sub_exec, meta_type, meta_field, location,
                                ),
                            }),
                            Poll::Pending => {
                                object.push_pending(SelectionFuture::Field(PendingField {
                                    resolver,
                                    executor: sub_exec,
                                    parent_type: meta_type,
                                    meta_field,
                                    response_key,
                                    location,
                                }));
                                ControlFlow::Continue(())
                            }
                        }
                    }
                }
            }

            Selection::FragmentSpread(Spanning {
                item: ref spread,
                ref span,
            }) => {
                if is_excluded(&spread.directives, executor.variables()) {
                    continue;
                }

                let fragment = &executor
                    .fragment_by_name(spread.name.item)
                    .expect("Fragment could not be found");

                let sub_exec = executor.type_sub_executor(
                    Some(fragment.type_condition.item),
                    Some(&fragment.selection_set[..]),
                );

                let concrete_type_name = instance.concrete_type_name(sub_exec.context(), info);
                let type_name = instance.type_name(info);
                if executor
                    .schema()
                    .is_named_subtype(&concrete_type_name, fragment.type_condition.item)
                    || Some(fragment.type_condition.item) == type_name.as_deref()
                {
                    let fields = resolve_fragment(
                        instance,
                        info,
                        &concrete_type_name,
                        &fragment.selection_set,
                        &sub_exec,
                        span.start,
                    )
                    .await;
                    object.push_resolved(ResolvedSelection::Fragment(fields))
                } else {
                    ControlFlow::Continue(())
                }
            }

            Selection::InlineFragment(Spanning {
                item: ref fragment,
                ref span,
            }) => {
                if is_excluded(&fragment.directives, executor.variables()) {
                    continue;
                }

                let sub_exec = executor.type_sub_executor(
                    fragment.type_condition.as_ref().map(|c| c.item),
                    Some(&fragment.selection_set[..]),
                );

                if let Some(ref type_condition) = fragment.type_condition {
                    let concrete_type_name = instance.concrete_type_name(sub_exec.context(), info);
                    if executor
                        .schema()
                        .is_named_subtype(&concrete_type_name, type_condition.item)
                    {
                        let fields = resolve_fragment(
                            instance,
                            info,
                            &concrete_type_name,
                            &fragment.selection_set,
                            &sub_exec,
                            span.start,
                        )
                        .await;
                        object.push_resolved(ResolvedSelection::Fragment(fields))
                    } else {
                        ControlFlow::Continue(())
                    }
                } else {
                    let mut pending_fragment = PendingFragment {
                        future: resolve_selection_set_into_async_boxed(
                            instance,
                            info,
                            &fragment.selection_set,
                            &sub_exec,
                        ),
                    };
                    match poll_once(&mut pending_fragment).await {
                        Poll::Ready(selection) => object.push_resolved(selection),
                        Poll::Pending => {
                            object.push_pending(SelectionFuture::Fragment(pending_fragment));
                            ControlFlow::Continue(())
                        }
                    }
                }
            }
        };

        if flow.is_break()
            // In serial execution, each selection completes before the next selection starts.
            || (executor.is_serial() && object.resolve_pending().await.is_break())
        {
            return Value::null();
        }
    }

    if object.resolve_pending().await.is_break() {
        return Value::null();
    }
    Value::Object(object.object)
}

/// Resolves the `selection_set` of a fragment for the `concrete_type_name` of the `instance`.
///
/// Pushes the error of a failed resolution at the `location` to the `executor`.
///
/// Returns [`None`] if the fragment makes the parent object `null`.
async fn resolve_fragment<'a, T, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    concrete_type_name: &str,
    selection_set: &'a [Selection<'a, S>],
    executor: &'a Executor<'a, 'a, T::Context, S>,
    location: SourcePosition,
) -> Option<Object<S>>
where
    T: GraphQLValueAsync<S> + ?Sized,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    match instance
        .resolve_into_type_async(info, concrete_type_name, Some(selection_set), executor)
        .await
    {
        Ok(value) => value.into_object(),
        Err(error) => {
            executor.push_error_at(error, location);
            None
        }
    }
}