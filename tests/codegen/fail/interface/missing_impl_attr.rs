use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
pub struct ObjA {
    id: String,
}

#[graphql_interface(for = ObjA)]
trait Character {
    fn id(&self) -> &str;
}

fn main() {}
