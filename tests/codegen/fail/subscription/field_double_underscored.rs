use std::pin::Pin;

use juniper::graphql_subscription;

type Stream<'a, I> = Pin<Box<dyn futures::Stream<Item = I> + Send + 'a>>;

struct ObjA;

#[graphql_subscription]
impl Character for ObjA {
    async fn __id() -> Stream<'static, &'static str> {
        Box::pin(stream::once(future::ready("funA")))
    }
}

fn main() {}
