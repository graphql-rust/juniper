use std::{env, pin::Pin, time::Duration};

use actix_cors::Cors;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};

use juniper::{
    tests::fixtures::starwars::schema::{Character as _, Database, Query},
    DefaultScalarValue, EmptyMutation, FieldError, RootNode,
};
use juniper_actix::{graphql_handler, playground_handler, subscriptions::subscriptions_handler};
use juniper_graphql_ws::ConnectionConfig;

type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::<Database>::new(), Subscription)
}

async fn playground() -> Result<HttpResponse, Error> {
    playground_handler("/graphql", Some("/subscriptions")).await
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

struct RandomHuman {
    id: String,
    name: String,
}

// TODO: remove this when async interfaces are merged
#[juniper::graphql_object(Context = Database)]
impl RandomHuman {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

type RandomHumanStream =
    Pin<Box<dyn futures::Stream<Item = Result<RandomHuman, FieldError>> + Send>>;

#[juniper::graphql_subscription(Context = Database)]
impl Subscription {
    #[graphql(
        description = "A random humanoid creature in the Star Wars universe every 3 seconds. Second result will be an error."
    )]
    async fn random_human(context: &Database) -> RandomHumanStream {
        let mut counter = 0;

        let context = (*context).clone();

        use rand::{rngs::StdRng, Rng, SeedableRng};
        let mut rng = StdRng::from_entropy();

        let stream = tokio::time::interval(Duration::from_secs(3)).map(move |_| {
            counter += 1;

            if counter == 2 {
                Err(FieldError::new(
                    "some field error from handler",
                    Value::Scalar(DefaultScalarValue::String(
                        "some additional string".to_string(),
                    )),
                ))
            } else {
                let random_id = rng.gen_range(1000, 1005).to_string();
                let human = context.get_human(&random_id).unwrap().clone();

                Ok(RandomHuman {
                    id: human.id().to_owned(),
                    name: human.name().unwrap().to_owned(),
                })
            }
        });

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

    subscriptions_handler(req, stream, schema, config).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .data(schema())
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::new()
                    .allowed_methods(vec!["POST", "GET"])
                    .supports_credentials()
                    .max_age(3600)
                    .finish(),
            )
            .service(web::resource("/subscriptions").route(web::get().to(subscriptions)))
            .service(
                web::resource("/graphql")
                    .route(web::post().to(graphql))
                    .route(web::get().to(graphql)),
            )
            .service(web::resource("/playground").route(web::get().to(playground)))
            .default_service(web::route().to(|| {
                HttpResponse::Found()
                    .header("location", "/playground")
                    .finish()
            }))
    })
    .bind(format!("{}:{}", "127.0.0.1", 8080))?
    .run()
    .await
}
