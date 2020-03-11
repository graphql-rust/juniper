//! This crate supplies [`SubscriptionCoordinator`] and [`SubscriptionConnection`] implementations
//! for the [juniper](https://github.com/graphql-rust/juniper) crate.
//!
//! You need both this and `juniper` crate.

#![deny(missing_docs)]
#![deny(warnings)]
#![doc(html_root_url = "https://docs.rs/juniper_subscriptions/0.14.2")]

use std::{iter::FromIterator, pin::Pin};

use juniper::{
    http::GraphQLRequest, BoxFuture, ExecutionError, GraphQLError, GraphQLSubscriptionType,
    GraphQLTypeAsync, Object, ScalarValue, SubscriptionConnection, SubscriptionCoordinator, Value,
    ValuesStream,
};

use futures::{task::Poll, Stream};
use juniper::http::GraphQLResponse;

/// [`SubscriptionCoordinator`] implementation
pub struct Coordinator<'a, QueryT, MutationT, SubscriptionT, CtxT, S>
where
    S: ScalarValue + Send + Sync + 'static,
    QueryT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    QueryT::TypeInfo: Send + Sync,
    MutationT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Send + Sync,
{
    root_node: juniper::RootNode<'a, QueryT, MutationT, SubscriptionT, S>,
}

impl<'a, QueryT, MutationT, SubscriptionT, CtxT, S>
    Coordinator<'a, QueryT, MutationT, SubscriptionT, CtxT, S>
where
    S: ScalarValue + Send + Sync + 'static,
    QueryT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    QueryT::TypeInfo: Send + Sync,
    MutationT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Send + Sync,
{
    /// Builds new [`Coordinator`] with specified `root_node`
    pub fn new(root_node: juniper::RootNode<'a, QueryT, MutationT, SubscriptionT, S>) -> Self {
        Self { root_node }
    }
}

impl<'a, QueryT, MutationT, SubscriptionT, CtxT, S> SubscriptionCoordinator<'a, CtxT, S>
    for Coordinator<'a, QueryT, MutationT, SubscriptionT, CtxT, S>
where
    S: ScalarValue + Send + Sync + 'a,
    QueryT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    QueryT::TypeInfo: Send + Sync,
    MutationT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Send + Sync,
{
    type Connection = Connection<'a, S>;

    fn subscribe(
        &'a self,
        req: &'a GraphQLRequest<S>,
        context: &'a CtxT,
    ) -> BoxFuture<'a, Result<Self::Connection, GraphQLError<'a>>> {
        let rn = &self.root_node;

        Box::pin(async move {
            let (stream, errors) = juniper::http::resolve_into_stream(req, rn, context).await?;

            Ok(Connection::from_stream(stream, errors))
        })
    }
}

/// Connection implementing [`SubscriptionConnection`].
pub struct Connection<'a, S> {
    stream: Pin<Box<dyn futures::Stream<Item = GraphQLResponse<'a, S>> + Send + 'a>>,
}

impl<'a, S> Connection<'a, S>
where
    S: ScalarValue + Send + Sync + 'a,
{
    /// Creates new [`Connection`] from values stream and errors
    pub fn from_stream(stream: Value<ValuesStream<'a, S>>, errors: Vec<ExecutionError<S>>) -> Self {
        Self {
            stream: whole_responses_stream(stream, errors),
        }
    }
}

