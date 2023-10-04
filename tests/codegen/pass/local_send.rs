// Local types overriding the stdlib Send should not affect generated code

trait Send {}

#[derive(juniper::GraphQLInputObject)]
#[graphql(name = "UserInformation")]
pub struct Update {
    pub email: Option<String>,
    pub username: Option<String>,
}

pub fn main() {}
