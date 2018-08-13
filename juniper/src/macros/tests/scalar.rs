use executor::Variables;
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use value::{Value, Object};

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

graphql_scalar!(DefaultName {
    resolve(&self) -> Value {
        Value::int(self.0)
    }

    from_input_value(v: &InputValue) -> Option<DefaultName> {
        v.as_int_value().map(|i| DefaultName(i))
    }
});

graphql_scalar!(OtherOrder {
    from_input_value(v: &InputValue) -> Option<OtherOrder> {
        v.as_int_value().map(|i| OtherOrder(i))
    }

    resolve(&self) -> Value {
        Value::int(self.0)
    }
});

graphql_scalar!(Named as "ANamedScalar" {
    resolve(&self) -> Value {
        Value::int(self.0)
    }

    from_input_value(v: &InputValue) -> Option<Named> {
        v.as_int_value().map(|i| Named(i))
    }
});

graphql_scalar!(ScalarDescription {
    description: "A sample scalar, represented as an integer"

    resolve(&self) -> Value {
        Value::int(self.0)
    }

    from_input_value(v: &InputValue) -> Option<ScalarDescription> {
        v.as_int_value().map(|i| ScalarDescription(i))
    }
});

graphql_object!(Root: () |&self| {
    field default_name() -> DefaultName { DefaultName(0) }
    field other_order() -> OtherOrder { OtherOrder(0) }
    field named() -> Named { Named(0) }
    field scalar_description() -> ScalarDescription { ScalarDescription(0) }
});

fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn(&Object) -> (),
{
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) =
        ::execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

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
fn default_name_introspection() {
    let doc = r#"
    {
        __type(name: "DefaultName") {
            name
            description
        }
    }
    "#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(type_info.get_field_value("name"), Some(&Value::string("DefaultName")));
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));
    });
}

#[test]
fn other_order_introspection() {
    let doc = r#"
    {
        __type(name: "OtherOrder") {
            name
            description
        }
    }
    "#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(type_info.get_field_value("name"), Some(&Value::string("OtherOrder")));
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));
    });
}

#[test]
fn named_introspection() {
    let doc = r#"
    {
        __type(name: "ANamedScalar") {
            name
            description
        }
    }
    "#;

    run_type_info_query(doc, |type_info| {
        assert_eq!(type_info.get_field_value("name"), Some(&Value::string("ANamedScalar")));
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));
    });
}

#[test]
fn scalar_description_introspection() {
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
            Some(&Value::string("ScalarDescription"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::string("A sample scalar, represented as an integer"))
        );
    });
}
