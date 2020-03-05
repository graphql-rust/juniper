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
    values_stream: Pin<Box<
        dyn futures::Stream<Item = GraphQLResponse<'a, S>> + Send + 'a
    >>,
}

impl<'a, S>
    From<(Value<ValuesResultStream<'a, S>>, Vec<ExecutionError<S>>)>
for Connection<'a, S>
{
    fn from((s, e): (Value<ValuesResultStream<'a, S>>, Vec<ExecutionError<S>>)) -> Self {
//        Self {
//            values_stream: None
//        }
        todo!()
    }
}

impl<'a, S> Connection<'a, S>
    where
        S: ScalarValue + Send + Sync + 'a,
{

}

impl<'c, S> SubscriptionConnection<'c, S> for Connection<'c, S>
    where
        S: ScalarValue + Send + Sync + 'c
{
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
        todo!()
    }
}