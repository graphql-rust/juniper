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
    fn wrong(
        &self,
        #[graphql(default = [true, false, false])]
        input: [bool; 2],
    ) -> bool {
        input[0]
    }
}

fn main() {}
