use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    test: String,
}

#[graphql_interface]
impl Character for ObjA {
    fn id(&self, obj: Self) -> &str {
        "funA"
    }
}

#[graphql_interface(for = ObjA)]
trait Character {
    fn id(&self, obj: ObjA) -> &str;
}

fn main() {}
