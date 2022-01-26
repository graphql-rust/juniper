use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
pub struct ObjA {
    test: String,
}

#[graphql_interface]
trait Character {
    fn id(&self, obj: ObjA) -> &str;
}

fn main() {}
