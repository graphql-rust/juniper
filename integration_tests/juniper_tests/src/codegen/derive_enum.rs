#[cfg(test)]
use fnv::FnvHashMap;

#[cfg(test)]
use juniper::{self, DefaultScalarValue, FromInputValue, GraphQLType, InputValue, ToInputValue};

pub struct CustomContext {}

impl juniper::Context for CustomContext {}

#[derive(juniper::GraphQLEnum, Debug, PartialEq)]
#[graphql(name = "Some", description = "enum descr")]
enum SomeEnum {
    Regular,
    #[graphql(name = "FULL", description = "field descr", deprecated = "depr")]
    Full,
}

/// Enum doc.
#[derive(juniper::GraphQLEnum)]
enum DocEnum {
    /// Variant doc.
    Foo,
}

/// Doc 1.\
/// Doc 2.
///
/// Doc 4.
#[derive(juniper::GraphQLEnum, Debug, PartialEq)]
enum MultiDocEnum {
    /// Variant 1.
    /// Variant 2.
    Foo,
}

/// This is not used as the description.
#[derive(juniper::GraphQLEnum, Debug, PartialEq)]
#[graphql(description = "enum override")]
enum OverrideDocEnum {
    /// This is not used as the description.
    #[graphql(description = "variant override")]
    Foo,
}

#[derive(juniper::GraphQLEnum)]
#[graphql(context = CustomContext, noasync)]
enum ContextEnum {
    A,
}

#[test]
fn test_derived_enum() {
    // Ensure that rename works.
    assert_eq!(
        <SomeEnum as GraphQLType<DefaultScalarValue>>::name(&()),
        Some("Some")
    );

    // Ensure validity of meta info.
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = SomeEnum::meta(&(), &mut registry);

    assert_eq!(meta.name(), Some("Some"));
    assert_eq!(meta.description(), Some(&"enum descr".to_string()));

    // Test Regular variant.
    assert_eq!(
        <_ as ToInputValue>::to_input_value(&SomeEnum::Regular),
        InputValue::scalar("REGULAR")
    );
    assert_eq!(
        FromInputValue::<DefaultScalarValue>::from_input_value(&InputValue::scalar("REGULAR")),
        Some(SomeEnum::Regular)
    );

    // Test FULL variant.
    assert_eq!(
        <_ as ToInputValue>::to_input_value(&SomeEnum::Full),
        InputValue::scalar("FULL")
    );
    assert_eq!(
        FromInputValue::<DefaultScalarValue>::from_input_value(&InputValue::scalar("FULL")),
        Some(SomeEnum::Full)
    );
}

#[test]
fn test_doc_comment() {
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = DocEnum::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some(&"Enum doc.".to_string()));
}

#[test]
fn test_multi_doc_comment() {
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = MultiDocEnum::meta(&(), &mut registry);
    assert_eq!(
        meta.description(),
        Some(&"Doc 1. Doc 2.\n\nDoc 4.".to_string())
    );
}

#[test]
fn test_doc_comment_override() {
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = OverrideDocEnum::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some(&"enum override".to_string()));
}

fn test_context<T>(_t: T)
where
    T: GraphQLType<DefaultScalarValue, Context = CustomContext>,
{
    // empty
}

#[test]
fn test_doc_custom_context() {
    test_context(ContextEnum::A);
    // test_context(OverrideDocEnum::Foo); does not work
}
