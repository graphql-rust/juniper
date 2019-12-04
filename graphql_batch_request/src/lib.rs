use juniper::{
    http, serde::Deserialize, DefaultScalarValue, GraphQLType, InputValue, RootNode, ScalarValue,
};

#[cfg(feature = "async")]
use juniper::GraphQLTypeAsync;

#[derive(Debug, serde_derive::Deserialize, PartialEq)]
#[serde(untagged)]
#[serde(bound = "InputValue<S>: Deserialize<'de>")]
pub enum GraphQLBatchRequest<S = DefaultScalarValue>
where
    S: ScalarValue,
{
    Single(http::GraphQLRequest<S>),
    Batch(Vec<http::GraphQLRequest<S>>),
}

#[derive(serde_derive::Serialize)]
#[serde(untagged)]
pub enum GraphQLBatchResponse<'a, S = DefaultScalarValue>
where
    S: ScalarValue,
{
    Single(http::GraphQLResponse<'a, S>),
    Batch(Vec<http::GraphQLResponse<'a, S>>),
}

impl<S> GraphQLBatchRequest<S>
where
    S: ScalarValue,
{
    pub fn execute<'a, CtxT, QueryT, MutationT>(
        &'a self,
        root_node: &'a RootNode<QueryT, MutationT, S>,
        context: &CtxT,
    ) -> GraphQLBatchResponse<'a, S>
    where
        QueryT: GraphQLType<S, Context = CtxT>,
        MutationT: GraphQLType<S, Context = CtxT>,
    {
        match self {
            &GraphQLBatchRequest::Single(ref request) => {
                GraphQLBatchResponse::Single(request.execute(root_node, context))
            }
            &GraphQLBatchRequest::Batch(ref requests) => GraphQLBatchResponse::Batch(
                requests
                    .iter()
                    .map(|request| request.execute(root_node, context))
                    .collect(),
            ),
        }
    }

    pub fn operation_names(&self) -> Vec<Option<&str>> {
        match self {
            GraphQLBatchRequest::Single(req) => vec![req.operation_name()],
            GraphQLBatchRequest::Batch(reqs) => {
                reqs.iter().map(|req| req.operation_name()).collect()
            }
        }
    }
}

#[cfg(feature = "async")]
impl<S> GraphQLBatchRequest<S>
where
    S: ScalarValue + Send + Sync,
{
    pub async fn execute_async<'a, CtxT, QueryT, MutationT>(
        &'a self,
        root_node: &'a RootNode<'_, QueryT, MutationT, S>,
        context: &'a CtxT,
    ) -> GraphQLBatchResponse<'a, S>
    where
        QueryT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
        QueryT::TypeInfo: Send + Sync,
        MutationT: GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
        MutationT::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
    {
        match self {
            &GraphQLBatchRequest::Single(ref request) => {
                GraphQLBatchResponse::Single(request.execute_async(root_node, context).await)
            }
            &GraphQLBatchRequest::Batch(ref requests) => {
                let futures = requests
                    .iter()
                    .map(|request| request.execute_async(root_node, context))
                    .collect::<Vec<_>>();

                GraphQLBatchResponse::Batch(futures::future::join_all(futures).await)
            }
        }
    }
}

impl<'a, S> GraphQLBatchResponse<'a, S>
where
    S: ScalarValue,
{
    pub fn is_ok(&self) -> bool {
        match self {
            &GraphQLBatchResponse::Single(ref response) => response.is_ok(),
            &GraphQLBatchResponse::Batch(ref responses) => responses
                .iter()
                .fold(true, |ok, response| ok && response.is_ok()),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
