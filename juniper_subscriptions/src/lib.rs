//! This crate supplies [`SubscriptionCoordinator`] and
//! [`SubscriptionConnection`] implementations for the
//! [juniper](https://github.com/graphql-rust/juniper) crate.
//!
//! You need both this and `juniper` crate.
//!
//! [`SubscriptionCoordinator`]: juniper::SubscriptionCoordinator
//! [`SubscriptionConnection`]: juniper::SubscriptionConnection

#![deny(missing_docs)]
#![deny(warnings)]
#![doc(html_root_url = "https://docs.rs/juniper_subscriptions/0.14.2")]

use std::{iter::FromIterator, pin::Pin};

use futures::{task::Poll, Stream};
use juniper::{
    http::{GraphQLRequest, GraphQLResponse},
    BoxFuture, ExecutionError, GraphQLError, GraphQLSubscriptionType, GraphQLTypeAsync, Object,
    ScalarValue, SubscriptionConnection, SubscriptionCoordinator, Value, ValuesStream,
};
/// Utilities for the implementation of subscriptions over WebSocket
///
/// This module provides some utilities for the implementation of Subscriptions over
/// WebSocket, such as the [`GraphQLOverWebSocketMessage`] that contains the messages that
/// could be sent by the server or client and a [`SubscriptionStateHandler`] trait that allows
/// the user of the integration to handle some Subscription Life Cycle Events, its based on:
///
/// - [Subscriptions Transport over WS][SubscriptionsWsProtocol]
/// - [GraphQL Subscriptions LifeCycle Events][GraphQLSubscriptionsLifeCycle]
///
/// [SubscriptionsWsProtocol]: https://github.com/apollographql/subscriptions-transport-ws/blob/master/PROTOCOL.md
/// [GraphQLSubscriptionsLifeCycle]: https://www.apollographql.com/docs/graphql-subscriptions/lifecycle-events/
/// [`GraphQLOverWebSocketMessage`]: GraphQLOverWebSocketMessage
/// [`SubscriptionStateHandler`]: SubscriptionStateHandler
#[cfg(feature = "ws-subscriptions")]
pub mod ws_util;
#[cfg(feature = "ws-subscriptions")]
pub use ws_util::*;

/// Simple [`SubscriptionCoordinator`] implementation:
/// - contains the schema
/// - handles subscription start
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

    type Error = GraphQLError<'a>;

    fn subscribe(
        &'a self,
        req: &'a GraphQLRequest<S>,
        context: &'a CtxT,
    ) -> BoxFuture<'a, Result<Self::Connection, Self::Error>> {
        let rn = &self.root_node;

        Box::pin(async move {
            let (stream, errors) = juniper::http::resolve_into_stream(req, rn, context).await?;

            Ok(Connection::from_stream(stream, errors))
        })
    }
}

/// Simple [`SubscriptionConnection`] implementation.
///
/// Resolves `Value<ValuesStream>` into `Stream<Item = GraphQLResponse>` using the following
/// logic:
///
/// [`Value::Null`] - returns [`Value::Null`] once
/// [`Value::Scalar`] - returns `Ok` value or [`Value::Null`] and errors vector
/// [`Value::List`] - resolves each stream from the list using current logic and returns
///                   values in the order received
/// [`Value::Object`] - waits while each field of the [`Object`] is returned, then yields the whole object
/// `Value::Object<Value::Object<_>>` - returns [`Value::Null`] if [`Value::Object`] consists of sub-objects
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

