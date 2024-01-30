use std::{future, pin::Pin};

use futures::{stream, Stream};
use juniper::{graphql_subscription, GraphQLInputObject};

type BoxStream<'a, I> = Pin<Box<dyn Stream<Item = I> + Send + 'a>>;

#[derive(GraphQLInputObject)]
struct ObjB {
    id: i32,
}

struct ObjA;

#[graphql_subscription]
impl ObjA {
    async fn id(&self) -> BoxStream<'static, ObjB> {
        Box::pin(stream::once(future::ready(ObjB { id: 34 })))
    }
}

fn main() {}
