use std::{env, pin::Pin, time::Duration};

use actix_cors::Cors;
use actix_web::{
    http::header,
    middleware,
    web::{self, Data},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};

use juniper::{
    graphql_subscription, graphql_value,
    tests::fixtures::starwars::schema::{Database, Query},
    EmptyMutation, FieldError, GraphQLObject, RootNode,
};
use juniper_actix::{
    graphiql_handler, graphql_handler, playground_handler,
    subscriptions::{graphql_transport_ws_handler, graphql_ws_handler},
};
use juniper_graphql_ws::ConnectionConfig;

type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::<Database>::new(), Subscription)
}

async fn playground() -> Result<HttpResponse, Error> {
    playground_handler("/graphql", Some("/legacy-subscriptions")).await
}

async fn graphiql() -> Result<HttpResponse, Error> {
    graphiql_handler("/graphql", Some("/subscriptions")).await
}

async fn graphql(
    req: actix_web::HttpRequest,
    payload: actix_web::web::Payload,
    schema: web::Data<Schema>,
) -> Result<HttpResponse, Error> {
    let context = Database::new();
    graphql_handler(&schema, &context, req, payload).await
}

struct Subscription;

#[derive(GraphQLObject)]
struct RandomHuman {
    id: String,
    name: String,
}

type RandomHumanStream =
    Pin<Box<dyn futures::Stream<Item = Result<RandomHuman, FieldError>> + Send>>;

#[graphql_subscription(context = Database)]
impl Subscription {
    #[graphql(
        description = "A random humanoid creature in the Star Wars universe every 3 seconds. Second result will be an error."
    )]
    async fn random_human(context: &Database) -> RandomHumanStream {
        let mut counter = 0;

        let context = (*context).clone();

        use rand::{rngs::StdRng, Rng, SeedableRng};
        let mut rng = StdRng::from_entropy();
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let stream = async_stream::stream! {
            counter += 1;
            loop {
                interval.tick().await;
                if counter == 2 {
                    yield Err(FieldError::new(
                        "some field error from handler",
                        graphql_value!("some additional string"),
                    ))
                } else {
                    let random_id = rng.gen_range(1000..1005).to_string();
                    let human = context.get_human(&random_id).unwrap().clone();

                    yield Ok(RandomHuman {
                        id: human.id().into(),
                        name: human.name().unwrap().into(),
                    })
                }
            }
        };

        Box::pin(stream)
    }
}

async fn subscriptions(
    req: HttpRequest,
    stream: web::Payload,
    schema: web::Data<Schema>,
) -> Result<HttpResponse, Error> {
    let context = Database::new();
    let schema = schema.into_inner();
    let config = ConnectionConfig::new(context);
    // set the keep alive interval to 15 secs so that it doesn't timeout in playground
    // playground has a hard-coded timeout set to 20 secs
    let config = config.with_keep_alive_interval(Duration::from_secs(15));

    graphql_transport_ws_handler(req, stream, schema, config).await
}

async fn legacy_subscriptions(
    req: HttpRequest,
    stream: web::Payload,
    schema: web::Data<Schema>,
) -> Result<HttpResponse, Error> {
    let context = Database::new();
    let schema = schema.into_inner();
    let config = ConnectionConfig::new(context);
    // set the keep alive interval to 15 secs so that it doesn't timeout in playground
    // playground has a hard-coded timeout set to 20 secs
    let config = config.with_keep_alive_interval(Duration::from_secs(15));

    graphql_ws_handler(req, stream, schema, config).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(schema()))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["POST", "GET"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(web::resource("/legacy-subscriptions").route(web::get().to(legacy_subscriptions)))
            .service(web::resource("/subscriptions").route(web::get().to(subscriptions)))
            .service(
                web::resource("/graphql")
                    .route(web::post().to(graphql))
                    .route(web::get().to(graphql)),
            )
            .service(web::resource("/playground").route(web::get().to(playground)))
            .service(web::resource("/graphiql").route(web::get().to(graphiql)))
            .default_service(web::to(|| async {
                HttpResponse::Found()
                    .append_header((header::LOCATION, "/playground"))
                    .finish()
            }))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
