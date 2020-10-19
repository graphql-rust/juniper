use juniper::{graphql_interface, GraphQLInputObject, GraphQLObject};

#[derive(GraphQLInputObject)]
pub struct ObjB {
    id: i32,
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    test: String,
}

#[graphql_interface]
impl Character for ObjA {}

#[graphql_interface(for = ObjA)]
trait Character {
    fn id(&self) -> ObjB {
        ObjB { id: 34 }
    }
}

fn main() {}
