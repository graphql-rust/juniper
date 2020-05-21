// Trait.

#[derive(juniper::GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

trait Character {
    fn as_human(&self) -> Option<&Human> {
        None
    }
    fn as_droid(&self) -> Option<&Droid> {
        None
    }
}

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

/*
#[juniper::graphql_union]
impl GraphQLUnion for dyn Character {
    fn resolve_human(&self) -> Option<&Human> {
        self.as_human()
    }

    fn resolve_droid(&self) -> Option<&Droid> {
        self.as_droid()
    }
}
*/

/*
#[derive(GraphQLUnion)]
#[graphql(
    Human = Char::resolve_human,
    Droid = Char::resolve_droid,
)]
#[graphql(with(Char::resolve_human) => Human)]
#[graphql(object = Droid, with = Char::resolve_droid)]
struct Char {
    id: String,
}

impl Char {
    fn resolve_human(&self, _: &Context) -> Option<&Human> {
        unimplemented!()
    }
    fn resolve_droid(&self, _: &Context) -> Option<&Droid> {
        unimplemented!()
    }
}

#[graphq_union]
trait Charctr {
    fn as_human(&self) -> Option<&Human> { None }
    fn as_droid(&self, _: &Context) -> Option<&Droid> { None }
}

#[graphq_union(
    Human = Char::resolve_human,
    Droid = Char::resolve_droid,
)]
#[graphql(object = Human, with = Charctr2::resolve_human)]
#[graphql(object = Droid, with = Charctr2::resolve_droid)]
trait Charctr2 {
    fn id(&self) -> &str;
}

impl dyn Charctr2 {
    fn resolve_human(&self, _: &Context) -> Option<&Human> {
        unimplemented!()
    }
    fn resolve_droid(&self, _: &Context) -> Option<&Droid> {
        unimplemented!()
    }
}
*/