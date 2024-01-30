use std::{future, pin::Pin};

use futures::{stream, Stream};
use juniper::{graphql_subscription, GraphQLObject};

type BoxStream<'a, I> = Pin<Box<dyn Stream<Item = I> + Send + 'a>>;

#[derive(GraphQLObject)]
struct ObjA {
    test: String,
}

struct ObjB;

#[graphql_subscription]
impl ObjB {
    async fn id(&self, obj: ObjA) -> BoxStream<'static, &'static str> {
        Box::pin(stream::once(future::ready("funA")))
    }
}

fn main() {}
