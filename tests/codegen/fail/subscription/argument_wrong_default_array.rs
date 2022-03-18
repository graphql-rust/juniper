use std::pin::Pin;

use futures::{future, stream};
use juniper::graphql_subscription;

type Stream<'a, I> = Pin<Box<dyn futures::Stream<Item = I> + Send + 'a>>;

struct ObjA;

#[graphql_subscription]
impl ObjA {
    async fn wrong(
        &self,
        #[graphql(default = [true, false, false])] input: [bool; 2],
    ) -> Stream<'static, bool> {
        Box::pin(stream::once(future::ready(input[0])))
    }
}

fn main() {}
