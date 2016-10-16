use std::collections::HashMap;

use ast::InputValue;
use executor::FieldResult;
use value::Value;
use schema::model::RootNode;

/*

Syntax to validate:

* Order of items: fields, description, interfaces
* Optional Generics/lifetimes
* Custom name vs. default name
* Optional commas between items

 */

struct Interface;

struct DefaultName;

struct WithLifetime;
struct WithGenerics;

struct DescriptionFirst;
struct FieldsFirst;
struct InterfacesFirst;

struct CommasWithTrailing;
struct CommasOnMeta;

struct Root;

graphql_object!(DefaultName: () |&self| {
    field simple() -> FieldResult<i64> { Ok(0) }
});


graphql_object!(<'a> &'a WithLifetime: () as "WithLifetime" |&self| {
    field simple() -> FieldResult<i64> { Ok(0) }
});

graphql_object!(<CtxT> WithGenerics: CtxT as "WithGenerics" |&self| {
    field simple() -> FieldResult<i64> { Ok(0) }
});

graphql_interface!(Interface: () as "Interface" |&self| {
    field simple() -> FieldResult<i64> { Ok(0) }

    instance_resolvers: |_| [
        Some(DescriptionFirst {}),
    ]
});

graphql_object!(DescriptionFirst: () as "DescriptionFirst" |&self| {
    description: "A description"

    field simple() -> FieldResult<i64> { Ok(0) }

    interfaces: [Interface]
});

graphql_object!(FieldsFirst: () as "FieldsFirst" |&self| {
    field simple() -> FieldResult<i64> { Ok(0) }

    description: "A description"

    interfaces: [Interface]
});

graphql_object!(InterfacesFirst: () as "InterfacesFirst" |&self| {
    interfaces: [Interface]

    field simple() -> FieldResult<i64> { Ok(0) }

    description: "A description"
});

graphql_object!(CommasWithTrailing: () as "CommasWithTrailing" |&self| {
    interfaces: [Interface],

    field simple() -> FieldResult<i64> { Ok(0) },

    description: "A description",
});


graphql_object!(CommasOnMeta: () as "CommasOnMeta" |&self| {
    interfaces: [Interface],
    description: "A description",

    field simple() -> FieldResult<i64> { Ok(0) }
});

graphql_object!(Root: () as "Root" |&self| {
    field default_name() -> FieldResult<DefaultName> { Ok(DefaultName {}) }

    field with_lifetime() -> FieldResult<&WithLifetime> { Err("Nope".to_owned()) }
    field with_generics() -> FieldResult<WithGenerics> { Ok(WithGenerics {}) }

    field description_first() -> FieldResult<DescriptionFirst> { Ok(DescriptionFirst {}) }
    field fields_first() -> FieldResult<FieldsFirst> { Ok(FieldsFirst {}) }
    field interfaces_first() -> FieldResult<InterfacesFirst> { Ok(InterfacesFirst {}) }

    field commas_with_trailing() -> FieldResult<CommasWithTrailing> { Ok(CommasWithTrailing {}) }
    field commas_on_meta() -> FieldResult<CommasOnMeta> { Ok(CommasOnMeta {}) }
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
    let schema = RootNode::new(Root {}, ());
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
fn introspect_default_name() {
    run_type_info_query("DefaultName", |object, fields| {
        assert_eq!(object.get("name"), Some(&Value::string("DefaultName")));
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
