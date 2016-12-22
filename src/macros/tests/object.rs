use std::collections::HashMap;
use std::marker::PhantomData;

use ast::InputValue;
use value::Value;
use schema::model::RootNode;
use types::scalars::EmptyMutation;

/*

Syntax to validate:

* Order of items: fields, description, interfaces
* Optional Generics/lifetimes
* Custom name vs. default name
* Optional commas between items

 */

struct Interface;

struct CustomName;

#[allow(dead_code)]
struct WithLifetime<'a> { data: PhantomData<&'a i64> }

#[allow(dead_code)]
struct WithGenerics<T> { data: T }

struct DescriptionFirst;
struct FieldsFirst;
struct InterfacesFirst;

struct CommasWithTrailing;
struct CommasOnMeta;

struct Root;

graphql_object!(CustomName: () as "ACustomNamedType" |&self| {
    field simple() -> i64 { 0 }
});


graphql_object!(<'a> WithLifetime<'a>: () as "WithLifetime" |&self| {
    field simple() -> i64 { 0 }
});

graphql_object!(<T> WithGenerics<T>: () as "WithGenerics" |&self| {
    field simple() -> i64 { 0 }
});


graphql_interface!(Interface: () |&self| {
    field simple() -> i64 { 0 }

    instance_resolvers: |_| {
        DescriptionFirst => Some(DescriptionFirst {}),
    }
});

graphql_object!(DescriptionFirst: () |&self| {
    description: "A description"

    field simple() -> i64 { 0 }

    interfaces: [Interface]
});

graphql_object!(FieldsFirst: () |&self| {
    field simple() -> i64 { 0 }

    description: "A description"

    interfaces: [Interface]
});

graphql_object!(InterfacesFirst: ()|&self| {
    interfaces: [Interface]

    field simple() -> i64 { 0 }

    description: "A description"
});

graphql_object!(CommasWithTrailing: () |&self| {
    interfaces: [Interface],

    field simple() -> i64 { 0 },

    description: "A description",
});


graphql_object!(CommasOnMeta: () |&self| {
    interfaces: [Interface],
    description: "A description",

    field simple() -> i64 { 0 }
});

graphql_object!(<'a> Root: () as "Root" |&self| {
    field custom_name() -> CustomName { CustomName {} }

    field with_lifetime() -> WithLifetime<'a> { WithLifetime { data: PhantomData } }
    field with_generics() -> WithGenerics<i64> { WithGenerics { data: 123 } }

    field description_first() -> DescriptionFirst { DescriptionFirst {} }
    field fields_first() -> FieldsFirst { FieldsFirst {} }
    field interfaces_first() -> InterfacesFirst { InterfacesFirst {} }

    field commas_with_trailing() -> CommasWithTrailing { CommasWithTrailing {} }
    field commas_on_meta() -> CommasOnMeta { CommasOnMeta {} }
});


fn run_type_info_query<F>(type_name: &str, f: F)
    where F: Fn(&HashMap<String, Value>, &Vec<Value>) -> ()
{
    let doc = r#"
    query ($typeName: String!) {
        __type(name: $typeName) {
            name
            description
            fields(includeDeprecated: true) {
                name
            }
            interfaces {
                name
                kind
            }
        }
    }
    "#;
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());
    let vars = vec![
        ("typeName".to_owned(), InputValue::string(type_name)),
    ].into_iter().collect();

    let (result, errs) = ::execute(doc, None, &schema, &vars, &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value().expect("Result is not an object")
        .get("__type").expect("__type field missing")
        .as_object_value().expect("__type field not an object value");

    let fields = type_info
        .get("fields").expect("fields field missing")
        .as_list_value().expect("fields field not a list value");

    f(type_info, fields);
}

#[test]
fn introspect_custom_name() {
    run_type_info_query("ACustomNamedType", |object, fields| {
        assert_eq!(object.get("name"), Some(&Value::string("ACustomNamedType")));
        assert_eq!(object.get("description"), Some(&Value::null()));
        assert_eq!(object.get("interfaces"), Some(&Value::list(vec![])));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("simple")),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_with_lifetime() {
    run_type_info_query("WithLifetime", |object, fields| {
        assert_eq!(object.get("name"), Some(&Value::string("WithLifetime")));
        assert_eq!(object.get("description"), Some(&Value::null()));
        assert_eq!(object.get("interfaces"), Some(&Value::list(vec![])));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("simple")),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_with_generics() {
    run_type_info_query("WithGenerics", |object, fields| {
        assert_eq!(object.get("name"), Some(&Value::string("WithGenerics")));
        assert_eq!(object.get("description"), Some(&Value::null()));
        assert_eq!(object.get("interfaces"), Some(&Value::list(vec![])));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("simple")),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_description_first() {
    run_type_info_query("DescriptionFirst", |object, fields| {
        assert_eq!(object.get("name"), Some(&Value::string("DescriptionFirst")));
        assert_eq!(object.get("description"), Some(&Value::string("A description")));
        assert_eq!(object.get("interfaces"), Some(&Value::list(vec![
            Value::object(vec![
                ("name", Value::string("Interface")),
                ("kind", Value::string("INTERFACE")),
            ].into_iter().collect()),
        ])));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("simple")),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_fields_first() {
    run_type_info_query("FieldsFirst", |object, fields| {
        assert_eq!(object.get("name"), Some(&Value::string("FieldsFirst")));
        assert_eq!(object.get("description"), Some(&Value::string("A description")));
        assert_eq!(object.get("interfaces"), Some(&Value::list(vec![
            Value::object(vec![
                ("name", Value::string("Interface")),
                ("kind", Value::string("INTERFACE")),
            ].into_iter().collect()),
        ])));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("simple")),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_interfaces_first() {
    run_type_info_query("InterfacesFirst", |object, fields| {
        assert_eq!(object.get("name"), Some(&Value::string("InterfacesFirst")));
        assert_eq!(object.get("description"), Some(&Value::string("A description")));
        assert_eq!(object.get("interfaces"), Some(&Value::list(vec![
            Value::object(vec![
                ("name", Value::string("Interface")),
                ("kind", Value::string("INTERFACE")),
            ].into_iter().collect()),
        ])));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("simple")),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_commas_with_trailing() {
    run_type_info_query("CommasWithTrailing", |object, fields| {
        assert_eq!(object.get("name"), Some(&Value::string("CommasWithTrailing")));
        assert_eq!(object.get("description"), Some(&Value::string("A description")));
        assert_eq!(object.get("interfaces"), Some(&Value::list(vec![
            Value::object(vec![
                ("name", Value::string("Interface")),
                ("kind", Value::string("INTERFACE")),
            ].into_iter().collect()),
        ])));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("simple")),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_commas_on_meta() {
    run_type_info_query("CommasOnMeta", |object, fields| {
        assert_eq!(object.get("name"), Some(&Value::string("CommasOnMeta")));
        assert_eq!(object.get("description"), Some(&Value::string("A description")));
        assert_eq!(object.get("interfaces"), Some(&Value::list(vec![
            Value::object(vec![
                ("name", Value::string("Interface")),
                ("kind", Value::string("INTERFACE")),
            ].into_iter().collect()),
        ])));

        assert!(fields.contains(&Value::object(vec![
            ("name", Value::string("simple")),
        ].into_iter().collect())));
    });
}
