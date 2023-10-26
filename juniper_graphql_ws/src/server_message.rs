//! Common definitions regarding server messages.

use std::{any::Any, fmt, marker::PhantomPinned};

use juniper::GraphQLError;
use serde::{Serialize, Serializer};

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

impl fmt::Debug for ErrorPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
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
