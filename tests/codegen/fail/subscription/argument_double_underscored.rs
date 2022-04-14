use std::pin::Pin;

use juniper::graphql_subscription;

type Stream<'a, I> = Pin<Box<dyn futures::Stream<Item = I> + Send + 'a>>;

struct Obj;

#[graphql_subscription]
impl Obj {
    async fn id(&self, __num: i32) -> Stream<'static, &'static str> {
        Box::pin(stream::once(future::ready("funA")))
    }
}

fn main() {}
