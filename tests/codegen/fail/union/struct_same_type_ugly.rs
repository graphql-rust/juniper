use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLObject)]
struct Test {
    id: i32,
}

#[derive(GraphQLUnion)]
#[graphql(on Test = Character::a)]
#[graphql(on self::Test = Character::b)]
struct Character;

impl Character {
    fn a(&self, _: &()) -> Option<&Test> {
        None
    }

    fn b(&self, _: &()) -> Option<&Test> {
        None
    }
}

fn main() {}
