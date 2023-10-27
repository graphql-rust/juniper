use juniper::Variables;
use serde::Deserialize;

use crate::util::default_for_null;

/// The payload for a client's "start" message. This triggers execution of a query, mutation, or
/// subscription.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
#[serde(rename_all = "camelCase")]
pub struct StartPayload<S> {
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
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ClientMessage<S> {
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
    use juniper::{graphql_vars, DefaultScalarValue};

    use super::*;

    #[test]
    fn test_deserialization() {
        type ClientMessage = super::ClientMessage<DefaultScalarValue>;

        assert_eq!(
            ClientMessage::ConnectionInit {
                payload: graphql_vars! {"foo": "bar"},
            },
            serde_json::from_str(r#"{"type": "connection_init", "payload": {"foo": "bar"}}"#)
                .unwrap(),
        );

        assert_eq!(
            ClientMessage::ConnectionInit {
                payload: graphql_vars! {},
            },
            serde_json::from_str(r#"{"type": "connection_init"}"#).unwrap(),
        );

        assert_eq!(
            ClientMessage::Start {
                id: "foo".into(),
                payload: StartPayload {
                    query: "query MyQuery { __typename }".into(),
                    variables: graphql_vars! {"foo": "bar"},
                    operation_name: Some("MyQuery".into()),
                },
            },
            serde_json::from_str(
                r#"{"type": "start", "id": "foo", "payload": {
                "query": "query MyQuery { __typename }",
                "variables": {
                    "foo": "bar"
                },
                "operationName": "MyQuery"
            }}"#
            )
            .unwrap(),
        );

        assert_eq!(
            ClientMessage::Start {
                id: "foo".into(),
                payload: StartPayload {
                    query: "query MyQuery { __typename }".into(),
                    variables: graphql_vars! {},
                    operation_name: None,
                },
            },
            serde_json::from_str(
                r#"{"type": "start", "id": "foo", "payload": {
                "query": "query MyQuery { __typename }"
            }}"#
            )
            .unwrap(),
        );

        assert_eq!(
            ClientMessage::Stop { id: "foo".into() },
            serde_json::from_str(r#"{"type": "stop", "id": "foo"}"#).unwrap(),
        );

        assert_eq!(
            ClientMessage::ConnectionTerminate,
            serde_json::from_str(r#"{"type": "connection_terminate"}"#).unwrap(),
        );
    }

    #[test]
    fn test_deserialization_of_null() -> serde_json::Result<()> {
        let payload = r#"{"query":"query","variables":null}"#;
        let payload: StartPayload<DefaultScalarValue> = serde_json::from_str(payload)?;

        let expected = StartPayload {
            query: "query".into(),
            variables: graphql_vars! {},
            operation_name: None,
        };

        assert_eq!(expected, payload);

        Ok(())
    }
}