/// Creates [`futures::Stream`] that yields [`GraphQLResponse`]s depending on the given [`Value`]:
///
/// [`Value::Null`] - returns [`Value::Null`] once
/// [`Value::Scalar`] - returns `Ok` value or [`Value::Null`] and errors vector
/// [`Value::List`] - resolves each stream from the list using current logic and returns
///                   values in the order received
/// [`Value::Object`] - waits while each field of the [`Object`] is returned, then yields the whole object
/// `Value::Object<Value::Object<_>>` - returns [`Value::Null`] if [`Value::Object`] consists of sub-objects
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
        Value::Object(mut object) => {
            let obj_len = object.field_count();
            if obj_len == 0 {
                return Box::pin(stream::once(async move {
                    GraphQLResponse::from_result(Ok((Value::Null, vec![])))
                }));
            }

            let mut filled_count = 0;
            let mut ready_vec = Vec::with_capacity(obj_len);
            for _ in 0..obj_len {
                ready_vec.push(None);
            }

            let stream = futures::stream::poll_fn(
                move |mut ctx| -> Poll<Option<GraphQLResponse<'static, S>>> {
                    let mut obj_iterator = object.iter_mut();

                    // Due to having to modify `ready_vec` contents (by-move pattern)
                    // and only being able to iterate over `object`'s mutable references (by-ref pattern)
                    // `ready_vec` and `object` cannot be iterated simultaneously.
                    // TODO: iterate over i and (ref field_name, ref val) once
                    //       [this RFC](https://github.com/rust-lang/rust/issues/68354)
                    //       is implemented
                    for ready in ready_vec.iter_mut().take(obj_len) {
                        let (field_name, val) = match obj_iterator.next() {
                            Some(v) => v,
                            None => break,
                        };

                        if ready.is_some() {
                            continue;
                        }

                        match val {
                            Value::Scalar(stream) => {
                                match Pin::new(stream).poll_next(&mut ctx) {
                                    Poll::Ready(None) => return Poll::Ready(None),
                                    Poll::Ready(Some(value)) => {
                                        *ready = Some((field_name.clone(), value));
                                        filled_count += 1;
                                    }
                                    Poll::Pending => { /* check back later */ }
                                }
                            }
                            _ => {
                                // For now only `Object<Value::Scalar>` is supported
                                *ready = Some((field_name.clone(), Ok(Value::Null)));
                                filled_count += 1;
                            }
                        }
                    }

                    if filled_count == obj_len {
                        filled_count = 0;
                        let new_vec = (0..obj_len).map(|_| None).collect::<Vec<_>>();
                        let ready_vec = std::mem::replace(&mut ready_vec, new_vec);
                        let ready_vec_iterator = ready_vec.into_iter().map(|el| {
                            let (name, val) = el.unwrap();
                            if let Ok(value) = val {
                                (name, value)
                            } else {
                                (name, Value::Null)
                            }
                        });
                        let obj = Object::from_iter(ready_vec_iterator);
                        Poll::Ready(Some(GraphQLResponse::from_result(Ok((
                            Value::Object(obj),
                            vec![],
                        )))))
                    } else {
                        Poll::Pending
                    }
                },
            );

            Box::pin(stream)
        }
    }
}

#[cfg(test)]
mod whole_responses_stream {
    use super::*;
    use futures::{stream, StreamExt as _};
    use juniper::{DefaultScalarValue, ExecutionError, FieldError};

