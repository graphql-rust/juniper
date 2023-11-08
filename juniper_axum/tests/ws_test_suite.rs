use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use axum::{routing::get, Extension, Router};
use futures::{SinkExt, StreamExt};
use juniper::{
    http::tests::{graphql_transport_ws, graphql_ws, WsIntegration, WsIntegrationMessage},
    tests::fixtures::starwars::schema::{Database, Query, Subscription},
    EmptyMutation, LocalBoxFuture, RootNode,
};
use juniper_axum::subscriptions;
use juniper_graphql_ws::ConnectionConfig;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;

#[derive(Clone)]
struct TestApp(Router);

impl TestApp {
    fn new(protocol: &'static str) -> Self {
        let schema = Schema::new(Query, EmptyMutation::new(), Subscription);

        let mut router = Router::new();
        router = if protocol == "graphql-ws" {
            router.route(
                "/subscriptions",
                get(subscriptions::graphql_ws::<Arc<Schema>>(
                    ConnectionConfig::new(Database::new()),
                )),
            )
        } else {
            router.route(
                "/subscriptions",
                get(subscriptions::graphql_transport_ws::<Arc<Schema>>(
                    ConnectionConfig::new(Database::new()),
                )),
            )
        };
        router = router.layer(Extension(Arc::new(schema)));

        Self(router)
    }

    async fn run(self, messages: Vec<WsIntegrationMessage>) -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind("0.0.0.0:0".parse::<SocketAddr>().unwrap()).unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::Server::from_tcp(listener)
                .unwrap()
                .serve(self.0.into_make_service())
                .await
                .unwrap();
        });

        let (mut websocket, _) = connect_async(format!("ws://{}/subscriptions", addr))
            .await
            .unwrap();

        for msg in messages {
            process_message(&mut websocket, msg).await?;
        }

        Ok(())
    }
}

impl WsIntegration for TestApp {
    fn run(
        &self,
        messages: Vec<WsIntegrationMessage>,
    ) -> LocalBoxFuture<Result<(), anyhow::Error>> {
        Box::pin(self.clone().run(messages))
    }
}

async fn process_message(
    mut websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    message: WsIntegrationMessage,
) -> Result<(), anyhow::Error> {
    match message {
        WsIntegrationMessage::Send(msg) => websocket.send(Message::Text(msg.to_string())).await
            .map_err(|e| anyhow!("Could not send message: {e}"))
            .map(drop),
        WsIntegrationMessage::Expect(expected, timeout) => {
            receive_message_from_socket_and_test(&mut websocket, &expected, timeout).await
        }
    }
}



async fn receive_message_from_socket_and_test(
    websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    expected: &String,
    timeout: u64,
) -> Result<(), anyhow::Error> {
    let message = tokio::time::timeout(Duration::from_millis(timeout), websocket.next())
        .await
        .map_err(|e| anyhow!("Timed out receiving message. Elapsed: {e}"))?;

    match message {
        None => Err(anyhow!("No Message received")),
        Some(Err(e)) => Err(anyhow!("Websocket error: {:?}", e)),
        Some(Ok(message)) => equals_received_text_message(&expected, message),
    }
}

fn equals_received_text_message(expected: &String, message: Message) -> Result<(), anyhow::Error> {
    match message {
        Message::Text(received) => is_the_same(&expected, &received),
        Message::Binary(_) => Err(anyhow!("Received binary message, but expected text")),
        Message::Ping(_) => Err(anyhow!("Received ping message, but expected text")),
        Message::Pong(_) => Err(anyhow!("Received pong message, but expected text")),
        Message::Close(_) => Err(anyhow!("Received close message, but expected text")),
        Message::Frame(_) => Err(anyhow!("Received frame message, but expected text")),
    }
}

/// Check if expected == received by transforming both to a JSON value
fn is_the_same(expected: &String, received: &String) -> Result<(), anyhow::Error> {
    let expected: Value =
        serde_json::from_str(&expected).map_err(|e| anyhow::anyhow!("Serde error: {e:?}"))?;

    let received: Value =
        serde_json::from_str(&received).map_err(|e| anyhow::anyhow!("Serde error: {e:?}"))?;

    if received != expected {
        return Err(anyhow!(
            "Expected: {:?}\nReceived: {:?}",
            expected,
            received
        ));
    }

    Ok(())
}

#[tokio::test]
async fn test_graphql_ws_integration() {
    let app = TestApp::new("graphql-ws");
    graphql_ws::run_test_suite(&app).await;
}

#[tokio::test]
async fn test_graphql_transport_integration() {
    let app = TestApp::new("graphql-transport-ws");
    graphql_transport_ws::run_test_suite(&app).await;
}
