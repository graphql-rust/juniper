extern crate iron;
extern crate mount;
extern crate logger;
extern crate rustc_serialize;
#[macro_use] extern crate juniper;

use mount::Mount;
use logger::Logger;
use iron::prelude::*;
use juniper::FieldResult;
use juniper::iron_handlers::{GraphQLHandler, GraphiQLHandler};

fn context_factory(_: &mut Request) -> () {
    ()
}

fn main() {
    let mut mount = Mount::new();

    let graphql_endpoint = GraphQLHandler::new(context_factory, Query { }, Mutation { });
    let graphiql_endpoint = GraphiQLHandler::new("/graphql");

    mount.mount("/graphiql", graphiql_endpoint);
    mount.mount("/graphql", graphql_endpoint);

    let (logger_before, logger_after) = Logger::new(None);

    let mut chain = Chain::new(mount);
    chain.link_before(logger_before);
    chain.link_after(logger_after);

    let host = "localhost:8080";
    println!("GraphQL server started on {}", host);
    Iron::new(chain).http(host).unwrap();
}

struct Query {}
struct Mutation {}

graphql_object!(Query: () as "Query" |&self| {
    field dummy() -> FieldResult<&str> {
        Ok("Dummy field")
    }

    field error() -> FieldResult<&str> {
        Err("Can't do it".to_owned())
    }
});

graphql_object!(<CtxT> Mutation: CtxT as "Mutation" |&self| {
    field print(value: String) -> FieldResult<String> {
        println!("Printing text according to mutation");
        println!("{}", value);
        Ok(value)
    }
});