//todo: test
/// Creates [`futures::Stream`] that yields [`GraphQLResponse`]s depending on the given [`Value`]:
///
/// [`Value::Null`] - returns [`Value::Null`] once
/// [`Value::Scalar`] - returns `Ok` value or [`Value::Null`] and errors vector
/// [`Value::List`] - resolves each stream from the list using current logic and returns
///                   values in the order received
/// [`Value::Object`] - waits while each field of the [`Object`] is returned, then yields the whole object
/// `Value::Object<Value::Object<_>>` - returns error if [`Value::Object`] consists of sub-objects
fn whole_responses_stream<'a, S>(
    stream: Value<ValuesStream<'a, S>>,
    errors: Vec<ExecutionError<S>>,
) -> Pin<Box<dyn futures::Stream<Item = GraphQLResponse<'a, S>> + Send + 'a>>
where
    S: ScalarValue + Send + Sync + 'a,
{
    use futures::stream::{self, StreamExt as _};

    if !errors.is_empty() {
        return Box::pin(stream::once(async move {
            GraphQLResponse::from_result(Ok((Value::Null, errors)))
        }));
    }

    match stream {
        Value::Null => Box::pin(stream::once(async move {
            GraphQLResponse::from_result(Ok((Value::Null, vec![])))
        })),
        Value::Scalar(s) => Box::pin(s.map(|res| match res {
            Ok(val) => GraphQLResponse::from_result(Ok((val, vec![]))),
            Err(err) => GraphQLResponse::from_result(Ok((Value::Null, vec![err]))),
        })),
        Value::List(list) => {
            let mut streams = vec![];
            for s in list.into_iter() {
                streams.push(whole_responses_stream(s, vec![]));
            }
            Box::pin(stream::select_all(streams))
        }
        Value::Object(obj) => {
            let obj_len = obj.field_count();
            let mut key_values = obj.into_key_value_list();
            if obj_len == 0 {
                return Box::pin(stream::once(async move {
                    GraphQLResponse::from_result(Ok((Value::Null, vec![])))
                }));
            }

            let mut filled_count = 0;
            let mut ready_vector = Vec::with_capacity(obj_len);
            for _ in 0..obj_len {
                ready_vector.push(None);
            }

            let stream = futures::stream::poll_fn(
                move |mut ctx| -> Poll<Option<GraphQLResponse<'static, S>>> {
                    for i in 0..ready_vector.len() {
                        let val = &mut ready_vector[i];
                        if val.is_none() {
                            let (field_name, ref mut stream_val) = &mut key_values[i];

                            match stream_val {
                                Value::Scalar(stream) => {
                                    match Pin::new(stream).poll_next(&mut ctx) {
                                        Poll::Ready(None) => {
                                            return Poll::Ready(None);
                                        }
                                        Poll::Ready(Some(value)) => {
                                            *val = Some((field_name.clone(), value));
                                            filled_count += 1;
                                        }
                                        Poll::Pending => { /* check back later */ }
                                    }
                                }
                                _ => {
                                    *val = Some((field_name.clone(), Ok(Value::Null)));
                                    filled_count += 1;
                                }
                            }
                        }
                    }
                    if filled_count == obj_len {
                        filled_count = 0;
                        let new_vec = (0..obj_len).map(|_| None).collect::<Vec<_>>();
                        let ready_vec = std::mem::replace(&mut ready_vector, new_vec);
                        let ready_vec_iterator = ready_vec.into_iter().map(|el| {
                            let (name, val) = el.unwrap();
                            if let Ok(value) = val {
                                (name, value)
                            } else {
                                (name, Value::Null)
                            }
                        });
                        let obj = Object::from_iter(ready_vec_iterator);
                        return Poll::Ready(Some(GraphQLResponse::from_result(Ok((
                            Value::Object(obj),
                            vec![],
                        )))));
                    } else {
                        return Poll::Pending;
                    }
                },
            );

            Box::pin(stream)
        }
    }
}

impl<'a, S> SubscriptionConnection<'a, S> for Connection<'a, S> where
    S: ScalarValue + Send + Sync + 'a
{
}

impl<'a, S> futures::Stream for Connection<'a, S>
where
    S: ScalarValue + Send + Sync + 'a,
{
    type Item = GraphQLResponse<'a, S>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // this is safe as stream is only mutated here and is not moved anywhere
        let Connection { stream } = unsafe { self.get_unchecked_mut() };
        let stream = unsafe { Pin::new_unchecked(stream) };
        stream.poll_next(cx)
    }
}
