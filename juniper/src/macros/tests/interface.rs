use std::marker::PhantomData;

use ast::InputValue;
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use value::{Value, Object};

/*

Syntax to validate:

* Order of items: fields, description, instance resolvers
* Optional Generics/lifetimes
* Custom name vs. default name
* Optional commas between items
* Optional trailing commas on instance resolvers

*/

struct Concrete;

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

struct ResolversWithTrailingComma;

struct Root;

graphql_object!(Concrete: () |&self| {
    field simple() -> i32 { 0 }
});

graphql_interface!(CustomName: () as "ACustomNamedInterface" |&self| {
    field simple() -> i32 { 0 }

    instance_resolvers: |_| { Concrete => Some(Concrete) }
});

graphql_interface!(<'a> WithLifetime<'a>: () as "WithLifetime" |&self| {
    field simple() -> i32 { 0 }
    instance_resolvers: |_| { Concrete => Some(Concrete) }
});

graphql_interface!(<T> WithGenerics<T>: () as "WithGenerics" |&self| {
    field simple() -> i32 { 0 }
    instance_resolvers: |_| { Concrete => Some(Concrete) }
});

graphql_interface!(DescriptionFirst: () |&self| {
    description: "A description"

    field simple() -> i32 { 0 }

    instance_resolvers: |_| { Concrete => Some(Concrete) }
});

graphql_interface!(FieldsFirst: () |&self| {
    field simple() -> i32 { 0 }

    description: "A description"

    instance_resolvers: |_| { Concrete => Some(Concrete) }
});

graphql_interface!(InterfacesFirst: () |&self| {
    instance_resolvers: |_| { Concrete => Some(Concrete) }

    field simple() -> i32 { 0 }

    description: "A description"
});

graphql_interface!(CommasWithTrailing: () |&self| {
    instance_resolvers: |_| { Concrete => Some(Concrete) },

    field simple() -> i32 { 0 },

    description: "A description",
});

graphql_interface!(CommasOnMeta: () |&self| {
    instance_resolvers: |_| { Concrete => Some(Concrete) }
    description: "A description",

    field simple() -> i32 { 0 }
});

graphql_interface!(ResolversWithTrailingComma: () |&self| {
    instance_resolvers: |_| { Concrete => Some(Concrete), }
    description: "A description",

    field simple() -> i32 { 0 }
});

graphql_object!(<'a> Root: () as "Root" |&self| {
    field custom_name() -> CustomName { CustomName {} }

    field with_lifetime() -> WithLifetime<'a> { WithLifetime { data: PhantomData } }
    field with_generics() -> WithGenerics<i32> { WithGenerics { data: 123 } }

    field description_first() -> DescriptionFirst { DescriptionFirst {} }
    field fields_first() -> FieldsFirst { FieldsFirst {} }
    field interfaces_first() -> InterfacesFirst { InterfacesFirst {} }

    field commas_with_trailing() -> CommasWithTrailing { CommasWithTrailing {} }
    field commas_on_meta() -> CommasOnMeta { CommasOnMeta {} }

    field resolvers_with_trailing_comma() -> ResolversWithTrailingComma {
        ResolversWithTrailingComma {}
    }

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
            }
        }
    }
    "#;
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());
    let vars = vec![("typeName".to_owned(), InputValue::string(type_name))]
        .into_iter()
        .collect();

    let (result, errs) = ::execute(doc, None, &schema, &vars, &()).expect("Execution failed");

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
    run_type_info_query("ACustomNamedInterface", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::string("ACustomNamedInterface"))
        );
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));

        assert!(
            fields.contains(&Value::object(
                vec![("name", Value::string("simple"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_with_lifetime() {
    run_type_info_query("WithLifetime", |object, fields| {
        assert_eq!(object.get_field_value("name"), Some(&Value::string("WithLifetime")));
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));

        assert!(
            fields.contains(&Value::object(
                vec![("name", Value::string("simple"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_with_generics() {
    run_type_info_query("WithGenerics", |object, fields| {
        assert_eq!(object.get_field_value("name"), Some(&Value::string("WithGenerics")));
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));

        assert!(
            fields.contains(&Value::object(
                vec![("name", Value::string("simple"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_description_first() {
    run_type_info_query("DescriptionFirst", |object, fields| {
        assert_eq!(object.get_field_value("name"), Some(&Value::string("DescriptionFirst")));
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );

        assert!(
            fields.contains(&Value::object(
                vec![("name", Value::string("simple"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_fields_first() {
    run_type_info_query("FieldsFirst", |object, fields| {
        assert_eq!(object.get_field_value("name"), Some(&Value::string("FieldsFirst")));
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );

        assert!(
            fields.contains(&Value::object(
                vec![("name", Value::string("simple"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_interfaces_first() {
    run_type_info_query("InterfacesFirst", |object, fields| {
        assert_eq!(object.get_field_value("name"), Some(&Value::string("InterfacesFirst")));
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );

        assert!(
            fields.contains(&Value::object(
                vec![("name", Value::string("simple"))]
                    .into_iter()
                    .collect(),
            ))
        );
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

        assert!(
            fields.contains(&Value::object(
                vec![("name", Value::string("simple"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_commas_on_meta() {
    run_type_info_query("CommasOnMeta", |object, fields| {
        assert_eq!(object.get_field_value("name"), Some(&Value::string("CommasOnMeta")));
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );

        assert!(
            fields.contains(&Value::object(
                vec![("name", Value::string("simple"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_resolvers_with_trailing_comma() {
    run_type_info_query("ResolversWithTrailingComma", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::string("ResolversWithTrailingComma"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::string("A description"))
        );

        assert!(
            fields.contains(&Value::object(
                vec![("name", Value::string("simple"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}
