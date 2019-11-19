//!
//! This example demonstrates asynchronous subscriptions usage with warp.
//! NOTE: this uses tokio 0.2.alpha
//!

use std::{pin::Pin, sync::Arc, time::Duration};

use futures::{Future, FutureExt as _, Stream};
use tokio::timer::Interval;
use warp::{http::Response, Filter};

use juniper::{EmptyMutation, FieldError, RootNode, DefaultScalarValue, Value};
use juniper_warp::playground_filter;

#[derive(Clone)]
struct Context {}

impl juniper::Context for Context {}

#[derive(juniper::GraphQLEnum, Clone, Copy)]
/// UserKind sample docs
enum UserKind {
    Admin,
    User,
    Guest,
}


/// Sample User representation
struct User {
    id: i32,
    kind: UserKind,
    name: String,
}

#[juniper::object(Context = Context)]
impl User {
    fn id(&self) -> i32 {
        self.id
    }

    fn kind(&self) -> UserKind {
        self.kind
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn friends(&self) -> Vec<User> {
        if self.id == 1 {
            return vec![
                User {
                    id: 11,
                    kind: UserKind::User,
                    name: "user11".into(),
                },
                User {
                    id: 12,
                    kind: UserKind::Admin,
                    name: "user12".into(),
                },
                User {
                    id: 13,
                    kind: UserKind::Guest,
                    name: "user13".into(),
                },
            ];
        } else if self.id == 2 {
            return vec![User {
                id: 21,
                kind: UserKind::User,
                name: "user21".into(),
            }];
        } else if self.id == 3 {
            return vec![
                User {
                    id: 31,
                    kind: UserKind::User,
                    name: "user31".into(),
                },
                User {
                    id: 32,
                    kind: UserKind::Guest,
                    name: "user32".into(),
                },
            ];
        } else {
            return vec![];
        }
    }
}

struct Query;

#[juniper::object(Context = Context)]
impl Query {
    async fn users(id: i32) -> Vec<User> {
        vec![User {
            id: id,
            kind: UserKind::Admin,
            name: "user1".into(),
        }]
    }

