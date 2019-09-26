// This file is used for testing implementantions
//_!! All changes should be reset before merging to master !!__

#![feature(decl_macro, proc_macro_hygiene)]

use rocket::{response::content, State};

use juniper::{RootNode, FieldResult, Selection, Executor, BoxFuture, Value};
use juniper_rocket::GraphQLResponse;
use std::sync::Arc;

#[derive(juniper::GraphQLObject)]
#[graphql(description = "A humanoid creature in the Star Wars universe")]
struct Human {
    id: String,
    name: String,
    home_planet: String,
}

struct MyQuery;

#[juniper::object]
impl MyQuery {
    fn human(id: String) -> FieldResult<Human> {
        let human = Human {
            id: "query".to_string(),
            name: "Query Human".to_string(),
            home_planet: "Query Human Planet".to_string(),
        };
        Ok(human)
    }
}

struct MyMutation;

#[juniper::object]
impl MyMutation {
    fn human(id: String) -> FieldResult<Human> {
        let human = Human {
            id: "mutation".to_string(),
            name: "Mutation Human Name".to_string(),
            home_planet: "Mutation Human Planet".to_string(),
        };
        Ok(human)
    }
}

struct MySubscription;

#[juniper::object]
impl MySubscription {
    fn human(id: String) -> FieldResult<Human> {
        let human = Human {
            id: "subscription".to_string(),
            name: "Subscription Human Name".to_string(),
            home_planet: "Subscription Human Home Planet".to_string(),
        };
        Ok(human)
    }
}

impl<S> juniper::SubscriptionHandlerAsync<S> for MySubscription
where
    MySubscription: juniper::GraphQLType<S>,
    Self::Context: Send + Sync,
    Self::TypeInfo: Send + Sync,
    S: juniper::ScalarValue + Send + Sync,
    for<'b> &'b S: juniper::ScalarRefValue<'b>,
{
    fn resolve_into_stream_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, std::pin::Pin<Box<dyn futures::Stream<Item = Value<S>>>>>
    {
        Box::pin(
            futures::future::ready(
                Box::pin(
                    futures::stream::once(futures::future::ready(
                        Value::<S>::Null
                    ))
                )
            )
        )
    }
}

type Schema = RootNode<'static, MyQuery, MyMutation, MySubscription>;

#[rocket::get("/")]
fn graphiql() -> content::Html<String> {
    juniper_rocket::graphiql_source("/graphql")
}

#[rocket::get("/graphql?<request>")]
fn get_graphql_handler(
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &())
}

#[rocket::post("/graphql", data = "<request>")]
fn post_graphql_handler(
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    use futures::Future;
    use futures::compat::Compat;
    use rocket::http::Status;
    use std::sync::mpsc::channel;
//    use futures1::Future;

    let cloned_schema = Arc::new(schema);

    let (sender, receiver) = channel();

    let mut x = futures::executor::block_on(
        async move {
            let x = request.execute_async(&cloned_schema.clone(), &()).await;
            sender.send(x);
        }
    );

    let res = receiver.recv().unwrap();

//    GraphQLResponse(Status {
//        code: 200,
//        reason: "because"
//    }, "it compiles".to_string());
    res
}

fn main() {
    rocket::ignite()
        .manage(Schema::new(MyQuery, MyMutation, MySubscription))
        .mount(
            "/",
            rocket::routes![graphiql, get_graphql_handler, post_graphql_handler],
        )
        .launch();
}
