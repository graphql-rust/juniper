#![deny(warnings)]

use actix_cors::Cors;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::Stream;
use juniper::{DefaultScalarValue, FieldError, RootNode};
use juniper_actix::{
    graphiql_handler as gqli_handler, graphql_handler, playground_handler as play_handler,
    subscriptions::{graphql_subscriptions as sub_handler, EmptySubscriptionHandler},
};
use juniper_subscriptions::Coordinator;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{pin::Pin, time::Duration};
use tokio::sync::broadcast::{channel, Receiver, Sender};

type Schema = RootNode<'static, Query, Mutation, Subscription>;
type MyCoordinator =
    Coordinator<'static, Query, Mutation, Subscription, Context, DefaultScalarValue>;

struct ChatRoom {
    pub name: String,
    pub channel: (Sender<Msg>, Receiver<Msg>),
}

impl ChatRoom {
    pub fn new(name: String) -> Self {
        Self {
            name,
            channel: channel(16),
        }
    }
}

struct Context {
    pub chat_rooms: Arc<Mutex<HashMap<String, ChatRoom>>>,
}

impl Context {
    pub fn new(chat_rooms: Arc<Mutex<HashMap<String, ChatRoom>>>) -> Self {
        Self { chat_rooms }
    }
}

impl juniper::Context for Context {}

struct Query;

#[juniper::graphql_object(Context = Context)]
impl Query {
    pub fn chat_rooms(ctx: &Context) -> Vec<String> {
        ctx.chat_rooms
            .lock()
            .unwrap()
            .iter()
            .map(|(_, chat_room)| chat_room.name.clone())
            .collect()
    }
}

struct Mutation;

#[juniper::graphql_object(Context = Context)]
impl Mutation {
    pub fn send_message(room_name: String, msg: String, sender: String, ctx: &Context) -> bool {
        ctx.chat_rooms
            .lock()
            .unwrap()
            .get(&room_name)
            .map(|chat_room| {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::new(0, 0));
                chat_room
                    .channel
                    .0
                    .send(Msg {
                        sender,
                        value: msg,
                        date: format!("{}", now.as_secs()),
                    })
                    .is_ok()
            })
            .is_some()
    }
}

#[derive(juniper::GraphQLObject, Clone)]
struct Msg {
    pub sender: String,
    pub value: String,
    pub date: String,
}

type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;

type VecStringStream = Pin<Box<dyn Stream<Item = Result<Vec<Msg>, FieldError>> + Send>>;

struct Subscription;

#[juniper::graphql_subscription(Context = Context)]
impl Subscription {
    async fn hello_world() -> StringStream {
        let mut counter = 0;
        let stream = tokio::time::interval(Duration::from_secs(1)).map(move |_| {
            counter += 1;
            if counter % 2 == 0 {
                Ok(String::from("World!"))
            } else {
                Ok(String::from("Hello"))
            }
        });

        Box::pin(stream)
    }

    async fn chat_room(room_name: String, ctx: &Context) -> VecStringStream {
        let mut messages: Vec<Msg> = Vec::new();
        let channel_rx = {
            match ctx.chat_rooms.lock().unwrap().entry(room_name.clone()) {
                Entry::Occupied(o) => o.get().channel.0.subscribe(),
                Entry::Vacant(v) => v.insert(ChatRoom::new(room_name)).channel.0.subscribe(),
            }
        };
        let stream = channel_rx.map(move |msg| {
            let msg = msg?;
            messages.push(msg);
            Ok(messages.clone())
        });
        Box::pin(stream)
    }
}

fn schema() -> Schema {
    Schema::new(Query {}, Mutation {}, Subscription {})
}

async fn graphiql_handler() -> Result<HttpResponse, Error> {
    gqli_handler("/", Some("/subscriptions")).await
}
async fn playground_handler() -> Result<HttpResponse, Error> {
    play_handler("/", Some("/subscriptions")).await
}

async fn graphql(
    req: actix_web::HttpRequest,
    payload: actix_web::web::Payload,
    schema: web::Data<Schema>,
    chat_rooms: web::Data<Mutex<HashMap<String, ChatRoom>>>,
) -> Result<HttpResponse, Error> {
    let context = Context::new(chat_rooms.into_inner());
    graphql_handler(&schema, &context, req, payload).await
}

async fn graphql_subscriptions(
    coordinator: web::Data<MyCoordinator>,
    stream: web::Payload,
    req: HttpRequest,
    chat_rooms: web::Data<Mutex<HashMap<String, ChatRoom>>>,
) -> Result<HttpResponse, Error> {
    let context = Context::new(chat_rooms.into_inner());
    let handler: Option<EmptySubscriptionHandler> = None;
    sub_handler(
        coordinator,
        context,
        stream,
        req,
        handler,
        Some(Duration::from_secs(5)),
    )
    .await
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    ::std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let chat_rooms: Mutex<HashMap<String, ChatRoom>> = Mutex::new(HashMap::new());
    let chat_rooms = web::Data::new(chat_rooms);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(chat_rooms.clone())
            .data(schema())
            .data(juniper_subscriptions::Coordinator::new(schema()))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::new()
                    .allowed_methods(vec!["POST", "GET"])
                    .supports_credentials()
                    .max_age(3600)
                    .finish(),
            )
            .service(
                web::resource("/")
                    .route(web::post().to(graphql))
                    .route(web::get().to(graphql)),
            )
            .service(web::resource("/playground").route(web::get().to(playground_handler)))
            .service(web::resource("/graphiql").route(web::get().to(graphiql_handler)))
            .service(web::resource("/subscriptions").to(graphql_subscriptions))
    });
    server.bind("127.0.0.1:8080").unwrap().run().await
}
