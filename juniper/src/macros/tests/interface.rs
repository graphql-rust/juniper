/*

Syntax to validate:

* Order of items: fields, description, instance resolvers
* Optional Generics/lifetimes
* Custom name vs. default name
* Optional commas between items
* Optional trailing commas on instance resolvers

*/

use crate::{
    ast::InputValue,
    graphql_interface, graphql_object,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    value::{DefaultScalarValue, Object, Value},
};

struct Concrete;

#[graphql_object(impl = [
    CustomNameValue, DescriptionValue, WithLifetimeValue<'_>, WithGenericsValue<()>,
])]
impl Concrete {
    fn simple() -> i32 {
        0
    }
}

#[graphql_interface(for = Concrete, name = "ACustomNamedInterface")]
trait CustomName {
    fn simple(&self) -> i32;
}
#[graphql_interface]
impl CustomName for Concrete {
    fn simple(&self) -> i32 {
        0
    }
}

#[graphql_interface(for = Concrete)]
trait WithLifetime<'a> {
    fn simple(&self) -> i32;
}
#[graphql_interface]
impl<'a> WithLifetime<'a> for Concrete {
    fn simple(&self) -> i32 {
        0
    }
}

#[graphql_interface(for = Concrete)]
trait WithGenerics<T> {
    fn simple(&self) -> i32;
}
#[graphql_interface]
impl<T> WithGenerics<T> for Concrete {
    fn simple(&self) -> i32 {
        0
    }
}

#[graphql_interface(for = Concrete, desc = "A description")]
trait Description {
    fn simple(&self) -> i32;
}
#[graphql_interface]
impl Description for Concrete {
    fn simple(&self) -> i32 {
        0
    }
}

struct Root;

#[graphql_object]
impl Root {
    fn custom_name() -> CustomNameValue {
        Concrete.into()
    }

    fn with_lifetime() -> WithLifetimeValue<'static> {
        Concrete.into()
    }
    fn with_generics() -> WithGenericsValue<i32> {
        Concrete.into()
    }

    fn description() -> DescriptionValue {
        Concrete.into()
    }
}

async fn run_type_info_query<F>(type_name: &str, f: F)
where
    F: Fn(&Object<DefaultScalarValue>, &Vec<Value<DefaultScalarValue>>) -> (),
{
    let doc = r#"
    query ($typeName: String!) {
        __type(name: $typeName) {
            name
            description
            fields(includeDeprecated: true) {
                name
            }
        }
    }
    "#;
    let schema = RootNode::new(
        Root {},
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );
    let vars = vec![("typeName".to_owned(), InputValue::scalar(type_name))]
        .into_iter()
        .collect();

    let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
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

    let fields = type_info
        .get_field_value("fields")
        .expect("fields field missing")
        .as_list_value()
        .expect("fields field not a list value");

    f(type_info, fields);
}

#[tokio::test]
async fn introspect_custom_name() {
    run_type_info_query("ACustomNamedInterface", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("ACustomNamedInterface"))
        );
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn introspect_with_lifetime() {
    run_type_info_query("WithLifetime", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("WithLifetime"))
        );
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn introspect_with_generics() {
    run_type_info_query("WithGenerics", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("WithGenerics"))
        );
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn introspect_description_first() {
    run_type_info_query("Description", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("Description"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::scalar("A description"))
        );

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}
