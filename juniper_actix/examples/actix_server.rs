#![deny(warnings)]

use std::env;

use actix_cors::Cors;
use actix_web::{http::header, middleware, web, App, Error, HttpResponse, HttpServer};
use juniper::{
    tests::fixtures::starwars::schema::{Database, Query},
    EmptyMutation, EmptySubscription, RootNode,
};
use juniper_actix::{
    graphiql_handler as gqli_handler, graphql_handler, playground_handler as play_handler,
};

type Schema = RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;

fn schema() -> Schema {
    Schema::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    )
}

async fn graphiql_handler() -> Result<HttpResponse, Error> {
    gqli_handler("/", None).await
}
async fn playground_handler() -> Result<HttpResponse, Error> {
    play_handler("/", None).await
}
async fn graphql(
    req: actix_web::HttpRequest,
    payload: actix_web::web::Payload,
    schema: web::Data<Schema>,
) -> Result<HttpResponse, Error> {
    let context = Database::new();
    graphql_handler(&schema, &context, req, payload).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let server = HttpServer::new(move || {
        App::new()
            .data(schema())
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::default()
                    .allowed_origin("http://127.0.0.1:8080")
                    .allowed_methods(vec!["POST", "GET"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600),
            )
            .service(
                web::resource("/")
                    .route(web::post().to(graphql))
                    .route(web::get().to(graphql)),
            )
            .service(web::resource("/playground").route(web::get().to(playground_handler)))
            .service(web::resource("/graphiql").route(web::get().to(graphiql_handler)))
    });
    server.bind("127.0.0.1:8080").unwrap().run().await
}
