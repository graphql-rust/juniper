/*
Syntax to validate:

* Order of items: fields, description, interfaces
* Optional Generics/lifetimes
* Custom name vs. default name
* Optional commas between items
* Nullable/fallible context switching
*/

#![allow(dead_code)]

use std::marker::PhantomData;

use crate::{
    ast::InputValue,
    executor::{Context, FieldResult},
    graphql_interface, graphql_object,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    value::{DefaultScalarValue, Object, Value},
    GraphQLObject,
};

struct CustomName;

#[graphql_object(name = "ACustomNamedType")]
impl CustomName {
    fn simple() -> i32 {
        0
    }
}

struct WithLifetime<'a> {
    data: PhantomData<&'a i32>,
}

#[graphql_object]
impl<'a> WithLifetime<'a> {
    fn simple() -> i32 {
        0
    }
}

struct WithGenerics<T> {
    data: T,
}

#[graphql_object]
impl<T> WithGenerics<T> {
    fn simple() -> i32 {
        0
    }
}

#[graphql_interface(for = SimpleObject)]
trait Interface {
    fn simple(&self) -> i32 {
        0
    }
}

#[derive(GraphQLObject)]
#[graphql(impl = InterfaceValue, description = "A description")]
struct SimpleObject {
    simple: i32,
}

#[graphql_interface]
impl Interface for SimpleObject {}

struct InnerContext;
impl Context for InnerContext {}

#[derive(GraphQLObject)]
#[graphql(context = InnerContext)]
struct InnerType {
    a: i32,
}

struct CtxSwitcher;

#[graphql_object(context = InnerContext)]
impl CtxSwitcher {
    fn ctx_switch_always() -> (&InnerContext, InnerType) {
        (executor.context(), InnerType { a: 0 })
    }

    fn ctx_switch_opt() -> Option<(&InnerContext, InnerType)> {
        Some((executor.context(), InnerType { a: 0 }))
    }

    fn ctx_switch_res() -> FieldResult<(&InnerContext, InnerType)> {
        Ok((executor.context(), InnerType { a: 0 }))
    }

    fn ctx_switch_res_opt() -> FieldResult<Option<(&InnerContext, InnerType)>> {
        Ok(Some((executor.context(), InnerType { a: 0 })))
    }
}

struct Root;

#[graphql_object(context = InnerContext)]
impl Root {
    fn custom_name() -> CustomName {
        CustomName {}
    }

    fn with_lifetime() -> WithLifetime<'static> {
        WithLifetime { data: PhantomData }
    }
    fn with_generics() -> WithGenerics<i32> {
        WithGenerics { data: 123 }
    }

    fn description_first() -> SimpleObject {
        SimpleObject { simple: 0 }
    }
    fn interface() -> InterfaceValue {
        SimpleObject { simple: 0 }.into()
    }

    fn ctx_switcher() -> CtxSwitcher {
        CtxSwitcher {}
    }
}

fn run_type_info_query<F>(type_name: &str, f: F)
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
                type {
                    kind
                    name
                    ofType {
                        kind
                        name
                    }
                }
            }
            interfaces {
                name
                kind
            }
        }
    }
    "#;
    let schema = RootNode::new(
        Root {},
        EmptyMutation::<InnerContext>::new(),
        EmptySubscription::<InnerContext>::new(),
    );
    let vars = vec![("typeName".to_owned(), InputValue::scalar(type_name))]
        .into_iter()
        .collect();

    let (result, errs) =
        crate::execute_sync(doc, None, &schema, &vars, &InnerContext).expect("Execution failed");

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

#[test]
fn introspect_custom_name() {
    run_type_info_query("ACustomNamedType", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("ACustomNamedType"))
        );
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            object.get_field_value("interfaces"),
            Some(&Value::list(vec![]))
        );

        assert!(fields.contains(&graphql_value!({
            "name": "simple",
            "type": {
                "kind": "NON_NULL",
                "name": None,
                "ofType": { "kind": "SCALAR", "name": "Int" }
            }
        })));
    });
}

#[test]
fn introspect_with_lifetime() {
    run_type_info_query("WithLifetime", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("WithLifetime"))
        );
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            object.get_field_value("interfaces"),
            Some(&Value::list(vec![]))
        );

        assert!(fields.contains(&graphql_value!({
            "name": "simple",
            "type": {
                "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" }
            }
        })));
    });
}

#[test]
fn introspect_with_generics() {
    run_type_info_query("WithGenerics", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("WithGenerics"))
        );
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            object.get_field_value("interfaces"),
            Some(&Value::list(vec![]))
        );

        assert!(fields.contains(&graphql_value!({
            "name": "simple",
            "type": {
                "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" }
            }
        })));
    });
}

#[test]
fn introspect_simple_object() {
    run_type_info_query("SimpleObject", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("SimpleObject"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::scalar("A description"))
        );
        assert_eq!(
            object.get_field_value("interfaces"),
            Some(&Value::list(vec![Value::object(
                vec![
                    ("name", Value::scalar("Interface")),
                    ("kind", Value::scalar("INTERFACE")),
                ]
                .into_iter()
                .collect(),
            )]))
        );

        assert!(fields.contains(&graphql_value!({
            "name": "simple",
            "type": {
                "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" }
            }
        })));
    });
}

#[test]
fn introspect_ctx_switch() {
    run_type_info_query("CtxSwitcher", |_, fields| {
        assert!(fields.contains(&graphql_value!({
            "name": "ctxSwitchAlways",
            "type": {
                "kind": "NON_NULL",
                "name": None,
                "ofType": {
                    "kind": "OBJECT",
                    "name": "InnerType",
                }
            }
        })));

        assert!(fields.contains(&graphql_value!({
            "name": "ctxSwitchOpt",
            "type": {
                "kind": "OBJECT",
                "name": "InnerType",
                "ofType": None
            }
        })));

        assert!(fields.contains(&graphql_value!({
            "name": "ctxSwitchRes",
            "type": {
                "kind": "NON_NULL",
                "name": None,
                "ofType": {
                    "kind": "OBJECT",
                    "name": "InnerType",
                }
            }
        })));

        assert!(fields.contains(&graphql_value!({
            "name": "ctxSwitchResOpt",
            "type": {
                "kind": "OBJECT",
                "name": "InnerType",
                "ofType": None
            }
        })));
    });
}
