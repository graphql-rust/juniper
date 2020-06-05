#[derive(juniper::GraphQLEnum)]
#[graphql(context = ())]
pub enum Test {
    A,
    B,
}

enum Character {
    Test(Test),
}

#[juniper::graphql_union]
impl Character {
    async fn resolve(&self) {
        match self {
            Test => match *self {
                Character::Test(ref h) => Some(h),
            },
        }
    }
}

fn main() {}
