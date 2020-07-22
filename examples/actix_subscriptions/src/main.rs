use std::sync::Arc;
use std::{env, pin::Pin, time::Duration};

use actix_cors::Cors;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::Stream;

use juniper::tests::fixtures::starwars::{model::Database, schema::Query};
use juniper::{DefaultScalarValue, EmptyMutation, FieldError, RootNode};
use juniper_actix::subscriptions::subscriptions_handler;
use juniper_actix::{graphql_handler, playground_handler};
use juniper_subscriptions::Coordinator;

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

#[juniper::graphql_subscription(Context = Database)]
impl Subscription {
    #[graphql(
        description = "A random humanoid creature in the Star Wars universe every 3 seconds. Second result will be an error."
    )]
    async fn random_human(
        context: &Database,
    ) -> Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>> {
        let mut counter = 0;

        // TODO: actually return a random human. But how to pass the context inside the closure?
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
                Ok(format!("test_{}", counter))
            }
        });

        Box::pin(stream)
    }
}

async fn subscriptions(
    req: HttpRequest,
    stream: web::Payload,
    coordinator: web::Data<
        Arc<
            Coordinator<
                'static,
                Query,
                EmptyMutation<Database>,
                Subscription,
                Database,
                DefaultScalarValue,
            >,
        >,
    >,
) -> Result<HttpResponse, Error> {
    let context = Database::new();
    subscriptions_handler(req, stream, Arc::clone(&coordinator), context).await
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let coordinator = Arc::new(Coordinator::new(schema()));

    HttpServer::new(move || {
        App::new()
            .data(schema())
            .data(Arc::clone(&coordinator))
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
