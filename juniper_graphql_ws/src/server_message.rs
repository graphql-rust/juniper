use juniper::{ExecutionError, GraphQLError, ScalarValue, Value};
use serde::{Serialize, Serializer};
use std::{any::Any, fmt, marker::PhantomPinned};

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionErrorPayload {
    pub message: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(bound(serialize = "S: ScalarValue"))]
#[serde(rename_all = "camelCase")]
pub struct DataPayload<S> {
    pub data: Value<S>,
    pub errors: Vec<ExecutionError<S>>,
}

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
    /// ConnectionError is used when the server rejects a connection based on the client's ConnectionInit
    /// message or when the server encounters a protocol error such as not being able to parse a
    /// client's message.
    ConnectionError { payload: ConnectionErrorPayload },
    /// ConnectionAck is sent in response to a client's ConnectionInit message if the server accepted a
    /// connection.
    ConnectionAck,
    /// Data contains the result of a query, mutation, or subscription event.
    Data { id: String, payload: DataPayload<S> },
    /// Error contains an error that occurs before execution, such as validation errors.
    Error { id: String, payload: ErrorPayload },
    /// Complete indicates that no more data will be sent for the given operation.
    Complete { id: String },
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
            r##"{"type":"data","id":"foo","payload":{"data":null,"errors":[]}}"##,
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