    #[tokio::test]
    async fn with_error() {
        let expected = vec![GraphQLResponse::<DefaultScalarValue>::error(
            FieldError::new("field error", Value::Null),
        )];
        let expected = serde_json::to_string(&expected).unwrap();

        let result = whole_responses_stream::<DefaultScalarValue>(
            Value::Null,
            vec![ExecutionError::at_origin(FieldError::new(
                "field error",
                Value::Null,
            ))],
        )
        .collect::<Vec<_>>()
        .await;
        let result = serde_json::to_string(&result).unwrap();

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn value_null() {
        let expected = vec![GraphQLResponse::<DefaultScalarValue>::from_result(Ok((
            Value::Null,
            vec![],
        )))];
        let expected = serde_json::to_string(&expected).unwrap();

        let result = whole_responses_stream::<DefaultScalarValue>(Value::Null, vec![])
            .collect::<Vec<_>>()
            .await;
        let result = serde_json::to_string(&result).unwrap();

        assert_eq!(result, expected);
    }

    type PollResult = Result<Value<DefaultScalarValue>, ExecutionError<DefaultScalarValue>>;

    #[tokio::test]
    async fn value_scalar() {
        let expected = vec![
            GraphQLResponse::from_result(Ok((
                Value::Scalar(DefaultScalarValue::Int(1i32)),
                vec![],
            ))),
            GraphQLResponse::from_result(Ok((
                Value::Scalar(DefaultScalarValue::Int(2i32)),
                vec![],
            ))),
            GraphQLResponse::from_result(Ok((
                Value::Scalar(DefaultScalarValue::Int(3i32)),
                vec![],
            ))),
            GraphQLResponse::from_result(Ok((
                Value::Scalar(DefaultScalarValue::Int(4i32)),
                vec![],
            ))),
            GraphQLResponse::from_result(Ok((
                Value::Scalar(DefaultScalarValue::Int(5i32)),
                vec![],
            ))),
        ];
        let expected = serde_json::to_string(&expected).unwrap();

        let mut counter = 0;
        let stream = stream::poll_fn(move |_| -> Poll<Option<PollResult>> {
            if counter == 5 {
                return Poll::Ready(None);
            }
            counter += 1;
            Poll::Ready(Some(Ok(Value::Scalar(DefaultScalarValue::Int(counter)))))
        });

        let result =
            whole_responses_stream::<DefaultScalarValue>(Value::Scalar(Box::pin(stream)), vec![])
                .collect::<Vec<_>>()
                .await;
        let result = serde_json::to_string(&result).unwrap();

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn value_list() {
        let expected = vec![
            GraphQLResponse::from_result(Ok((
                Value::Scalar(DefaultScalarValue::Int(1i32)),
                vec![],
            ))),
            GraphQLResponse::from_result(Ok((
                Value::Scalar(DefaultScalarValue::Int(2i32)),
                vec![],
            ))),
            GraphQLResponse::from_result(Ok((Value::Null, vec![]))),
            GraphQLResponse::from_result(Ok((
                Value::Scalar(DefaultScalarValue::Int(4i32)),
                vec![],
            ))),
        ];
        let expected = serde_json::to_string(&expected).unwrap();

        let streams: Vec<Value<ValuesStream>> = vec![
            Value::Scalar(Box::pin(stream::once(async {
                PollResult::Ok(Value::Scalar(DefaultScalarValue::Int(1i32)))
            }))),
            Value::Scalar(Box::pin(stream::once(async {
                PollResult::Ok(Value::Scalar(DefaultScalarValue::Int(2i32)))
            }))),
            Value::Null,
            Value::Scalar(Box::pin(stream::once(async {
                PollResult::Ok(Value::Scalar(DefaultScalarValue::Int(4i32)))
            }))),
        ];

        let result = whole_responses_stream::<DefaultScalarValue>(Value::List(streams), vec![])
            .collect::<Vec<_>>()
            .await;
        let result = serde_json::to_string(&result).unwrap();

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn value_object() {
        let expected = vec![
            GraphQLResponse::from_result(Ok((
                Value::Object(Object::from_iter(
                    vec![
                        ("one", Value::Scalar(DefaultScalarValue::Int(1i32))),
                        ("two", Value::Scalar(DefaultScalarValue::Int(1i32))),
                    ]
                    .into_iter(),
                )),
                vec![],
            ))),
            GraphQLResponse::from_result(Ok((
                Value::Object(Object::from_iter(
                    vec![
                        ("one", Value::Scalar(DefaultScalarValue::Int(2i32))),
                        ("two", Value::Scalar(DefaultScalarValue::Int(2i32))),
                    ]
                    .into_iter(),
                )),
                vec![],
            ))),
        ];
        let expected = serde_json::to_string(&expected).unwrap();

        let mut counter = 0;
        let big_stream = stream::poll_fn(move |_| -> Poll<Option<PollResult>> {
            if counter == 2 {
                return Poll::Ready(None);
            }
            counter += 1;
            Poll::Ready(Some(Ok(Value::Scalar(DefaultScalarValue::Int(counter)))))
        });

        let mut counter = 0;
        let small_stream = stream::poll_fn(move |_| -> Poll<Option<PollResult>> {
            if counter == 2 {
                return Poll::Ready(None);
            }
            counter += 1;
            Poll::Ready(Some(Ok(Value::Scalar(DefaultScalarValue::Int(counter)))))
        });

        let vals: Vec<(&str, Value<ValuesStream>)> = vec![
            ("one", Value::Scalar(Box::pin(big_stream))),
            ("two", Value::Scalar(Box::pin(small_stream))),
        ];

        let result = whole_responses_stream::<DefaultScalarValue>(
            Value::Object(Object::from_iter(vals.into_iter())),
            vec![],
        )
        .collect::<Vec<_>>()
        .await;
        let result = serde_json::to_string(&result).unwrap();

        assert_eq!(result, expected);
    }
}
