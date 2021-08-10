use std::pin::Pin;

use juniper::graphql_subscription;

type Stream<'a, I> = Pin<Box<dyn futures::Stream<Item = I> + Send + 'a>>;

struct ObjA;

#[graphql_subscription]
impl ObjA {
    async fn id(&self) -> Stream<'static, &'static str> {
        Box::pin(stream::once(future::ready("funA")))
    }

    #[graphql(name = "id")]
    async fn id2(&self) -> Stream<'static, &'static str> {
        Box::pin(stream::once(future::ready("funB")))
    }
}

fn main() {}
