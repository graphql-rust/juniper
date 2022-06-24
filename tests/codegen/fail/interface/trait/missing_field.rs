use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    test: String,
}

#[graphql_interface(for = ObjA)]
trait Character {
    fn id(&self) -> &str;
}

fn main() {}
