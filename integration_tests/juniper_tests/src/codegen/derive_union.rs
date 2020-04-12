// Default Test
#[derive(juniper::GraphQLObject)]
pub struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
pub struct Droid {
    id: String,
    primary_function: String,
}

#[derive(juniper::GraphQLUnion)]
pub enum Character {
    One(Human),
    Two(Droid),
}

// Context Test
pub struct CustomContext;

impl juniper::Context for CustomContext {}

#[derive(juniper::GraphQLObject)]
#[graphql(Context = CustomContext)]
pub struct HumanContext {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
#[graphql(Context = CustomContext)]
pub struct DroidContext {
    id: String,
    primary_function: String,
}

#[derive(juniper::GraphQLUnion)]
#[graphql(Context = CustomContext)]
pub enum CharacterContext {
    One(HumanContext),
    Two(DroidContext),
}

// #[juniper::object] compatibility

pub struct HumanCompat {
    id: String,
    home_planet: String,
}

#[juniper::graphql_object]
impl HumanCompat {
    fn id(&self) -> &String {
        &self.id
    }

    fn home_planet(&self) -> &String {
        &self.home_planet
    }
}

pub struct DroidCompat {
    id: String,
    primary_function: String,
}

#[juniper::graphql_object]
impl DroidCompat {
    fn id(&self) -> &String {
        &self.id
    }

    fn primary_function(&self) -> &String {
        &self.primary_function
    }
}

// NOTICE: this can not compile
// #[derive(juniper::GraphQLUnion)]
// pub enum CharacterCompatFail {
//     One(HumanCompat),
//     Two(DroidCompat),
// }

#[derive(juniper::GraphQLUnion)]
#[graphql(Scalar = juniper::DefaultScalarValue)]
pub enum CharacterCompat {
    One(HumanCompat),
    Two(DroidCompat),
}
