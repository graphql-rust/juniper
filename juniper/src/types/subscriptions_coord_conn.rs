use crate::{SubscriptionCoordinator, Variables, BoxFuture, GraphQLError, ScalarValue, GraphQLTypeAsync, GraphQLSubscriptionType, Value, ValuesResultStream, ExecutionError, SubscriptionConnection};
use crate::http::GraphQLRequest;

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
    ) -> BoxFuture<'c, Result<Box<dyn SubscriptionConnection + 'c>, GraphQLError<'c>>>
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

            let c: Box<dyn SubscriptionConnection + 'c> = Box::new(
                Connection::from(res)
            );

            Ok(c)
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

impl<'a, S> SubscriptionConnection for Connection<'a, S> {

}
