//! Common definitions regarding server messages.

use derive_more::with_trait::Debug;
#[cfg(doc)]
use juniper::futures::Stream;
use juniper::{ExecutionError, GraphQLError, Value};
use serde::Serialize;

/// Payload to be send after execution of an operation.
///
/// - For queries and mutations, this is sent to the client once.
/// - For subscriptions, this is sent for every event in the event [`Stream`].
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextPayload<S> {
    /// Execution result data.
    pub data: Value<S>,

    /// Errors that have occurred during execution.
    ///
    /// Note, that parse and validation errors are not included here. They are sent via
    /// [`ErrorPayload`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ExecutionError<S>>,
}

/// Payload for errors that can happen before execution.
///
/// Errors that happen during execution are instead sent to the client via
/// [`graphql_ws::DataPayload`] or [`graphql_transport_ws::NextPayload`]. [`ErrorPayload`] is a
/// wrapper for an owned [`GraphQLError`].
///
/// [`graphql_transport_ws::NextPayload`]: crate::graphql_transport_ws::NextPayload
/// [`graphql_ws::DataPayload`]: crate::graphql_ws::DataPayload
#[derive(Debug, PartialEq, Serialize)]
#[debug("{error:?}")]
#[serde(transparent)]
pub struct ErrorPayload {
    error: GraphQLError,
}

impl ErrorPayload {
    /// Returns the contained [`GraphQLError`].
    pub fn graphql_error(&self) -> &GraphQLError {
        &self.error
    }
}

impl From<GraphQLError> for ErrorPayload {
    fn from(error: GraphQLError) -> Self {
        Self { error }
    }
}
