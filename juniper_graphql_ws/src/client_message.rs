use juniper::{ScalarValue, Variables};
use serde::Deserialize;

use crate::utils::default_for_null;

/// The payload for a client's "start" message. This triggers execution of a query, mutation, or
/// subscription.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(bound(deserialize = "S: ScalarValue"))]
#[serde(rename_all = "camelCase")]
pub struct StartPayload<S: ScalarValue> {
    /// The document body.
    pub query: String,

    /// The optional variables.
    #[serde(default, deserialize_with = "default_for_null")]
    pub variables: Variables<S>,

    /// The optional operation name (required if the document contains multiple operations).
    pub operation_name: Option<String>,
}

/// ClientMessage defines the message types that clients can send.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(bound(deserialize = "S: ScalarValue"))]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ClientMessage<S: ScalarValue> {
    /// ConnectionInit is sent by the client upon connecting.
    ConnectionInit {
        /// Optional parameters of any type sent from the client. These are often used for
        /// authentication.
        #[serde(default, deserialize_with = "default_for_null")]
        payload: Variables<S>,
    },
    /// Start messages are used to execute a GraphQL operation.
    Start {
        /// The id of the operation. This can be anything, but must be unique. If there are other
        /// in-flight operations with the same id, the message will be ignored or cause an error.
        id: String,

        /// The query, variables, and operation name.
        payload: StartPayload<S>,
    },
    /// Stop messages are used to unsubscribe from a subscription.
    Stop {
        /// The id of the operation to stop.
        id: String,
    },
    /// ConnectionTerminate is used to terminate the connection.
    ConnectionTerminate,
}

#[cfg(test)]
mod test {
    use super::*;
    use juniper::{DefaultScalarValue, InputValue};

    #[test]
    fn test_deserialization() {
        type ClientMessage = super::ClientMessage<DefaultScalarValue>;

        assert_eq!(
            ClientMessage::ConnectionInit {
                payload: [("foo".to_string(), InputValue::scalar("bar"))]
                    .iter()
                    .cloned()
                    .collect(),
            },
            serde_json::from_str(r##"{"type": "connection_init", "payload": {"foo": "bar"}}"##)
                .unwrap(),
        );

        assert_eq!(
            ClientMessage::ConnectionInit {
                payload: Variables::default(),
            },
            serde_json::from_str(r##"{"type": "connection_init"}"##).unwrap(),
        );

        assert_eq!(
            ClientMessage::Start {
                id: "foo".to_string(),
                payload: StartPayload {
                    query: "query MyQuery { __typename }".to_string(),
                    variables: [("foo".to_string(), InputValue::scalar("bar"))]
                        .iter()
                        .cloned()
                        .collect(),
                    operation_name: Some("MyQuery".to_string()),
                },
            },
            serde_json::from_str(
                r##"{"type": "start", "id": "foo", "payload": {
                "query": "query MyQuery { __typename }",
                "variables": {
                    "foo": "bar"
                },
                "operationName": "MyQuery"
            }}"##
            )
            .unwrap(),
        );

        assert_eq!(
            ClientMessage::Start {
                id: "foo".to_string(),
                payload: StartPayload {
                    query: "query MyQuery { __typename }".to_string(),
                    variables: Variables::default(),
                    operation_name: None,
                },
            },
            serde_json::from_str(
                r##"{"type": "start", "id": "foo", "payload": {
                "query": "query MyQuery { __typename }"
            }}"##
            )
            .unwrap(),
        );

        assert_eq!(
            ClientMessage::Stop {
                id: "foo".to_string()
            },
            serde_json::from_str(r##"{"type": "stop", "id": "foo"}"##).unwrap(),
        );

        assert_eq!(
            ClientMessage::ConnectionTerminate,
            serde_json::from_str(r##"{"type": "connection_terminate"}"##).unwrap(),
        );
    }

    #[test]
    fn test_deserialization_of_null() -> serde_json::Result<()> {
        let payload = r#"{"query":"query","variables":null}"#;
        let payload: StartPayload<DefaultScalarValue> = serde_json::from_str(payload)?;

        let expected = StartPayload {
            query: "query".into(),
            variables: Variables::default(),
            operation_name: None,
        };

        assert_eq!(expected, payload);

        Ok(())
    }
}
