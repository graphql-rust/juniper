use std::pin::Pin;

use futures::{future, stream};
use juniper::{graphql_subscription, GraphQLObject};

type Stream<'a, I> = Pin<Box<dyn futures::Stream<Item = I> + Send + 'a>>;

#[derive(GraphQLObject)]
struct ObjA {
    test: String,
}

struct ObjB;

#[graphql_subscription]
impl ObjB {
    async fn id(&self, obj: ObjA) -> Stream<'static, &'static str> {
        Box::pin(stream::once(future::ready("funA")))
    }
}

fn main() {}
