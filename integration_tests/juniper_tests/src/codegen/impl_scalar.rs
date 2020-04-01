use fnv::FnvHashMap;
#[cfg(test)]
use juniper::{
    self, parser::ScalarToken, DefaultScalarValue, GraphQLType, InputValue, ParseScalarResult,
    ParseScalarValue, Value,
};

struct UserId(String);

// TODO Trait that the macro handles, move to proper location! Naming?
trait ParseCustomScalarValue<T, S = DefaultScalarValue> {
    fn resolve(&self) -> Value;
    fn from_input_value(value: &InputValue) -> Option<T>;
    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S>;
}

#[juniper::graphql_scalar2(name = "MyCustomName", description = "My custom description")]
impl ParseCustomScalarValue<UserId> for UserId {
    fn resolve(&self) -> Value {
        Value::scalar(self.0.to_owned())
    }

    fn from_input_value(value: &InputValue) -> Option<UserId> {
        value.as_string_value().map(|s| UserId(s.to_owned()))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
}

#[test]
fn test_generated_meta() {
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = <UserId as GraphQLType<DefaultScalarValue>>::meta(&(), &mut registry);

    assert_eq!(meta.name(), Some("MyCustomName"));
    assert_eq!(
        meta.description(),
        Some(&"My custom description".to_string())
    );
}
