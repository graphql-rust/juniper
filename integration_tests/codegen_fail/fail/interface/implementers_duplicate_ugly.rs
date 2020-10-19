use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    test: String,
}

type ObjAlias = ObjA;

#[graphql_interface]
impl Character for ObjA {
    fn id(&self) -> &str {
        "funA"
    }
}

#[graphql_interface(for = [ObjA, ObjAlias])]
trait Character {
    fn id(&self) -> &str;
}

fn main() {}
