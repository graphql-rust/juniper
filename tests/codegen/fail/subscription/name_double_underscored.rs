use std::{future, pin::Pin};

use futures::{stream, Stream};
use juniper::graphql_subscription;

type BoxStream<'a, I> = Pin<Box<dyn Stream<Item = I> + Send + 'a>>;

struct __Obj;

#[graphql_subscription]
impl __Obj {
    fn id(&self) -> BoxStream<'static, &'static str> {
        Box::pin(stream::once(future::ready("funA")))
    }
}

fn main() {}
