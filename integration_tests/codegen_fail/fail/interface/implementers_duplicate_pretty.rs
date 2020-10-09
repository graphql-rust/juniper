use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    test: String,
}

#[graphql_interface]
impl Character for ObjA {
    fn id(&self) -> &str {
        "funA"
    }
}

#[graphql_interface(for = [ObjA, ObjA])]
trait Character {
    fn id(&self) -> &str;
}

fn main() {}
