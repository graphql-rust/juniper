use juniper::{
    execute, graphql_object, graphql_scalar, graphql_value, graphql_vars, DefaultScalarValue,
    EmptyMutation, EmptySubscription, Object, ParseScalarResult, ParseScalarValue, RootNode, Value,
};

use crate::custom_scalar::MyScalarValue;

struct DefaultName(i32);
struct OtherOrder(i32);
struct Named(i32);
struct ScalarDescription(i32);
struct ScalarSpecifiedByUrl(i32);
struct Generated(String);

struct Root;

/*

Syntax to validate:

* Default name vs. custom name
* Description vs. no description on the scalar

*/

#[graphql_scalar]
impl<S> GraphQLScalar for DefaultName
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    fn from_input_value(v: &InputValue) -> Result<DefaultName, String> {
        v.as_int_value()
            .map(DefaultName)
            .ok_or_else(|| format!("Expected `Int`, found: {}", v))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <i32 as ParseScalarValue<S>>::from_str(value)
    }
}

#[graphql_scalar]
impl GraphQLScalar for OtherOrder {
    fn resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    fn from_input_value(v: &InputValue) -> Result<OtherOrder, String> {
        v.as_int_value()
            .map(OtherOrder)
            .ok_or_else(|| format!("Expected `Int`, found: {}", v))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, DefaultScalarValue> {
        <i32 as ParseScalarValue>::from_str(value)
    }
}

#[graphql_scalar(name = "ANamedScalar")]
impl GraphQLScalar for Named {
    fn resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    fn from_input_value(v: &InputValue) -> Result<Named, String> {
        v.as_int_value()
            .map(Named)
            .ok_or_else(|| format!("Expected `Int`, found: {}", v))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, DefaultScalarValue> {
        <i32 as ParseScalarValue>::from_str(value)
    }
}

#[graphql_scalar(description = "A sample scalar, represented as an integer")]
impl GraphQLScalar for ScalarDescription {
    fn resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    fn from_input_value(v: &InputValue) -> Result<ScalarDescription, String> {
        v.as_int_value()
            .map(ScalarDescription)
            .ok_or_else(|| format!("Expected `Int`, found: {}", v))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, DefaultScalarValue> {
        <i32 as ParseScalarValue>::from_str(value)
    }
}

#[graphql_scalar(specified_by_url = "https://tools.ietf.org/html/rfc4122")]
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

macro_rules! impl_scalar {
    ($name: ident) => {
        #[graphql_scalar]
        impl<S> GraphQLScalar for $name
        where
            S: ScalarValue,
        {
            fn resolve(&self) -> Value {
                Value::scalar(self.0.clone())
            }

            fn from_input_value(v: &InputValue) -> Result<Self, &'static str> {
                v.as_scalar_value()
                    .and_then(|v| v.as_str())
                    .and_then(|s| Some(Self(s.to_owned())))
                    .ok_or_else(|| "Expected `String`")
            }

            fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
                <String as ParseScalarValue<S>>::from_str(value)
            }
        }
    };
}

impl_scalar!(Generated);

#[graphql_object(scalar = DefaultScalarValue)]
impl Root {
    fn default_name() -> DefaultName {
        DefaultName(0)
    }
    fn other_order() -> OtherOrder {
        OtherOrder(0)
    }
    fn named() -> Named {
        Named(0)
    }
    fn scalar_description() -> ScalarDescription {
        ScalarDescription(0)
    }
    fn scalar_specified_by_url() -> ScalarSpecifiedByUrl {
        ScalarSpecifiedByUrl(0)
    }
    fn generated() -> Generated {
        Generated("foo".to_owned())
    }
}

struct WithCustomScalarValue(i32);

#[graphql_scalar]
impl GraphQLScalar for WithCustomScalarValue {
    fn resolve(&self) -> Value<MyScalarValue> {
        Value::scalar(self.0)
    }

    fn from_input_value(v: &InputValue<MyScalarValue>) -> Result<WithCustomScalarValue, String> {
        v.as_int_value()
            .map(WithCustomScalarValue)
            .ok_or_else(|| format!("Expected Int, found: {}", v))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, MyScalarValue> {
        <i32 as ParseScalarValue<MyScalarValue>>::from_str(value)
    }
}

struct RootWithCustomScalarValue;

#[graphql_object(scalar = MyScalarValue)]
impl RootWithCustomScalarValue {
    fn with_custom_scalar_value() -> WithCustomScalarValue {
        WithCustomScalarValue(0)
    }
}

async fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn(&Object<DefaultScalarValue>) -> (),
{
    let schema = RootNode::new(
        Root {},
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = execute(doc, None, &schema, &graphql_vars! {}, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:#?}", result);

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    f(type_info);
}

#[test]
fn path_in_resolve_return_type() {
    struct ResolvePath(i32);

    #[graphql_scalar]
    impl GraphQLScalar for ResolvePath {
        fn resolve(&self) -> self::Value {
            Value::scalar(self.0)
        }

        fn from_input_value(v: &InputValue) -> Result<ResolvePath, String> {
            v.as_int_value()
                .map(ResolvePath)
                .ok_or_else(|| format!("Expected `Int`, found: {}", v))
        }

        fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, DefaultScalarValue> {
            <i32 as ParseScalarValue>::from_str(value)
        }
    }
}

#[tokio::test]
async fn default_name_introspection() {
    let doc = r#"
    {
        __type(name: "DefaultName") {
            name
            description
        }
    }
    "#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("DefaultName")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );
    })
    .await;
}

#[tokio::test]
async fn other_order_introspection() {
    let doc = r#"
    {
        __type(name: "OtherOrder") {
            name
            description
        }
    }
    "#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("OtherOrder")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );
    })
    .await;
}

#[tokio::test]
async fn named_introspection() {
    let doc = r#"
    {
        __type(name: "ANamedScalar") {
            name
            description
        }
    }
    "#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("ANamedScalar")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );
    })
    .await;
}

#[tokio::test]
async fn scalar_description_introspection() {
    let doc = r#"
    {
        __type(name: "ScalarDescription") {
            name
            description
            specifiedByUrl
        }
    }
    "#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("ScalarDescription")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(
                "A sample scalar, represented as an integer",
            )),
        );
        assert_eq!(
            type_info.get_field_value("specifiedByUrl"),
            Some(&graphql_value!(null)),
        );
    })
    .await;
}

#[tokio::test]
async fn scalar_specified_by_url_introspection() {
    let doc = r#"{
        __type(name: "ScalarSpecifiedByUrl") {
            name
            specifiedByUrl
        }
    }"#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("ScalarSpecifiedByUrl")),
        );
        assert_eq!(
            type_info.get_field_value("specifiedByUrl"),
            Some(&graphql_value!("https://tools.ietf.org/html/rfc4122")),
        );
    })
    .await;
}

#[tokio::test]
async fn generated_scalar_introspection() {
    let doc = r#"
    {
        __type(name: "Generated") {
            name
            description
        }
    }
    "#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("Generated")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );
    })
    .await;
}

#[tokio::test]
async fn resolves_with_custom_scalar_value() {
    const DOC: &str = r#"{ withCustomScalarValue }"#;

    let schema = RootNode::<_, _, _, MyScalarValue>::new_with_scalar_value(
        RootWithCustomScalarValue,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    assert_eq!(
        execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
        Ok((graphql_value!({"withCustomScalarValue": 0}), vec![])),
    );
}
