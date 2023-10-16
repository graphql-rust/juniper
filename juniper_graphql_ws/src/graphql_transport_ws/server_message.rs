use juniper::{ExecutionError, Value};
use serde::Serialize;

pub use crate::server_message::ErrorPayload;

/// Sent after execution of an operation. For queries and mutations, this is sent to the client
/// once. For subscriptions, this is sent for every event in the event stream.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextPayload<S> {
    /// The result data.
    pub data: Value<S>,

    /// The errors that have occurred during execution. Note that parse and validation errors are
    /// not included here. They are sent via Error messages.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ExecutionError<S>>,
}

/// ServerMessage defines the message types that servers can send.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ServerMessage<S> {
    /// ConnectionAck is sent in response to a client's ConnectionInit message if the server accepted a
    /// connection.
    ConnectionAck,
    /// The response to the `Ping` message.
    Pong,
    /// Data contains the result of a query, mutation, or subscription event.
    Next {
        /// The id of the operation that the data is for.
        id: String,

        /// The data and errors that occurred during execution.
        payload: NextPayload<S>,
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
}

#[cfg(test)]
mod test {
    use juniper::{graphql_value, DefaultScalarValue, GraphQLError};

    use super::*;

    #[test]
    fn test_serialization() {
        type ServerMessage = super::ServerMessage<DefaultScalarValue>;

        assert_eq!(
            serde_json::to_string(&ServerMessage::ConnectionAck).unwrap(),
            r#"{"type":"connection_ack"}"#,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::Pong).unwrap(),
            r#"{"type":"pong"}"#,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::Next {
                id: "foo".into(),
                payload: NextPayload {
                    data: graphql_value!(null),
                    errors: vec![],
                },
            })
            .unwrap(),
            r#"{"type":"next","id":"foo","payload":{"data":null}}"#,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::Error {
                id: "foo".into(),
                payload: GraphQLError::UnknownOperationName.into(),
            })
            .unwrap(),
            r#"{"type":"error","id":"foo","payload":[{"message":"Unknown operation"}]}"#,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::Complete { id: "foo".into() }).unwrap(),
            r#"{"type":"complete","id":"foo"}"#,
        );
    }
}
