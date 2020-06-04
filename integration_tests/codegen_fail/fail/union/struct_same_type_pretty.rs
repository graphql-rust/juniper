use juniper::GraphQLUnion;

#[derive(GraphQLUnion)]
#[graphql(on i32 = Character::a)]
#[graphql(on i32 = Character::b)]
struct Character;

impl Character {
    fn a(&self, _: &()) -> Option<&i32> {
        None
    }

    fn b(&self, _: &()) -> Option<&i32> {
        None
    }
}

fn main() {}
