// This module is used for testing implementantions
//_!! All changes should be reset before merging to master !!__

#![feature(decl_macro, proc_macro_hygiene)]

use rocket::{response::content, State};

use futures::StreamExt;
use juniper::{
    parser::Spanning, Arguments, BoxFuture, DefaultScalarValue, Executor, FieldError, FieldResult,
    GraphQLType, RootNode, Selection, Value, ValuesIterator, ValuesStream,
};
use juniper_rocket::GraphQLResponse;
use std::{env::args, sync::Arc};

#[derive(juniper::GraphQLObject)]
#[graphql(description = "A humanoid creature in the Star Wars universe")]
#[derive(Clone)]
struct Human {
    id: String,
    name: String,
    home_planet: String,
}

struct MyQuery;

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

#[juniper::subscription(
    context = MyContext
)]
impl MySubscription {
    fn human(id: String) -> Human {
        let iter = Box::new(std::iter::repeat(Human {
            id: "subscription id".to_string(),
            name: "subscription name".to_string(),
            home_planet: "subscription planet".to_string(),
        }));
        Ok(iter)
    }

    async fn human() -> Human {
        Ok(Box::pin(futures::stream::repeat(Human {
            id: "stream human id".to_string(),
            name: "stream human name".to_string(),
            home_planet: "stream human home planet".to_string(),
        })))
    }
}

//#[juniper::object(
//    context = MyContext
//)]
//impl MySubscription {
//    fn human(id: String) -> FieldResult<Human> {
//        unreachable!()
//    }
//
//    fn nothuman(id: String) -> FieldResult<Human> {
//        unreachable!()
//    }
//}

//impl juniper::SubscriptionHandlerAsync<DefaultScalarValue> for MySubscription
//where
//    MySubscription: juniper::GraphQLType<DefaultScalarValue>,
//    Self::Context: Send + Sync + Clone,
//    Self::TypeInfo: Send + Sync,
//{
//    fn resolve_field_async<'a>(
//        &'a self,
//        info: &'a Self::TypeInfo,
//        field_name: &'a str,
//        arguments: Arguments<'a, DefaultScalarValue>,
//        executor: Executor<'a, Self::Context, DefaultScalarValue>,
//    ) -> BoxFuture<'a, juniper::SubscriptionResultAsync<'a, DefaultScalarValue>> {
//        use futures::future;
//        match field_name {
//            "human" => {
//                futures::FutureExt::boxed(async move {
//                    let id = arguments.get::<String>("id").expect(
//                        "Internal error: missing argument id - validation must have failed",
//                    );
//
//                    let res = {
//                        println!("!!!!! got id: {:?} !!!!", id);
//                        (move || {
//                            Box::pin(futures::stream::repeat(Human {
//                                id: "stream human id".to_string(),
//                                name: "stream human name".to_string(),
//                                home_planet: "stream human home planet".to_string(),
//                            }))
//                        })()
//                    };
//
//                    let f = res.then(move |res| {
//                        let res2: FieldResult<_, DefaultScalarValue> =
//                            juniper::IntoResolvable::into(res, executor.context());
//
//                        let ex = executor.clone();
//                        async move {
//                            match res2 {
//                                Ok(Some((ctx, r))) => {
//                                    let sub = ex.replaced_context(ctx);
//                                    match sub.resolve_with_ctx_async(&(), &r).await {
//                                        Ok(v) => v,
//                                        Err(_) => juniper::Value::Null,
//                                    }
//                                }
//                                Ok(None) => juniper::Value::null(),
//                                Err(e) => juniper::Value::Null,
//                            }
//                        }
//                    });
//                    Ok(Value::Scalar::<juniper::ValuesStream>(Box::pin(f)))
//                })
//            }
//            _ => {
//                panic!("field not found");
//            }
//        }
//    }
//}

//impl juniper::SubscriptionHandler<DefaultScalarValue> for MySubscription {
//    fn resolve_field_into_iterator<'r>(
//        &self,
//        info: &Self::TypeInfo,
//        field_name: &str,
//        arguments: &Arguments<DefaultScalarValue>,
//        executor: Executor<'r, Self::Context, DefaultScalarValue>,
//    ) -> juniper::SubscriptionResult<'r, DefaultScalarValue> {
//        match field_name {
//            "human" => {
//                let res = {
//                    (move || -> FieldResult<Box<dyn Iterator<Item = Human>>, DefaultScalarValue> {
//                        let iter = Box::new(std::iter::repeat(
//                            //                Value::Scalar(DefaultScalarValue::Int(22))
//                            Human {
//                                id: "subscription id".to_string(),
//                                name: "subscription name".to_string(),
//                                home_planet: "subscription planet".to_string(),
//                            },
//                        ));
//
//                        Ok(iter)
//                    })()
//                }?;
//                let iter = res.map(move |res| {
//                    juniper::IntoResolvable::into(
//                        res,
//                        executor.context(),
//                    )
//                    .and_then(|res| match res {
//                        Some((ctx, r)) => {
//                            let resolve_res =
//                                executor.replaced_context(ctx).resolve_with_ctx(&(), &r);
//                            resolve_res
//                        }
//                        None => Ok(Value::null()),
//                    })
//                    .unwrap_or_else(|_| Value::Null)
//                });
//                Ok(Value::Scalar(Box::new(iter)))
//                //                iter.take(5).for_each(|x| println!("About to send result: {:?}", x));
//                //                Ok(Value::Null)
//            }
//            "nothuman" => {
//                unimplemented!()
//                //                Ok(Value::Scalar(Box::new(std::iter::once(Value::Scalar(
//                //                    DefaultScalarValue::Int(32),
//                //                )))))
//            }
//            _ => {
//                panic!("field not found");
//            }
//        }
//    }
//
//    //    fn resolve_into_iterator<'a>(
//    //        &'a self,
//    //        info: &'a Self::TypeInfo,
//    //        selection_set: Option<&'a [Selection<DefaultScalarValue>]>,
//    //        executor: &'a Executor<Self::Context, DefaultScalarValue>,
//    //    ) -> juniper::ValuesIterator<DefaultScalarValue> {
//    //        println!("Selection: {:#?}", selection_set);
//    //        Box::new(std::iter::repeat(Value::Scalar(DefaultScalarValue::Int(32))))
//    //    }
//}

#[derive(Debug, Clone)]
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
    //    is_async = true;

    //        if is_async {
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
    //        } else {
    //            request.execute(&schema, &MyContext(1234))
    //        }

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
