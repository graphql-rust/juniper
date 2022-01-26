use juniper::{graphql_interface, GraphQLInputObject};

#[derive(GraphQLInputObject)]
pub struct ObjB {
    id: i32,
}

#[graphql_interface]
trait Character {
    fn id(&self) -> ObjB;
}

fn main() {}
