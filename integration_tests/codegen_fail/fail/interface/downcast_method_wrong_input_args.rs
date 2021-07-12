use juniper::{graphql_interface, GraphQLObject};

#[graphql_interface(for = Human)]
trait Character {
    fn id(&self) -> i32 {
        0
    }

    #[graphql(downcast)]
    fn a(&self, ctx: &(), rand: u8) -> Option<&Human> {
        None
    }
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct Human {
    id: String,
}

#[graphql_interface]
impl Character for Human {}

fn main() {}
