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
    root_node: &'a juniper::RootNode<'a, QueryT, MutationT, SubscriptionT, S>
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
//todo: return box<result> or T
    ) -> BoxFuture<'c,
            Result<
                Box<dyn SubscriptionConnection<S> + 'c>,
                GraphQLError<'c>
            >
        >
    {
        let rn = self.root_node;

        Box::pin(async move {
            let req  = req;
            let ctx= context;

            let res = juniper::http::resolve_into_stream(
                req,
                rn,
                ctx,
            )
                .await?;

            let c: Box<dyn SubscriptionConnection<S> + 'c> = Box::new(
                Connection::from(res)
            );

            Ok(c)
        })

    }
}

pub struct Connection<'a, S> {
    stream: Value<ValuesResultStream<'a, S>>,
    err: Vec<ExecutionError<S>>,

    values_stream: Option<Pin<Box<
        dyn futures::Stream<Item = GraphQLResponse<'a, S>> + Send + 'a
    >>>,
}

impl<'a, S>
From<(Value<ValuesResultStream<'a, S>>, Vec<ExecutionError<S>>)>
for Connection<'a, S>
{
    fn from((s, e): (Value<ValuesResultStream<'a, S>>, Vec<ExecutionError<S>>)) -> Self {
        Self {
            stream: s,
            err: e,
            values_stream: None
        }
    }
}

impl<'a, S> Connection<'a, S>
    where
        S: ScalarValue + Send + Sync + 'a,
{
    fn init(&mut self) -> Result<(), ()> {
        use std::iter::FromIterator as _;

        if self.err.len() != 0 {
            return Err(());
        };

        match &mut self.stream {
            Value::Null => return Err(()),
            Value::Scalar(stream) => {
                self.values_stream = Some(Box::pin(stream.map(|value| {
                    match value {
                        Ok(val) => GraphQLResponse::from_result(Ok((val, vec![]))),
                        // TODO#433: not return random error
                        Err(_) => GraphQLResponse::from_result(Err(GraphQLError::IsSubscription)),
                    }
                })));
            }
            // TODO#433: remove this implementation and add a // TODO: implement these
            //           (current implementation might be confusing)
            Value::List(_) => return Err(()),
            Value::Object(obj) => {
                let len = obj.field_count();
                let mut key_values = obj.iter_mut();
                if len == 0 {
                    return Err(());
                }

                let mut filled_count = 0;
                let mut ready_vector = Vec::with_capacity(len);
                for _ in 0..len {
                    ready_vector.push(None);
                }

                let stream = futures::stream::poll_fn(
                    move |mut ctx| -> Poll<Option<GraphQLResponse<'static, S>>> {
                        for i in 0..ready_vector.len() {
                            let val = &mut ready_vector[i];
                            if val.is_none() {
                                //todo: not unwrap
                                let (field_name, ref mut stream_val) = &mut key_values.next().unwrap();

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
                        if filled_count == len {
                            filled_count = 0;
                            let new_vec = (0..len).map(|_| None).collect::<Vec<_>>();
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

                self.values_stream = Some(Box::pin(stream));
            }
        }

//        Ok(self)
        Ok(())
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
                let len = obj.field_count();
                let mut key_values = obj.into_key_value_list();
                if key_values.is_empty() {
                    return Err(vec![]);
                }

                let mut filled_count = 0;
                let mut ready_vector = Vec::with_capacity(len);
                for _ in 0..len {
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
                        if filled_count == len {
                            filled_count = 0;
                            let new_vec = (0..len).map(|_| None).collect::<Vec<_>>();
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


impl<'c, S> futures::Stream for Connection<'c, S>
    where
        S: ScalarValue + Send + Sync + 'c
{
    type Item = GraphQLResponse<'static, S>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context<'_>
    ) -> Poll<Option<Self::Item>> {
//        match self.stream {
//            Value::Null => Poll::Ready(None),
//            Value::Scalar(ref mut stream) => {
//                let v = match stream.poll_next(cx) {
//                    Poll::Ready(v) => v,
//                    Poll::Pending => return Poll::Pending,
//                };
//
//                Poll::Ready(v.map(|value| {
//                    match value {
//                        Ok(val) => GraphQLResponse::from_result(Ok((val, vec![]))),
//                        // TODO#433: not return random error
//                        Err(_) => GraphQLResponse::from_result(Err(GraphQLError::IsSubscription)),
//                    }
//                }))
//            }
//            // TODO#433: remove this implementation and add a // TODO: implement these
//            //           (current implementation might be confusing)
////            Value::List(_) => return Poll::Ready(None),
////            Value::Object(ref mut obj) => {
////                let field_count = obj.field_count();
////                let mut key_values = obj.iter_mut();
//////                if key_values.is_empty() { return Poll::Ready(None); }
////
////                let mut filled_count = 0;
////                let mut ready_vector = Vec::with_capacity(field_count);
////                for _ in 0..field_count {
////                    ready_vector.push(None);
////                }
////
////                for i in 0..ready_vector.len() {
////                    let val = &mut ready_vector[i];
////                    if val.is_none() {
////                        let (field_name, ref mut stream_val) = match key_values.next()
////                            {
////                                Some(v) => v,
////                                None => return Poll::Ready(None)
////                            };
////
////                        match stream_val {
////                            Value::Scalar(ref mut stream) => {
////                                match Pin::new(stream).poll_next(cx) {
////                                    Poll::Ready(None) => {
////                                        return Poll::Ready(None);
////                                    },
////                                    Poll::Ready(Some(value)) => {
////                                        *val = Some((field_name.clone(), value));
////                                        filled_count += 1;
////                                    }
////                                    Poll::Pending => { /* check back later */ }
////                                }
////                            },
////                            // TODO#433: not panic on errors
////                            _ => panic!("into_stream supports only Value::Scalar returned in Value::Object")
////                        }
////                    }
////                }
////                if filled_count == field_count {
////                    filled_count = 0;
////                    let new_vec = (0..field_count).map(|_| None).collect::<Vec<_>>();
////                    let ready_vec = std::mem::replace(&mut ready_vector, new_vec);
////                    let ready_vec_iterator = ready_vec.into_iter()
////                        .map(|el| {
////                            let (name, val) = el.unwrap();
////                            if let Ok(value) = val {
////                                (name, value)
////                            } else {
////                                (name, Value::Null)
////                            }
////                        });
////                    let obj = Object::from_iter(ready_vec_iterator);
////                    return Poll::Ready(Some(GraphQLResponse::from_result(Ok((
////                        Value::Object(obj),
////                        vec![],
////                    )))));
////                } else {
////                    return Poll::Pending;
////                }
////
////            }
//            _ => todo!()
//        }
        todo!()
    }
}