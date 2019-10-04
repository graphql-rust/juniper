// This module is used for testing implementantions
//_!! All changes should be reset before merging to master !!__

// For now, decided to do the following:
//   [✔] Implement field resolvers for subscriptions
//       [✔] Sync
//       [✔] Async
//   [ ] Group field resolvers to avoid code duplication
//       [ ] Sync
//       [ ] Async
//   [ ] after all that push changes to GitHub
//   [ ] Start implementing FragmentSpread and InlineFragment
//   [ ] consider doing unions, nested selections and other stuff
//   [ ] consider checking schema resolver and moving metadata to a different base type
//
// todo: discuss what to do with stream/iterator of null values

#![feature(decl_macro, proc_macro_hygiene)]

use rocket::{response::content, State};

use juniper::{
    parser::Spanning, Arguments, BoxFuture, DefaultScalarValue, Executor, FieldResult, RootNode,
    Selection, Value,
};
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

//todo: panics:
//             thread 'tokio-runtime-worker-1' panicked at 'Field __schema not found on type Mutation', juniper_rocket/examples/rocket_server.rs:22:1
//             thread 'tokio-runtime-worker-0' panicked at 'TODO.async: sender was dropped, error instead: Canceled', src/libcore/result.rs:1165:5
#[juniper::object(
    context = MyContext
)]
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

#[juniper::object(
    context = MyContext
)]
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

#[juniper::object(
    context = MyContext
)]
impl MySubscription {
    fn human(id: String) -> FieldResult<Human> {
        unreachable!()
    }

    fn nothuman(id: String) -> FieldResult<Human> {
        unreachable!()
    }
}

impl juniper::SubscriptionHandlerAsync<DefaultScalarValue> for MySubscription
where
    MySubscription: juniper::GraphQLType<DefaultScalarValue>,
    Self::Context: Send + Sync,
    Self::TypeInfo: Send + Sync,
{
    fn resolve_field_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        field_name: &'a str,
        arguments: &'a Arguments<DefaultScalarValue>,
        executor: &'a Executor<Self::Context, DefaultScalarValue>,
    ) -> BoxFuture<'a, juniper::SubscriptionResultAsync<DefaultScalarValue>> {
        println!("field name: {}", field_name);

        match field_name {
            "human" => {
                let ctx = executor.context();
                let x: std::pin::Pin<Box<dyn futures::Stream<Item = Value<DefaultScalarValue>>>> =
                    Box::pin(futures::stream::once(async {
                        Value::Scalar(DefaultScalarValue::String("human".to_owned()))
                    }));

                Box::pin(futures::future::ready(Ok(x)))
            }
            "nothuman" => {
                use futures::{channel::mpsc, future::FutureExt, stream::StreamExt};
                use std::{thread, time};

                let (tx1, rx1) = mpsc::unbounded();

                thread::spawn(move || {
                    tx1.unbounded_send(1).unwrap();
                    thread::sleep(time::Duration::from_millis(1000));
                    tx1.unbounded_send(2).unwrap();
                });

                let x: std::pin::Pin<Box<dyn futures::Stream<Item = Value<DefaultScalarValue>>>> =
                    Box::pin(
                        //                    futures::stream::repeat(Value::Scalar(DefaultScalarValue::String("not human".to_owned()))),
                        rx1.map(|x| Value::Scalar(DefaultScalarValue::Int(x))),
                    );
                Box::pin(futures::future::ready(Ok(x)))
            }
            _ => {
                panic!("field not found");
            }
        }
    }

    //    fn resolve_into_stream<'a>(
    //        &'a self,
    //        info: &'a Self::TypeInfo,
    //        selection_set: Option<&'a [Selection<DefaultScalarValue>]>,
    //        executor: &'a Executor<Self::Context, DefaultScalarValue>,
    //    ) -> BoxFuture<'a, juniper::ValuesStream> {
    //        let ctx = executor.context();
    //        let x: std::pin::Pin<Box<dyn futures::Stream<Item = Value<DefaultScalarValue>>>> = Box::pin(
    //            futures::stream::repeat(Value::Scalar(DefaultScalarValue::Int(ctx.0))),
    //        );
    //
    //        Box::pin(futures::future::ready(x))
    //    }
}

impl juniper::SubscriptionHandler<DefaultScalarValue> for MySubscription {
    fn resolve_field_into_iterator(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        arguments: &Arguments<DefaultScalarValue>,
        executor: &Executor<Self::Context, DefaultScalarValue>,
    ) -> Result<
            Value<juniper::ValuesIterator<DefaultScalarValue>>,
            juniper::FieldError<DefaultScalarValue>
        > {

        match field_name {
            "human" => {
                Ok(
                    Value::Scalar(
                        Box::new(std::iter::repeat(Value::Scalar(DefaultScalarValue::Int(22))))
                    )
                )
            }
            "nothuman" => {
                Ok(
                    Value::Scalar(
                        Box::new(std::iter::once(Value::Scalar(DefaultScalarValue::Int(32))))
                    )
                )
            }
            _ => {
                panic!("field not found");
            }
        }
    }


//    fn resolve_into_iterator<'a>(
//        &'a self,
//        info: &'a Self::TypeInfo,
//        selection_set: Option<&'a [Selection<DefaultScalarValue>]>,
//        executor: &'a Executor<Self::Context, DefaultScalarValue>,
//    ) -> juniper::ValuesIterator<DefaultScalarValue> {
//        println!("Selection: {:#?}", selection_set);
//        Box::new(std::iter::repeat(Value::Scalar(DefaultScalarValue::Int(32))))
//    }
}

#[derive(Debug)]
pub struct MyContext(i32);
impl juniper::Context for MyContext {}

type Schema = RootNode<'static, MyQuery, MyMutation, MySubscription, DefaultScalarValue>;

#[rocket::get("/")]
fn graphiql() -> content::Html<String> {
    juniper_rocket::graphiql_source("/graphql")
}

#[rocket::post("/graphql", data = "<request>")]
fn post_graphql_handler(
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    let mut is_async = false;
    is_async = true;

    if is_async {
        use futures::{compat::Compat, Future};
        use rocket::http::Status;
        use std::sync::mpsc::channel;

        let cloned_schema = Arc::new(schema);

        let (sender, receiver) = channel();
        let mut x = futures::executor::block_on(async move {
            let x = request
                .execute_async(&cloned_schema.clone(), &MyContext(1234))
                .await;
            sender.send(x);
        });

        let res = receiver.recv().unwrap();
        res
    } else {
        request.execute(&schema, &MyContext(1234))
    }

    //    GraphQLResponse(Status {
    //        code: 200,
    //        reason: "because"
    //    }, "it compiles".to_string());
}

fn main() {
    rocket::ignite()
        .manage(Schema::new(MyQuery, MyMutation, MySubscription))
        .mount("/", rocket::routes![graphiql, post_graphql_handler])
        .launch();
}
