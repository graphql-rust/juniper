use std::{future, pin::Pin};

use futures::{stream, Stream};
use juniper::graphql_subscription;

type BoxStream<'a, I> = Pin<Box<dyn Stream<Item = I> + Send + 'a>>;

struct Obj;

#[graphql_subscription]
impl Obj {
    async fn id(&self, __num: i32) -> BoxStream<'static, &'static str> {
        Box::pin(stream::once(future::ready("funA")))
    }
}

fn main() {}
