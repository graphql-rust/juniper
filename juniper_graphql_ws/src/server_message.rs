//! Common definitions regarding server messages.

use std::{any::Any, marker::PhantomPinned};

use derive_more::with_trait::Debug;
#[cfg(doc)]
use juniper::futures::Stream;
use juniper::{ExecutionError, GraphQLError, Value};
use serde::{Serialize, Serializer};

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
// XXX: Think carefully before deriving traits. This is self-referential (error references
// _execution_params).
#[derive(Debug)]
#[debug("{error:?}")]
pub struct ErrorPayload {
    _execution_params: Option<Box<dyn Any + Send>>,
    error: GraphQLError,
    _pinned: PhantomPinned,
}

impl ErrorPayload {
    /// Creates a new [`ErrorPayload`] out of the provide `execution_params` and [`GraphQLError`].
    pub(crate) fn new(execution_params: Box<dyn Any + Send>, error: GraphQLError) -> Self {
        Self {
            _execution_params: Some(execution_params),
            error,
            _pinned: PhantomPinned,
        }
    }

    /// Returns the contained [`GraphQLError`].
    pub fn graphql_error(&self) -> &GraphQLError {
        &self.error
    }
}

impl PartialEq for ErrorPayload {
    fn eq(&self, other: &Self) -> bool {
        self.error.eq(&other.error)
    }
}

impl Serialize for ErrorPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.error.serialize(serializer)
    }
}

impl From<GraphQLError> for ErrorPayload {
    fn from(error: GraphQLError) -> Self {
        Self {
            _execution_params: None,
            error,
            _pinned: PhantomPinned,
        }
    }
}