    /// Fetch a URL and return the response body text.
    async fn request(url: String) -> Result<String, FieldError> {
        use futures::{
            compat::{Future01CompatExt, Stream01CompatExt},
            stream::TryStreamExt,
        };

        let res = reqwest::r#async::Client::new()
            .get(&url)
            .send()
            .compat()
            .await?;

        let body_raw = res.into_body().compat().try_concat().await?;
        let body = std::str::from_utf8(&body_raw).unwrap_or("invalid utf8");
        Ok(body.to_string())
    }
}

struct Subscription;

#[juniper::subscription(Context = Context)]
impl Subscription {
    async fn users() -> Result<Pin<Box<futures::Stream<Item = User> + Send>>, FieldError> {
        Err(FieldError::new(
            "some field error from handler",
            Value::Scalar(
                DefaultScalarValue::String("some additional string".to_string())
            )
        ))
    }
}

//impl juniper::GraphQLType<juniper::DefaultScalarValue> for Subscription {
//
//    type Context = Context;
//    type TypeInfo = ();
//
//    fn name(_: &Self::TypeInfo) -> Option<&str> {
//        Some("Subscription")
//    }
//
//    fn meta<'r>(
//        info: &Self::TypeInfo,
//        registry: &mut juniper::Registry<'r, juniper::DefaultScalarValue>,
//    ) -> juniper::meta::MetaType<'r, juniper::DefaultScalarValue>
//        where
//            juniper::DefaultScalarValue: 'r,
//            for<'z> &'z juniper::DefaultScalarValue: juniper::ScalarRefValue<'z>,
//    {
//        trait GraphQLTrait<T> {
//            type Item: juniper::GraphQLType<juniper::DefaultScalarValue>;
//        }
//        impl<T, Type> GraphQLTrait<T> for Type
//            where
//                Type: std::iter::Iterator<Item = T>,
//                T: juniper::GraphQLType<juniper::DefaultScalarValue>,
//        {
//            type Item = T;
//        }
//        trait GraphQLTraitAsync<T> {
//            type Item: juniper::GraphQLType<juniper::DefaultScalarValue>;
//        }
//        impl<T, Type> GraphQLTraitAsync<T> for Type
//            where
//                Type: futures::Stream<Item = T>,
//                T: juniper::GraphQLType<juniper::DefaultScalarValue>,
//        {
//            type Item = T;
//        }
//        let fields = vec![
//            registry.field_convert::<
//                <Pin<Box<dyn Stream<Item=User> + Send>> as GraphQLTraitAsync<_>>::Item,
//                _,
//                Self::Context
//            >("users",info)
//        ] ;
//        let meta = registry.build_object_type::<Subscription>(info, &fields);
//        meta.into_meta()
//    }
//
//    fn resolve_field(
//        &self,
//        _: &(),
//        _: &str,
//        _: &juniper::Arguments<juniper::DefaultScalarValue>,
//        _: &juniper::Executor<Self::Context, juniper::DefaultScalarValue>,
//    ) -> juniper::ExecutionResult<juniper::DefaultScalarValue> {
//        {
//            panic!("Called `resolve_field` on subscription object")
//        };
//    }
//
//    fn concrete_type_name(&self, _: &Self::Context, _: &Self::TypeInfo) -> String {
//        "Subscription".to_string()
//    }
//}
//
//impl juniper::GraphQLSubscriptionType<juniper::DefaultScalarValue> for Subscription {
//
//    fn resolve_field_into_stream<'args, 'e, 'res, 'life0, 'life1, 'life2, 'async_trait>(
//        &'life0 self,
//        info: &'life1 Self::TypeInfo,
//        field_name: &'life2 str,
//        arguments: juniper::Arguments<'args, juniper::DefaultScalarValue>,
//        executor: std::sync::Arc<juniper::Executor<'e, Self::Context, juniper::DefaultScalarValue>>,
//    ) -> std::pin::Pin<
//        Box<
//            dyn futures::future::Future<
//                Output = Result<
//                    juniper::Value<juniper::ValuesResultStream<'res, juniper::DefaultScalarValue>>,
//                    juniper::FieldError<juniper::DefaultScalarValue>,
//                >,
//            > + Send
//            + 'async_trait,
//        >,
//    >
//        where
//            'args: 'res,
//            'e: 'res,
//            'res: 'async_trait,
//            'life0: 'async_trait,
//            'life1: 'async_trait,
//            'life2: 'async_trait,
//            Self: 'async_trait,
//    {
//        use futures::stream::StreamExt as _;
//        use juniper::Value;
//        match field_name {
//            "users" => futures::FutureExt::boxed(async move {
//                let res: Pin<Box<dyn Stream<Item = User> + Send>> = {
//                    {
//                        let mut counter = 0;
//                        let stream =
//                            Interval::new_interval(Duration::from_secs(5)).map(move |_| {
//                                counter += 1;
//                                User {
//                                    id: counter,
//                                    kind: UserKind::Admin,
//                                    name: "stream user".to_string(),
//                                }
//                            });
//                        Box::pin(stream)
//                    }
//                };
//                return Err(FieldError::new(
//                    "some field error from handler",
//                    Value::Scalar(
//                        DefaultScalarValue::String("some additional string".to_string())
//                    )
//                ));
//                let f = res.then(move |res| {
//                    let res2: juniper::FieldResult<_, juniper::DefaultScalarValue> =
//                        juniper::IntoResolvable::into(res, executor.context());
//                    let ex = executor.clone();
//                    async move {
//                        match res2 {
//                            Ok(Some((ctx, r))) => {
//                                let sub = ex.replaced_context(ctx);
//                                sub.resolve_with_ctx_async(&(), &r).await
//                            }
//                            Ok(None) => Ok(Value::null()),
//                            Err(e) => Err(e)
//                        }
//                    }
//                });
//                Ok(juniper::Value::Scalar::<juniper::ValuesResultStream>(Box::pin(f)))
//            }),
//            _ => {
//                panic!("field not found")
//            }
//        }
//    }
//}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::new(), Subscription)
}

#[tokio::main]
async fn main() {
    ::std::env::set_var("RUST_LOG", "warp_async");
    env_logger::init();

    let log = warp::log("warp_server");

    let homepage = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(format!(
                "<html><h1>juniper_subscriptions demo</h1><div>visit <a href=\"/playground\">graphql playground</a></html>"
            ))
    });

    let state = warp::any().map(move || Context {});
    let qm_schema = schema();

    let state2 = warp::any().map(move || Context {});
    let s_schema = Arc::new(schema());
    let qm_graphql_filter = juniper_warp::make_graphql_filter_async(qm_schema, state.boxed());

    println!("Listening on 127.0.0.1:8080");

    let routes = (warp::path("subscriptions")
        .and(warp::ws())
        .and(state2.clone())
        .and(warp::any().map(move || Arc::clone(&s_schema)))
        .map(|ws: warp::ws::Ws, ctx: Context, schema: Arc<Schema>| {
            ws.on_upgrade(|websocket| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                println!("ws connected");
                juniper_warp::graphql_subscriptions_async(websocket, schema, ctx).boxed()
            })
        }))
    .or(warp::post()
        .and(warp::path("graphql"))
        .and(qm_graphql_filter))
    .or(warp::get()
        .and(warp::path("playground"))
        .and(playground_filter("/graphql", Some("/subscriptions"))))
    .or(homepage)
    .with(log);

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
