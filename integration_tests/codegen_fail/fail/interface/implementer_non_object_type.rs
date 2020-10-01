use juniper::{graphql_interface, GraphQLInputObject};

#[derive(GraphQLInputObject)]
pub struct ObjA {
    test: String,
}

#[graphql_interface]
impl Character for ObjA {
    fn id(&self) -> &str {
        "funA"
    }
}

#[graphql_interface(for = ObjA)]
trait Character {
    fn id(&self) -> &str;
}

fn main() {}
