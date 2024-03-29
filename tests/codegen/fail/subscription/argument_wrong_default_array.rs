use std::{future, pin::Pin};

use futures::{stream, Stream};
use juniper::graphql_subscription;

type BoxStream<'a, I> = Pin<Box<dyn Stream<Item = I> + Send + 'a>>;

struct ObjA;

#[graphql_subscription]
impl ObjA {
    async fn wrong(
        &self,
        #[graphql(default = [true, false, false])] input: [bool; 2],
    ) -> BoxStream<'static, bool> {
        Box::pin(stream::once(future::ready(input[0])))
    }
}

fn main() {}
