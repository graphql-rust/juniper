use juniper::graphql_scalar;

struct ScalarSpecifiedByUrl(i32);

#[graphql_scalar(specified_by_url = "not an url")]
impl GraphQLScalar for ScalarSpecifiedByUrl {
    type Error = String;

    fn to_output(&self) -> Value {
        Value::scalar(self.0)
    }

    fn from_input(v: &InputValue) -> Result<Self, Self::Error> {
        v.as_int_value()
            .map(Self)
            .ok_or_else(|| format!("Expected `Int`, found: {}", v))
    }

    fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_> {
        <i32 as ParseScalarValue>::from_str(value)
    }
}

fn main() {}
