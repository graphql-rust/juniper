use std::marker::PhantomData;

use ast::InputValue;
use executor::{Context, FieldResult};
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use value::{Object, Value};

/*

Syntax to validate:

* Order of items: fields, description, interfaces
* Optional Generics/lifetimes
* Custom name vs. default name
* Optional commas between items
* Nullable/fallible context switching

 */

struct Interface;

struct CustomName;

#[allow(dead_code)]
struct WithLifetime<'a> {
    data: PhantomData<&'a i32>,
}

#[allow(dead_code)]
struct WithGenerics<T> {
    data: T,
}

struct DescriptionFirst;
struct FieldsFirst;
struct InterfacesFirst;

struct CommasWithTrailing;
struct CommasOnMeta;

struct Root;

graphql_object!(CustomName: () as "ACustomNamedType" |&self| {
    field simple() -> i32 { 0 }
});

graphql_object!(<'a> WithLifetime<'a>: () as "WithLifetime" |&self| {
    field simple() -> i32 { 0 }
});

graphql_object!(<T> WithGenerics<T>: () as "WithGenerics" |&self| {
    field simple() -> i32 { 0 }
});

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

graphql_object!(FieldsFirst: () |&self| {
    field simple() -> i32 { 0 }

    description: "A description"

    interfaces: [Interface]
});

graphql_object!(InterfacesFirst: ()|&self| {
    interfaces: [Interface]

    field simple() -> i32 { 0 }

    description: "A description"
});

graphql_object!(CommasWithTrailing: () |&self| {
    interfaces: [Interface],

    field simple() -> i32 { 0 },

    description: "A description",
});

graphql_object!(CommasOnMeta: () |&self| {
    interfaces: [Interface],
    description: "A description",

    field simple() -> i32 { 0 }
});

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
    F: Fn(&Object, &Vec<Value>) -> (),
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
    let vars = vec![("typeName".to_owned(), InputValue::string(type_name))]
        .into_iter()
        .collect();

    let (result, errs) =
        ::execute(doc, None, &schema, &vars, &InnerContext).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

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
            Some(&Value::string("ACustomNamedType"))
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
            Some(&Value::string("WithLifetime"))
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
            Some(&Value::string("WithGenerics"))
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
            Some(&Value::string("DescriptionFirst"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );
        assert_eq!(
            object.get_field_value("interfaces"),
            Some(&Value::list(vec![Value::object(
                vec![
                    ("name", Value::string("Interface")),
                    ("kind", Value::string("INTERFACE")),
                ].into_iter()
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
            Some(&Value::string("FieldsFirst"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );
        assert_eq!(
            object.get_field_value("interfaces"),
            Some(&Value::list(vec![Value::object(
                vec![
                    ("name", Value::string("Interface")),
                    ("kind", Value::string("INTERFACE")),
                ].into_iter()
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
            Some(&Value::string("InterfacesFirst"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );
        assert_eq!(
            object.get_field_value("interfaces"),
            Some(&Value::list(vec![Value::object(
                vec![
                    ("name", Value::string("Interface")),
                    ("kind", Value::string("INTERFACE")),
                ].into_iter()
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
            Some(&Value::string("CommasWithTrailing"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );
        assert_eq!(
            object.get_field_value("interfaces"),
            Some(&Value::list(vec![Value::object(
                vec![
                    ("name", Value::string("Interface")),
                    ("kind", Value::string("INTERFACE")),
                ].into_iter()
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
            Some(&Value::string("CommasOnMeta"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );
        assert_eq!(
            object.get_field_value("interfaces"),
            Some(&Value::list(vec![Value::object(
                vec![
                    ("name", Value::string("Interface")),
                    ("kind", Value::string("INTERFACE")),
                ].into_iter()
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
