use std::{future, pin::Pin};

use futures::{stream, Stream};
use juniper::graphql_subscription;

type BoxStream<'a, I> = Pin<Box<dyn Stream<Item = I> + Send + 'a>>;

struct ObjA {
    field: bool
}

#[graphql_subscription]
impl ObjA {
    fn id(&self) -> BoxStream<'static, bool> {
        Box::pin(stream::once(future::ready(self.self.field)))
    }
}

fn main() {}
