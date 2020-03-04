use crate::{SubscriptionCoordinator, Variables, BoxFuture, GraphQLError, ScalarValue, GraphQLTypeAsync, GraphQLSubscriptionType};
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
    type Connection = Connection;

    fn subscribe<'c>(
        &self,
        req: &'c GraphQLRequest<S>,
        context: &'c CtxT,
    ) -> BoxFuture<'c, Result<Self::Connection, GraphQLError<'c>>>
    {
        crate::http::resolve_into_stream(
            req,
            self.root_node,
            context,
        );

        todo!()
    }
}

pub struct Connection {

}