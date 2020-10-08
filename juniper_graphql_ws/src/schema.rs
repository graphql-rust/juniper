use juniper::{GraphQLSubscriptionType, GraphQLTypeAsync, RootNode};
use std::sync::Arc;

/// Schema defines the requirements for schemas that can be used for operations. Typically this is
/// just an `Arc<RootNode<...>>` and you should not have to implement it yourself.
pub trait Schema: Unpin + Clone + Send + Sync + 'static {
    /// The context type.
    type Context: Unpin + Send + Sync;

    /// The query type info.
    type QueryTypeInfo: Send + Sync;

    /// The query type.
    type Query: GraphQLTypeAsync<Context = Self::Context, TypeInfo = Self::QueryTypeInfo> + Send;

    /// The mutation type info.
    type MutationTypeInfo: Send + Sync;

    /// The mutation type.
    type Mutation: GraphQLTypeAsync<Context = Self::Context, TypeInfo = Self::MutationTypeInfo>
        + Send;

    /// The subscription type info.
    type SubscriptionTypeInfo: Send + Sync;

    /// The subscription type.
    type Subscription: GraphQLSubscriptionType<Context = Self::Context, TypeInfo = Self::SubscriptionTypeInfo>
        + Send;

    /// Returns the root node for the schema.
    fn root_node(&self) -> &RootNode<'static, Self::Query, Self::Mutation, Self::Subscription>;
}

/// This exists as a work-around for this issue: https://github.com/rust-lang/rust/issues/64552
///
/// It can be used in generators where using Arc directly would result in an error.
// TODO: Remove this once that issue is resolved.
#[doc(hidden)]
pub struct ArcSchema<QueryT, MutationT, SubscriptionT, CtxT>(
    pub Arc<RootNode<'static, QueryT, MutationT, SubscriptionT>>,
)
where
    QueryT: GraphQLTypeAsync<Context = CtxT> + Send + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT: GraphQLTypeAsync<Context = CtxT> + Send + 'static,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLSubscriptionType<Context = CtxT> + Send + 'static,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Unpin + Send + Sync;

impl<QueryT, MutationT, SubscriptionT, CtxT> Clone
    for ArcSchema<QueryT, MutationT, SubscriptionT, CtxT>
where
    QueryT: GraphQLTypeAsync<Context = CtxT> + Send + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT: GraphQLTypeAsync<Context = CtxT> + Send + 'static,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLSubscriptionType<Context = CtxT> + Send + 'static,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Unpin + Send + Sync,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<QueryT, MutationT, SubscriptionT, CtxT> Schema
    for ArcSchema<QueryT, MutationT, SubscriptionT, CtxT>
where
    QueryT: GraphQLTypeAsync<Context = CtxT> + Send + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT: GraphQLTypeAsync<Context = CtxT> + Send + 'static,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLSubscriptionType<Context = CtxT> + Send + 'static,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Unpin + Send + Sync + 'static,
{
    type Context = CtxT;
    type QueryTypeInfo = QueryT::TypeInfo;
    type Query = QueryT;
    type MutationTypeInfo = MutationT::TypeInfo;
    type Mutation = MutationT;
    type SubscriptionTypeInfo = SubscriptionT::TypeInfo;
    type Subscription = SubscriptionT;

    fn root_node(&self) -> &RootNode<'static, QueryT, MutationT, SubscriptionT> {
        &self.0
    }
}

impl<QueryT, MutationT, SubscriptionT, CtxT> Schema
    for Arc<RootNode<'static, QueryT, MutationT, SubscriptionT>>
where
    QueryT: GraphQLTypeAsync<Context = CtxT> + Send + 'static,
    QueryT::TypeInfo: Send + Sync,
    MutationT: GraphQLTypeAsync<Context = CtxT> + Send + 'static,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLSubscriptionType<Context = CtxT> + Send + 'static,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Unpin + Send + Sync,
{
    type Context = CtxT;
    type QueryTypeInfo = QueryT::TypeInfo;
    type Query = QueryT;
    type MutationTypeInfo = MutationT::TypeInfo;
    type Mutation = MutationT;
    type SubscriptionTypeInfo = SubscriptionT::TypeInfo;
    type Subscription = SubscriptionT;

    fn root_node(&self) -> &RootNode<'static, QueryT, MutationT, SubscriptionT> {
        self
    }
}
