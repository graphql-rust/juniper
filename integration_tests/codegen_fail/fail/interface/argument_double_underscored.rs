use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    test: String,
}

#[graphql_interface]
impl Character for ObjA {}

#[graphql_interface(for = ObjA)]
trait Character {
    fn id(&self, __num: i32) -> &str {
        "funA"
    }
}

fn main() {}
