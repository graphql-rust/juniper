enum Character {
    A(std::string::String),
    B(String),
}

#[juniper::graphql_union]
impl Character {
    fn resolve(&self) {
        match self {
            String => match *self {
                Character::A(ref h) => Some(h),
                _ => None,
            },
            String => match *self {
                Character::B(ref h) => Some(h),
                _ => None,
            },
        }
    }
}

fn main() {}
