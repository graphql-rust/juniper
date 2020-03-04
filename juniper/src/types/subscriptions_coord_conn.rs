use crate::{SubscriptionCoordinator, Variables, BoxFuture, GraphQLError, ScalarValue, GraphQLTypeAsync, GraphQLSubscriptionType, Value, ValuesResultStream, ExecutionError, SubscriptionConnection, http::GraphQLRequest, Object};

use std::pin::Pin;
use futures::task::Poll;
use crate::http::GraphQLResponse;
use futures::{StreamExt, Stream};
use std::any::Any;

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
    root_node: &'a crate::RootNode<'a, QueryT, MutationT, SubscriptionT, S>
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
        root_node: &'a crate::RootNode<'a, QueryT, MutationT, SubscriptionT, S>
    ) -> Self {
        Self {
            root_node
        }
    }
}

impl <'a, QueryT, MutationT, SubscriptionT, CtxT, S>
        SubscriptionCoordinator<CtxT, S>
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
    fn subscribe<'c>(
        &'c self,
        req: &'c GraphQLRequest<S>,
        context: &'c CtxT,
//    ) -> BoxFuture<'c, Result<Box<dyn SubscriptionConnection<S> + 'c>, GraphQLError<'c>>>
//todo: return box<result> or T
    ) -> BoxFuture<'c, Result<crate::Connection<'c, S>, GraphQLError<'c>>>
    {
        let rn = self.root_node;

        Box::pin(async move {
            let req  = req;
            let ctx= context;

            let res= crate::http::resolve_into_stream(
                req,
                rn,
                ctx,
            )
                .await?;

//            let c: Box<dyn SubscriptionConnection<S> + 'c> = Box::new(
//                Connection::from(res)
//            );

            Ok(
                Connection::from(res)
            )
        })

    }
}

pub struct Connection<'a, S> {
    stream: Value<ValuesResultStream<'a, S>>,
    err: Vec<ExecutionError<S>>,
}

impl<'a, S>
    From<(Value<ValuesResultStream<'a, S>>, Vec<ExecutionError<S>>)>
    for Connection<'a, S>
{
    fn from((s, e): (Value<ValuesResultStream<'a, S>>, Vec<ExecutionError<S>>)) -> Self {
        Self {
            stream: s,
            err: e,
        }
    }
}

impl<'c, S> SubscriptionConnection<'c, S> for Connection<'c, S>
where
    S: ScalarValue + Send + Sync + 'c
{

    /// Converts `Self` into default `Stream` implementantion
    // todo: refactor
    fn into_stream(
        self,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = GraphQLResponse<'static, S>> + Send + 'c>>,
        Vec<ExecutionError<S>>,
    > {
        use std::iter::FromIterator as _;

        if self.err.len() != 0 {
            return Err(vec![]);
        };

        match self.stream {
            Value::Null => Err(vec![]),
            Value::Scalar(stream) => {
                Ok(Box::pin(stream.map(|value| {
                    match value {
                        Ok(val) => GraphQLResponse::from_result(Ok((val, vec![]))),
                        // TODO#433: not return random error
                        Err(_) => GraphQLResponse::from_result(Err(GraphQLError::IsSubscription)),
                    }
                })))
            }
            // TODO#433: remove this implementation and add a // TODO: implement these
            //           (current implementation might be confusing)
            Value::List(_) => return Err(vec![]),
            Value::Object(obj) => {
                let mut key_values = obj.into_key_value_list();
                if key_values.is_empty() {
                    return Err(vec![]);
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

                Ok(Box::pin(stream))
            }
        }
    }

}