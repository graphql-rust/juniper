use serde::Serialize;

pub use crate::server_message::{ErrorPayload, NextPayload as DataPayload};

/// The payload for errors that are not associated with a GraphQL operation.
#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionErrorPayload {
    /// The error message.
    pub message: String,
}

/// ServerMessage defines the message types that servers can send.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ServerMessage<S> {
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
    use juniper::{DefaultScalarValue, GraphQLError, graphql_value};

    use super::*;

    #[test]
    fn test_serialization() {
        type ServerMessage = super::ServerMessage<DefaultScalarValue>;

        assert_eq!(
            serde_json::to_string(&ServerMessage::ConnectionError {
                payload: ConnectionErrorPayload {
                    message: "foo".into(),
                },
            })
            .unwrap(),
            r#"{"type":"connection_error","payload":{"message":"foo"}}"#,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::ConnectionAck).unwrap(),
            r#"{"type":"connection_ack"}"#,
        );

        assert_eq!(
            serde_json::to_string(&ServerMessage::Data {
                id: "foo".into(),
                payload: DataPayload {
                    data: graphql_value!(null),
                    errors: vec![],
                },
            })
            .unwrap(),
            r#"{"type":"data","id":"foo","payload":{"data":null}}"#,
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

        assert_eq!(
            serde_json::to_string(&ServerMessage::ConnectionKeepAlive).unwrap(),
            r#"{"type":"ka"}"#,
        );
    }
}
