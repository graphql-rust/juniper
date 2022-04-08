use juniper::GraphQLUnion;

#[derive(GraphQLUnion)]
#[graphql(on String = Character::a)]
#[graphql(on std::string::String = Character::b)]
struct Character;

impl Character {
    fn a(&self, _: &()) -> Option<&String> {
        None
    }

    fn b(&self, _: &()) -> Option<&String> {
        None
    }
}

fn main() {}
