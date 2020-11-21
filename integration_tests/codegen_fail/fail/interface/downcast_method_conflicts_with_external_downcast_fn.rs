use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    test: String,
}

#[graphql_interface]
impl Character for ObjA {
    fn id(&self, _: i32) -> &str {
        "funA"
    }

    fn as_obja(&self) -> Option<&ObjA> {
        Some(self)
    }
}

#[graphql_interface(for = ObjA)]
#[graphql_interface(on ObjA = downcast_obja)]
trait Character {
    fn id(&self, num: i32) -> &str;

    #[graphql(downcast)]
    fn as_obja(&self) -> Option<&ObjA>;
}

fn main() {}
