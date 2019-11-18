use actix_web::{guard, middleware, web, App, HttpServer, Responder};

use juniper::{
    tests::{model::Database, schema::Query},
    EmptyMutation, RootNode,
};

use juniper_actix_web::{graphiql_source, playground_source, GraphQLRequest};

use std::sync::Arc;

type Schema = RootNode<'static, Query, EmptyMutation<Database>>;

struct Data {
    schema: Schema,
    context: Database,
}

fn data() -> Data {
    let schema = Schema::new(Query, EmptyMutation::<Database>::new());
    let context = Database::new();
    Data { schema, context }
}

fn graphql(st: web::Data<Arc<Data>>, data: GraphQLRequest) -> impl Responder {
    data.execute(&st.schema, &st.context)
}

fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=info");
    pretty_env_logger::init();

    let data = Arc::new(data());

    HttpServer::new(move || {
        App::new()
            .data(data.clone())
            .wrap(middleware::Logger::default())
            .service(
                web::resource("/graphql")
                    .guard(guard::Any(guard::Get()).or(guard::Post()))
                    .to(graphql),
            )
            .service(
                web::resource("/graphiql")
                    .route(web::get().to(|| graphiql_source("http://localhost:8088/graphql"))),
            )
            .service(
                web::resource("/playground")
                    .route(web::get().to(|| playground_source("http://localhost:8088/graphql"))),
            )
    })
    .bind("localhost:8088")?
    .run()
}
