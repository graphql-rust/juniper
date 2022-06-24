use juniper::{graphql_union, FromContext, GraphQLObject};

#[graphql_union]
trait Character {
    fn a(&self, ctx: &SubContext) -> Option<&Human>;
    fn b(&self, ctx: &CustomContext) -> Option<&Droid>;
}

#[derive(GraphQLObject)]
#[graphql(context = CustomContext)]
pub struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
#[graphql(context = CustomContext)]
pub struct Droid {
    id: String,
    primary_function: String,
}

pub struct CustomContext;
impl juniper::Context for CustomContext {}

pub struct SubContext;
impl juniper::Context for SubContext {}

impl FromContext<CustomContext> for SubContext {
    fn from(_: &CustomContext) -> &Self {
        &Self
    }
}

fn main() {}
