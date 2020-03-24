#![deny(warnings)]

extern crate log;
use actix_cors::Cors;
use actix_web::{middleware, web, App, Error, HttpResponse, HttpServer};
use juniper::{
    tests::{model::Database, schema::Query},
    DefaultScalarValue, EmptyMutation, EmptySubscription, RootNode,
};
use juniper_actix::{
    get_graphql_handler, graphiql_handler as gqli_handler, playground_handler as play_handler,
    post_graphql_handler, GraphQLBatchRequest,
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
    gqli_handler("/").await
}
async fn playground_handler() -> Result<HttpResponse, Error> {
    play_handler("/", None).await
}
async fn graphql(
    req: web::Json<GraphQLBatchRequest<DefaultScalarValue>>,
    schema: web::Data<Schema>,
) -> Result<HttpResponse, Error> {
    let context = Database::new();
    post_graphql_handler(&schema, &context, req).await
}

async fn graphql_get(
    req: web::Query<GraphQLBatchRequest<DefaultScalarValue>>,
    schema: web::Data<Schema>,
) -> Result<HttpResponse, Error> {
    let context = Database::new();
    get_graphql_handler(&schema, &context, req).await
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    ::std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let server = HttpServer::new(move || {
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
            .service(
                web::resource("/")
                    .route(web::post().to(graphql))
                    .route(web::get().to(graphql_get)),
            )
            .service(web::resource("/playground").route(web::get().to(playground_handler)))
            .service(web::resource("/graphiql").route(web::get().to(graphiql_handler)))
    });
    server.bind("127.0.0.1:8080").unwrap().run().await
}
