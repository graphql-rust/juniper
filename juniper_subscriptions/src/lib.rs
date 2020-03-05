// todo: update docs
use juniper::{SubscriptionCoordinator, Variables, BoxFuture, GraphQLError, ScalarValue, GraphQLTypeAsync, GraphQLSubscriptionType, Value, ValuesResultStream, ExecutionError, SubscriptionConnection, http::GraphQLRequest, Object, Context};

use std::pin::Pin;
use futures::task::Poll;
use juniper::http::GraphQLResponse;
use futures::{StreamExt, Stream};
use std::any::Any;
use std::iter::FromIterator;
use std::borrow::BorrowMut;

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
    root_node: &'a juniper::RootNode<'a, QueryT, MutationT, SubscriptionT, S>,
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
    pub fn new(
        root_node: &'a juniper::RootNode<'a, QueryT, MutationT, SubscriptionT, S>
    ) -> Self {
        Self {
            root_node
        }
    }
}

impl <'a, QueryT, MutationT, SubscriptionT, CtxT, S>
    SubscriptionCoordinator<'a, CtxT, S>
    for Coordinator<'a, QueryT, MutationT, SubscriptionT, CtxT, S>
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
    type Connection = Connection<'a, S>;

    fn subscribe(
        &'a self,
        req: &'a GraphQLRequest<S>,
        context: &'a CtxT,
    ) -> BoxFuture<'a,
            Result<Connection<'a, S>, GraphQLError<'a>>
        >
    {
        let rn = self.root_node;

        Box::pin(async move {
            let req = req;
            let ctx= context;

            let res = juniper::http::resolve_into_stream(
                req,
                rn,
                ctx,
            )
                .await?;

            let c  = Connection::from(res);

            Ok(c)
        })

    }
}

pub struct Connection<'a, S> {
    values_stream: Pin<Box<
        dyn futures::Stream<
            Item = GraphQLResponse<'a, S>
        > + Send + 'a
    >>,
}

impl<'a, S>
    From<(Value<ValuesResultStream<'a, S>>, Vec<ExecutionError<S>>)>
for Connection<'a, S>
    where
        S: ScalarValue + Send + Sync + 'a,
{
    //todo: find out why Vec<ExecutionError<S>> is needed and not ignore it
    fn from((s, e): (Value<ValuesResultStream<'a, S>>, Vec<ExecutionError<S>>)) -> Self {
        Self::from_valuesresultstream(s)
    }
}

impl<'a, S> Connection<'a, S>
    where
        S: ScalarValue + Send + Sync + 'a,
{
    pub fn from_valuesresultstream(
        stream: Value<ValuesResultStream<'a, S>>
    ) -> Self {
        use futures::stream;

        let values_stream: Pin<Box<
            dyn futures::Stream<Item = GraphQLResponse<'a, S>> + Send + 'a
        >> = match stream {
            Value::Null => todo!(),
            Value::Scalar(_) => todo!(),
            Value::List(_) => todo!(),
            Value::Object(obj) => {
                let mut key_values = obj.into_key_value_list();
                if key_values.is_empty() {
                    todo!();
                }

                let mut filled_count = 0;
                let mut ready_vector = Vec::with_capacity(key_values.len());
                for _ in 0..key_values.len() {
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
                                            },
                                            Poll::Ready(Some(value)) => {
                                                *val = Some((field_name.clone(), value));
                                                filled_count += 1;
                                            }
                                            Poll::Pending => { /* check back later */ }
                                        }
                                    },
                                    // TODO#433: not panic on errors
                                    _ => panic!("into_stream supports only Value::Scalar returned in Value::Object")
                                }
                            }
                        }
                        if filled_count == key_values.len() {
                            filled_count = 0;
                            let new_vec = (0..key_values.len()).map(|_| None).collect::<Vec<_>>();
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
            },
        };

        Self {
            values_stream
        }
    }
}

impl<'a, S> SubscriptionConnection<'a, S> for Connection<'a, S>
    where
        S: ScalarValue + Send + Sync + 'a
{
}


impl<'a, S> futures::Stream for Connection<'a, S>
    where
        S: ScalarValue + Send + Sync + 'a
{
    type Item = GraphQLResponse<'a, S>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>
    ) -> Poll<Option<Self::Item>> {
        let Connection { values_stream} =
            unsafe { self.get_unchecked_mut() };
        let values_stream = unsafe { Pin::new_unchecked(values_stream) };
        values_stream.poll_next(cx)

    }
}