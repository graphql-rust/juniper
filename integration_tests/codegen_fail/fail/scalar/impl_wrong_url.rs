struct ScalarSpecifiedByUrl(i32);

#[juniper::graphql_scalar(specified_by_url = "not an url")]
impl GraphQLScalar for ScalarSpecifiedByUrl {
    fn resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    fn from_input_value(v: &InputValue) -> Result<ScalarSpecifiedByUrl, String> {
        v.as_int_value()
            .map(ScalarSpecifiedByUrl)
            .ok_or_else(|| format!("Expected `Int`, found: {}", v))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, DefaultScalarValue> {
        <i32 as ParseScalarValue>::from_str(value)
    }
}

fn main() {}
