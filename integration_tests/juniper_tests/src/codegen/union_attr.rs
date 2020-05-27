use juniper::{graphql_union, GraphQLObject};

#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[graphql_union(name = "Character")]
#[graphql_union(description = "A Collection of things")]
trait Character {
    fn as_human(&self, _: &()) -> Option<&Human> {
        None
    }
    fn as_droid(&self) -> Option<&Droid> {
        None
    }
    #[graphql_union(ignore)]
    fn some(&self);
}

/*
impl Character for Human {
    fn as_human(&self) -> Option<&Human> {
        Some(&self)
    }
}

impl Character for Droid {
    fn as_droid(&self) -> Option<&Droid> {
        Some(&self)
    }
}

#[juniper::graphql_union]
impl<'a> GraphQLUnion for &'a dyn Character {
    fn resolve(&self) {
        match self {
            Human => self.as_human(),
            Droid => self.as_droid(),
        }
    }
}
*/
