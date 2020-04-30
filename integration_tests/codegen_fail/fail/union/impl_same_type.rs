#[derive(juniper::GraphQLObject)]
pub struct Test {
    test: String,
}

enum Character {
    A(Test),
    B(Test),
}

#[juniper::graphql_union]
impl Character {
    fn resolve(&self) {
        match self {
            Test => match *self {
                Character::A(ref h) => Some(h),
                _ => None,
            },
            Test => match *self {
                Character::B(ref h) => Some(h),
                _ => None,
            },
        }
    }
}

fn main() {}
