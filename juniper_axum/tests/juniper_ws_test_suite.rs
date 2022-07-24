use anyhow::anyhow;
use axum::{extract::WebSocketUpgrade, response::Response, routing::get, Extension, Router};
use futures::{SinkExt, StreamExt};
use juniper::{
    http::tests::{run_ws_test_suite, WsIntegration, WsIntegrationMessage},
    tests::fixtures::starwars::schema::{Database, Query, Subscription},
    EmptyMutation, LocalBoxFuture, RootNode,
};
use juniper_axum::subscriptions::handle_graphql_socket;
use serde_json::Value;
use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

/// The app we want to test
#[derive(Clone)]
struct AxumApp(Router);

type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;

/// Create a new axum app to test
fn test_app() -> AxumApp {
    let schema = Schema::new(Query, EmptyMutation::<Database>::new(), Subscription);

    let context = Database::new();

    let router = Router::new()
        .route("/subscriptions", get(juniper_subscriptions))
        .layer(Extension(Arc::from(schema)))
        .layer(Extension(context));

    AxumApp(router)
}

/// Axum handler for websockets
pub async fn juniper_subscriptions(
    Extension(schema): Extension<Arc<Schema>>,
    Extension(context): Extension<Database>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(|socket| handle_graphql_socket(socket, schema, context))
}

/// Test a vector of WsIntegrationMessages by
/// - sending messages to server
/// - receiving messages from server
///
/// This function will result in an error if
/// - Message couldn't be send
/// - receiving the message timed out
/// - an error happened during receiving
/// - the received message was not a text message
/// - if expected_message != received_message
async fn run_async_tests(
    app: AxumApp,
    messages: Vec<WsIntegrationMessage>,
) -> Result<(), anyhow::Error> {
    // Spawn test server
    let listener = TcpListener::bind("0.0.0.0:0".parse::<SocketAddr>().unwrap()).unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app.0.into_make_service())
            .await
            .unwrap();
    });

    // Connect to server with tokio-tungstenite library
    let (mut websocket, _) = connect_async(format!("ws://{}/subscriptions", addr))
        .await
        .unwrap();

    // Send and receive messages
    for message in messages {
        process_message(&mut websocket, message).await?;
    }

    Ok(())
}

/// Send or receive an message to the server
async fn process_message(
    mut websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    message: WsIntegrationMessage,
) -> Result<(), anyhow::Error> {
    match message {
        WsIntegrationMessage::Send(mes) => send_message(&mut websocket, mes).await,
        WsIntegrationMessage::Expect(expected, timeout) => {
            receive_message_from_socket_and_test(&mut websocket, &expected, timeout).await
        }
    }
}

async fn send_message(
    websocket: &mut &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    mes: String,
) -> Result<(), anyhow::Error> {
    match websocket.send(Message::Text(mes)).await {
        Ok(_) => Ok(()),
        Err(err) => Err(anyhow!("Could not send message: {:?}", err)),
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

/// Implement WsIntegration trait so we can automize our tests
impl WsIntegration for AxumApp {
    fn run(
        &self,
        messages: Vec<WsIntegrationMessage>,
    ) -> LocalBoxFuture<Result<(), anyhow::Error>> {
        let app = self.clone();
        Box::pin(run_async_tests(app, messages))
    }
}

#[tokio::test]
async fn juniper_ws_test_suite() {
    let app = test_app();
    run_ws_test_suite(&app).await;
}
