use juniper::graphql_scalar;

struct ScalarSpecifiedByUrl(i32);

#[graphql_scalar(specified_by_url = "not an url")]
impl GraphQLScalar for ScalarSpecifiedByUrl {
    type Error = String;

    fn resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    fn from_input_value(v: &InputValue) -> Result<Self, String> {
        v.as_int_value()
            .map(Self)
            .ok_or_else(|| format!("Expected `Int`, found: {}", v))
    }

    fn from_str(value: ScalarToken<'_>) -> ParseScalarResult<'_> {
        <i32 as ParseScalarValue>::from_str(value)
    }
}

fn main() {}
