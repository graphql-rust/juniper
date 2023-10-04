// Local types overriding the stdlib Result should not affect generated code

type Result<T> = std::result::Result<T, ()>;

#[derive(juniper::GraphQLInputObject)]
#[graphql(name = "UserInformation")]
pub struct Update {
    pub email: Option<String>,
    pub username: Option<String>,
}

pub fn main() {}
