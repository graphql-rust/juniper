//! `juniper_warp` subscriptions implementation.
//! Cannot be merged to `juniper_warp` yet as GraphQL over WS [1]
//! is not fully supported in current implementation.
//!
//! [1]: https://github.com/apollographql/subscriptions-transport-ws/blob/master/PROTOCOL.md


use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};
use std::sync::Arc;

use futures::{channel::mpsc, stream::StreamExt as _, Future};
use futures::future::FutureExt as _;
use serde::Deserialize;

use serde::Serialize;

use warp::ws::Message;

use juniper::http::GraphQLRequest;
use juniper::{InputValue, ScalarValue};
use juniper_subscriptions::Coordinator;

/// Listen to `websocket`'s messages and do one of the following:
///  - execute subscription and return values from stream
///  - stop stream and close ws connection

pub fn graphql_subscriptions_async<Query, Mutation, Subscription, Context, S>(
    websocket: warp::ws::WebSocket,
    coordinator: Arc<Coordinator<'static, Query, Mutation, Subscription, Context, S>>,
    context: Context,
) -> impl Future<Output = ()> + Send
    where
        S: ScalarValue + Send + Sync + 'static,
        Context: Clone + Send + Sync + 'static,
        Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Query::TypeInfo: Send + Sync,
        Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
        Mutation::TypeInfo: Send + Sync,
        Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
        Subscription::TypeInfo: Send + Sync,
{
    let (sink_tx, sink_rx) = websocket.split();
    let (ws_tx, ws_rx) = mpsc::unbounded();
    tokio::task::spawn(
        ws_rx
            .take_while(|x: &Option<_>| {
                // keep this stream until `None` is received
                let keep_going = x.is_some();
                futures::future::ready(keep_going)
            })
            .map(|x| x.unwrap())
            .forward(sink_tx)
            .map(|result| {
                if let Err(e) = result {
                    println!("websocket send error: {}", e);
                }
            }),
    );

    let coordinator = Arc::new(coordinator);
    let context = Arc::new(context);
    let got_close_signal = Arc::new(AtomicBool::new(false));

    sink_rx.for_each(move |msg| {
        if msg.is_err() {
            println!("message is error: {:?}", msg);
            return futures::future::ready(());
        }
        let msg = msg.unwrap();

        if msg.is_close() {
            return futures::future::ready(());
        }
        let context = context.clone();
        let got_close_signal = got_close_signal.clone();

        let msg = msg.to_str().unwrap();

        let coordinator = coordinator.clone();
        let context = context.clone();
        let request: WsPayload<S> = serde_json::from_str(msg).unwrap();

        match request.type_name.as_str() {
            "connection_init" => {}
            "start" => {
                let ws_tx = ws_tx.clone();

                tokio::task::spawn(async move {
                    let payload = request.payload.expect("could not deserialize payload");
                    let request_id = request.id.unwrap_or("1".to_owned());

                    let graphql_request =
                        GraphQLRequest::<S>::new(payload.query.unwrap(), None, payload.variables);

                    use juniper::SubscriptionCoordinator as _;

                    let response_stream = match coordinator
                        .subscribe(&graphql_request, &context)
                        .await {
                        Ok(s) => s,
                        Err(err) => {
                            let _ = ws_tx.unbounded_send(Some(Ok(Message::text(format!(
                                r#"{{"type":"error","id":"{}","payload":{}}}"#,
                                request_id,
                                serde_json::ser::to_string(&err).unwrap()
                            )))));

                            let close_text = format!(
                                r#"{{"type":"complete","id":"{}","payload":null}}"#,
                                request_id
                            );

                            // send message that we are closing channel
                            let _ =
                                ws_tx.unbounded_send(Some(Ok(Message::text(close_text.clone()))));

                            // close channel
                            let _ = ws_tx.unbounded_send(None);

                            return;
                        }
                    };

                    response_stream
                        .take_while(move |response| {
                            let request_id = request_id.clone();
                            let closed = got_close_signal.load(Ordering::Relaxed);
                            if closed {
                                let close_text = format!(
                                    r#"{{"type":"complete","id":"{}","payload":null}}"#,
                                    request_id
                                );

                                //  send message that we are closing channel
                                let _ = ws_tx.unbounded_send(Some(Ok(Message::text(close_text))));

                                // close channel
                                let _ = ws_tx.unbounded_send(None);
                            } else {
                                let mut response_text = serde_json::to_string(&response).unwrap();
                                response_text = format!(
                                    r#"{{"type":"data","id":"{}","payload":{} }}"#,
                                    request_id, response_text
                                );

                                let _ =
                                    ws_tx.unbounded_send(Some(Ok(Message::text(response_text))));
                            }
                            async move { !closed }
                        })
                        .for_each(|_| async {})
                        .await;
                });
            }
            "stop" => {
                got_close_signal.store(true, Ordering::Relaxed);
            }
            _ => panic!("unknown type"),
        }

        futures::future::ready(())
    })
}


#[derive(Deserialize)]
#[serde(bound = "GraphQLPayload<S>: Deserialize<'de>")]
struct WsPayload<S>
    where
        S: ScalarValue + Send + Sync + 'static,
{
    id: Option<String>,
    #[serde(rename(deserialize = "type"))]
    type_name: String,
    payload: Option<GraphQLPayload<S>>,
}


#[derive(Debug, Deserialize)]
#[serde(bound = "InputValue<S>: Deserialize<'de>")]
struct GraphQLPayload<S>
    where
        S: ScalarValue + Send + Sync + 'static,
{
    variables: Option<InputValue<S>>,
    extensions: Option<HashMap<String, String>>,
    #[serde(rename(deserialize = "operationName"))]
    operaton_name: Option<String>,
    query: Option<String>,
}


#[derive(Serialize)]
struct Output {
    data: String,
    variables: String,
}