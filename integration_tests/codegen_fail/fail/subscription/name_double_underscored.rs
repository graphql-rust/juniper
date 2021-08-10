use std::pin::Pin;

use juniper::graphql_subscription;

type Stream<'a, I> = Pin<Box<dyn futures::Stream<Item = I> + Send + 'a>>;

struct __Obj;

#[graphql_subscription]
impl __Obj {
    fn id(&self) -> Stream<'static, &'static str> {
        Box::pin(stream::once(future::ready("funA")))
    }
}

fn main() {}
