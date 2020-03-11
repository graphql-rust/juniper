use crate::{
    executor::Variables,
    schema::model::RootNode,
    types::scalars::EmptyMutation,
    value::{DefaultScalarValue, Object, ParseScalarResult, ParseScalarValue, Value},
};

struct DefaultName(i32);
struct OtherOrder(i32);
struct Named(i32);
struct ScalarDescription(i32);

struct Root;

/*

Syntax to validate:

* Default name vs. custom name
* Description vs. no description on the scalar

*/

graphql_scalar!(DefaultName where Scalar = <S> {
    resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    from_input_value(v: &InputValue) -> Option<DefaultName> {
        v.as_scalar_value().and_then(|s| s.as_int()).map(|i| DefaultName(i))
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <i32 as ParseScalarValue<S>>::from_str(value)
    }
});

graphql_scalar!(OtherOrder {
    resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    from_input_value(v: &InputValue) -> Option<OtherOrder> {
        v.as_scalar_value::<i32>().map(|i| OtherOrder(*i))
    }


    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, DefaultScalarValue> {
        <i32 as ParseScalarValue>::from_str(value)
    }
});

graphql_scalar!(Named as "ANamedScalar" where Scalar = DefaultScalarValue {
    resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    from_input_value(v: &InputValue) -> Option<Named> {
        v.as_scalar_value::<i32>().map(|i| Named(*i))
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, DefaultScalarValue> {
        <i32 as ParseScalarValue>::from_str(value)
    }
});

graphql_scalar!(ScalarDescription  {
    description: "A sample scalar, represented as an integer"

    resolve(&self) -> Value {
        Value::scalar(self.0)
    }

    from_input_value(v: &InputValue) -> Option<ScalarDescription> {
        v.as_scalar_value::<i32>().map(|i| ScalarDescription(*i))
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a> {
        <i32 as ParseScalarValue>::from_str(value)
    }
});

#[crate::graphql_object_internal]
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
}

async fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn(&Object<DefaultScalarValue>) -> (),
{
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) = crate::execute(doc, None, &schema, &Variables::new(), &())
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

    graphql_scalar!(ResolvePath {
        resolve(&self) -> self::Value {
            Value::scalar(self.0)
        }

        from_input_value(v: &InputValue) -> Option<ResolvePath> {
            v.as_scalar_value::<i32>().map(|i| ResolvePath(*i))
        }

        from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a> {
            <i32 as ParseScalarValue>::from_str(value)
        }
    });
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
            Some(&Value::scalar("DefaultName"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::null())
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
            Some(&Value::scalar("OtherOrder"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::null())
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
            Some(&Value::scalar("ANamedScalar"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::null())
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
        }
    }
    "#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::scalar("ScalarDescription"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::scalar("A sample scalar, represented as an integer"))
        );
    })
    .await;
}
