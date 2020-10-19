use juniper::{ExecutionError, GraphQLError, ScalarValue, Value};
use serde::{Serialize, Serializer};
use std::{any::Any, fmt, marker::PhantomPinned};

/// The payload for errors that are not associated with a GraphQL operation.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionErrorPayload {
    /// The error message.
    pub message: String,
}

/// Sent after execution of an operation. For queries and mutations, this is sent to the client
/// once. For subscriptions, this is sent for every event in the event stream.
#[derive(Debug, Serialize, PartialEq)]
#[serde(bound(serialize = "S: ScalarValue"))]
#[serde(rename_all = "camelCase")]
pub struct DataPayload<S> {
    /// The result data.
    pub data: Value<S>,

    /// The errors that have occurred during execution. Note that parse and validation errors are
    /// not included here. They are sent via Error messages.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ExecutionError<S>>,
}

/// A payload for errors that can happen before execution. Errors that happen during execution are
/// instead sent to the client via `DataPayload`. `ErrorPayload` is a wrapper for an owned
/// `GraphQLError`.
// XXX: Think carefully before deriving traits. This is self-referential (error references
// _execution_params).
pub struct ErrorPayload {
    _execution_params: Option<Box<dyn Any + Send>>,
    error: GraphQLError<'static>,
    _marker: PhantomPinned,
}

impl ErrorPayload {
    /// For this to be okay, the caller must guarantee that the error can only reference data from
    /// execution_params and that execution_params has not been modified or moved.
    pub(crate) unsafe fn new_unchecked<'a>(
        execution_params: Box<dyn Any + Send>,
        error: GraphQLError<'a>,
    ) -> Self {
        Self {
            _execution_params: Some(execution_params),
            error: std::mem::transmute(error),
            _marker: PhantomPinned,
        }
    }

    /// Returns the contained GraphQLError.
    pub fn graphql_error<'a>(&'a self) -> &GraphQLError<'a> {
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

impl From<GraphQLError<'static>> for ErrorPayload {
    fn from(error: GraphQLError<'static>) -> Self {
        Self {
            _execution_params: None,
            error,
            _marker: PhantomPinned,
        }
    }
}

/// ServerMessage defines the message types that servers can send.
#[derive(Debug, Serialize, PartialEq)]
#[serde(bound(serialize = "S: ScalarValue"))]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ServerMessage<S: ScalarValue> {
    /// ConnectionError is used for errors that are not associated with a GraphQL operation. For
    /// example, this will be used when:
    ///
    ///   * The server is unable to parse a client's message.
    ///   * The client's initialization parameters are rejected.
    ConnectionError {
        /// The error that occurred.
        payload: ConnectionErrorPayload,
    },
    /// ConnectionAck is sent in response to a client's ConnectionInit message if the server accepted a
    /// connection.
    ConnectionAck,
    /// Data contains the result of a query, mutation, or subscription event.
    Data {
        /// The id of the operation that the data is for.
        id: String,

        /// The data and errors that occurred during execution.
        payload: DataPayload<S>,
    },
    /// Error contains an error that occurs before execution, such as validation errors.
    Error {
        /// The id of the operation that triggered this error.
        id: String,

        /// The error(s).
        payload: ErrorPayload,
    },
    /// Complete indicates that no more data will be sent for the given operation.
    Complete {
        /// The id of the operation that has completed.
        id: String,
    },
    /// ConnectionKeepAlive is sent periodically after accepting a connection.
    #[serde(rename = "ka")]
    ConnectionKeepAlive,
}

#[cfg(test)]
mod test {
    use super::*;
    use juniper::DefaultScalarValue;

    #[test]
    fn test_serialization() {
        type ServerMessage = super::ServerMessage<DefaultScalarValue>;

        assert_eq!(
            serde_json::to_string(&ServerMessage::ConnectionError {
                payload: ConnectionErrorPayload {
                    message: "foo".to_string(),
                },
            })
            .unwrap(),
            r##"{"type":"connection_error","payload":{"message":"foo"}}"##,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::ConnectionAck).unwrap(),
            r##"{"type":"connection_ack"}"##,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::Data {
                id: "foo".to_string(),
                payload: DataPayload {
                    data: Value::null(),
                    errors: vec![],
                },
            })
            .unwrap(),
            r##"{"type":"data","id":"foo","payload":{"data":null}}"##,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::Error {
                id: "foo".to_string(),
                payload: GraphQLError::UnknownOperationName.into(),
            })
            .unwrap(),
            r##"{"type":"error","id":"foo","payload":[{"message":"Unknown operation"}]}"##,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::Complete {
                id: "foo".to_string(),
            })
            .unwrap(),
            r##"{"type":"complete","id":"foo"}"##,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::ConnectionKeepAlive).unwrap(),
            r##"{"type":"ka"}"##,
        );
    }
}
