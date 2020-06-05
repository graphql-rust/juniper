use juniper::{GraphQLEnum, GraphQLUnion};

#[derive(GraphQLEnum)]
pub enum Test {
    A,
    B,
}

#[derive(GraphQLUnion)]
#[graphql(on Test = Character::a)]
struct Character;

impl Character {
    fn a(&self, _: &()) -> Option<&Test> {
        None
    }
}

fn main() {}
