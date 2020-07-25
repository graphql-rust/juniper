use juniper::{ScalarValue, Variables};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(bound(deserialize = "S: ScalarValue"))]
#[serde(rename_all = "camelCase")]
pub struct StartPayload<S: ScalarValue> {
    pub query: String,
    #[serde(default)]
    pub variables: Variables<S>,
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
        #[serde(default)]
        payload: Variables<S>,
    },
    /// Start messages are used to execute a GraphQL operation.
    Start {
        id: String,
        payload: StartPayload<S>,
    },
    /// Stop messages are used to unsubscribe from a subscription.
    Stop { id: String },
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
}
