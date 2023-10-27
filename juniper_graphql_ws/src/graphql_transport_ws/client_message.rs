use juniper::Variables;
use serde::Deserialize;

use crate::util::default_for_null;

/// The payload for a client's "start" message. This triggers execution of a query, mutation, or
/// subscription.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
#[serde(rename_all = "camelCase")]
pub struct SubscribePayload<S> {
    /// The document body.
    pub query: String,

    /// The optional variables.
    #[serde(default, deserialize_with = "default_for_null")]
    pub variables: Variables<S>,

    /// The optional operation name (required if the document contains multiple operations).
    pub operation_name: Option<String>,

    /// The optional extension data.
    #[serde(default, deserialize_with = "default_for_null")]
    pub extensions: Variables<S>,
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
    /// Ping is used for detecting failed connections, displaying latency metrics or other types of network probing.
    Ping {
        /// Optional parameters of any type used to transfer additional details about the ping.
        #[serde(default, deserialize_with = "default_for_null")]
        payload: Variables<S>,
    },
    /// The response to the `Ping` message.
    Pong {
        /// Optional parameters of any type used to transfer additional details about the pong.
        #[serde(default, deserialize_with = "default_for_null")]
        payload: Variables<S>,
    },
    /// Requests an operation specified in the message payload.
    Subscribe {
        /// The id of the operation. This can be anything, but must be unique. If there are other
        /// in-flight operations with the same id, the message will cause an error.
        id: String,

        /// The query, variables, and operation name.
        payload: SubscribePayload<S>,
    },
    /// Indicates that the client has stopped listening and wants to complete the subscription.
    Complete {
        /// The id of the operation to stop.
        id: String,
    },
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
            ClientMessage::Subscribe {
                id: "foo".into(),
                payload: SubscribePayload {
                    query: "query MyQuery { __typename }".into(),
                    variables: graphql_vars! {"foo": "bar"},
                    operation_name: Some("MyQuery".into()),
                    extensions: Default::default(),
                },
            },
            serde_json::from_str(
                r#"{"type": "subscribe", "id": "foo", "payload": {
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
            ClientMessage::Subscribe {
                id: "foo".into(),
                payload: SubscribePayload {
                    query: "query MyQuery { __typename }".into(),
                    variables: graphql_vars! {},
                    operation_name: None,
                    extensions: Default::default(),
                },
            },
            serde_json::from_str(
                r#"{"type": "subscribe", "id": "foo", "payload": {
                "query": "query MyQuery { __typename }"
            }}"#
            )
            .unwrap(),
        );

        assert_eq!(
            ClientMessage::Complete { id: "foo".into() },
            serde_json::from_str(r#"{"type": "complete", "id": "foo"}"#).unwrap(),
        );
    }

    #[test]
    fn test_deserialization_of_null() -> serde_json::Result<()> {
        let payload = r#"{"query":"query","variables":null}"#;
        let payload: SubscribePayload<DefaultScalarValue> = serde_json::from_str(payload)?;

        let expected = SubscribePayload {
            query: "query".into(),
            variables: graphql_vars! {},
            operation_name: None,
            extensions: Default::default(),
        };

        assert_eq!(expected, payload);

        Ok(())
    }
}
