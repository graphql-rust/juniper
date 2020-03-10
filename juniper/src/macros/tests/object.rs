// TODO: make sure proc macro tests cover all
// variants of the below

/*
use std::marker::PhantomData;

use crate::{
    ast::InputValue,
    executor::{Context, FieldResult},
    schema::model::RootNode,
    types::scalars::EmptyMutation,
    value::{DefaultScalarValue, Object, Value},
};


Syntax to validate:

* Order of items: fields, description, interfaces
* Optional Generics/lifetimes
* Custom name vs. default name
* Optional commas between items
* Nullable/fallible context switching

 */

/*
struct CustomName;
graphql_object!(CustomName: () as "ACustomNamedType" |&self| {
    field simple() -> i32 { 0 }
});

#[allow(dead_code)]
struct WithLifetime<'a> {
    data: PhantomData<&'a i32>,
}
graphql_object!(<'a> WithLifetime<'a>: () as "WithLifetime" |&self| {
    field simple() -> i32 { 0 }
});

#[allow(dead_code)]
struct WithGenerics<T> {
    data: T,
}
graphql_object!(<T> WithGenerics<T>: () as "WithGenerics" |&self| {
    field simple() -> i32 { 0 }
});

struct Interface;
struct DescriptionFirst;
graphql_interface!(Interface: () |&self| {
    field simple() -> i32 { 0 }

    instance_resolvers: |_| {
        DescriptionFirst => Some(DescriptionFirst {}),
    }
});
graphql_object!(DescriptionFirst: () |&self| {
    description: "A description"

    field simple() -> i32 { 0 }

    interfaces: [Interface]
});

struct FieldsFirst;
graphql_object!(FieldsFirst: () |&self| {
    field simple() -> i32 { 0 }

    description: "A description"

    interfaces: [Interface]
});

struct InterfacesFirst;
graphql_object!(InterfacesFirst: ()|&self| {
    interfaces: [Interface]

    field simple() -> i32 { 0 }

    description: "A description"
});

struct CommasWithTrailing;
graphql_object!(CommasWithTrailing: () |&self| {
    interfaces: [Interface],

    field simple() -> i32 { 0 },

    description: "A description",
});

struct CommasOnMeta;
graphql_object!(CommasOnMeta: () |&self| {
    interfaces: [Interface],
    description: "A description",

    field simple() -> i32 { 0 }
});

struct Root;

struct InnerContext;
impl Context for InnerContext {}

struct InnerType;
graphql_object!(InnerType: InnerContext | &self | {});

struct CtxSwitcher;
graphql_object!(CtxSwitcher: InnerContext |&self| {
    field ctx_switch_always(&executor) -> (&InnerContext, InnerType) {
        (executor.context(), InnerType)
    }

    field ctx_switch_opt(&executor) -> Option<(&InnerContext, InnerType)> {
        Some((executor.context(), InnerType))
    }

    field ctx_switch_res(&executor) -> FieldResult<(&InnerContext, InnerType)> {
        Ok((executor.context(), InnerType))
    }

    field ctx_switch_res_opt(&executor) -> FieldResult<Option<(&InnerContext, InnerType)>> {
        Ok(Some((executor.context(), InnerType)))
    }
});

graphql_object!(<'a> Root: InnerContext as "Root" |&self| {
    field custom_name() -> CustomName { CustomName {} }

    field with_lifetime() -> WithLifetime<'a> { WithLifetime { data: PhantomData } }
    field with_generics() -> WithGenerics<i32> { WithGenerics { data: 123 } }

    field description_first() -> DescriptionFirst { DescriptionFirst {} }
    field fields_first() -> FieldsFirst { FieldsFirst {} }
    field interfaces_first() -> InterfacesFirst { InterfacesFirst {} }

    field commas_with_trailing() -> CommasWithTrailing { CommasWithTrailing {} }
    field commas_on_meta() -> CommasOnMeta { CommasOnMeta {} }

    field ctx_switcher() -> CtxSwitcher { CtxSwitcher {} }
});

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
    let schema = RootNode::new(Root {}, EmptyMutation::<InnerContext>::new());
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
            "type": { "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" } }
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
            "type": { "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" } }
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
            "type": { "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" } }
        })));
    });
}

#[test]
fn introspect_description_first() {
    run_type_info_query("DescriptionFirst", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("DescriptionFirst"))
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
            "type": { "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" } }
        })));
    });
}

#[test]
fn introspect_fields_first() {
    run_type_info_query("FieldsFirst", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("FieldsFirst"))
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
            "type": { "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" } }
        })));
    });
}

#[test]
fn introspect_interfaces_first() {
    run_type_info_query("InterfacesFirst", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("InterfacesFirst"))
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
            "type": { "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" } }
        })));
    });
}

#[test]
fn introspect_commas_with_trailing() {
    run_type_info_query("CommasWithTrailing", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("CommasWithTrailing"))
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
            "type": { "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" } }
        })));
    });
}

#[test]
fn introspect_commas_on_meta() {
    run_type_info_query("CommasOnMeta", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("CommasOnMeta"))
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
            "type": { "kind": "NON_NULL", "name": None, "ofType": { "kind": "SCALAR", "name": "Int" } }
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
*/
