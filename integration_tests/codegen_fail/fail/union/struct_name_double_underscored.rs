use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLUnion)]
#[graphql(on Human = __Character::a)]
struct __Character;

impl __Character {
    fn a(&self, _: &()) -> Option<&Human> {
        None
    }
}

#[derive(GraphQLObject)]
pub struct Human {
    id: String,
}

fn main() {}
