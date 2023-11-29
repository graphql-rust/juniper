use std::pin::Pin;

use futures::{future, stream};
use juniper::{graphql_subscription, GraphQLInputObject};

type Stream<'a, I> = Pin<Box<dyn futures::Stream<Item = I> + Send + 'a>>;

#[derive(GraphQLInputObject)]
struct ObjB {
    id: i32,
}

struct ObjA;

#[graphql_subscription]
impl ObjA {
    async fn id(&self) -> Stream<'static, ObjB> {
        Box::pin(stream::once(future::ready(ObjB { id: 34 })))
    }
}

fn main() {}
