use std::collections::HashMap;

use value::Value;
use ast::InputValue;
use schema::model::RootNode;
use executor::FieldResult;

struct Interface;
struct Root;

/*

Syntax to validate:

* Object vs. interface
* Description vs. no description
* Deprecated vs. not deprecated
* FieldResult vs. object directly
* Return vs. implicit return

*/

graphql_object!(Root: () |&self| {
    field simple() -> i64 { 0 }

    field description() -> i64 as "Field description" { 0 }

    field deprecated "Deprecation reason"
        deprecated() -> i64 { 0 }

    field deprecated "Deprecation reason"
        deprecated_descr() -> i64 as "Field description" { 0 }

    field with_field_result() -> FieldResult<i64> { Ok(0) }

    field with_return() -> i64 { return 0; }

    field with_return_field_result() -> FieldResult<i64> { return Ok(0); }

    interfaces: [Interface]
});

graphql_interface!(Interface: () |&self| {
    field simple() -> i64 { 0 }

    field description() -> i64 as "Field description" { 0 }

    field deprecated "Deprecation reason"
        deprecated() -> i64 { 0 }

    field deprecated "Deprecation reason"
        deprecated_descr() -> i64 as "Field description" { 0 }

    instance_resolvers: |&_| {
        Root => Some(Root {}),
    }
});

fn run_field_info_query<F>(type_name: &str, field_name: &str, f: F)
    where F: Fn(&HashMap<String, Value>) -> ()
{
    let doc = r#"
    query ($typeName: String!) {
        __type(name: $typeName) {
            fields(includeDeprecated: true) {
                name
                description
                isDeprecated
                deprecationReason
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
        .as_list_value().expect("fields not a list");

    let field = fields
        .into_iter().filter(
            |f| f.as_object_value().expect("Field not an object")
                .get("name").expect("name field missing from field")
                .as_string_value().expect("name is not a string")
                == field_name)
        .next().expect("Field not found")
        .as_object_value().expect("Field is not an object");

    println!("Field: {:?}", field);

    f(field);
}

#[test]
fn introspect_object_field_simple() {
    run_field_info_query("Root", "simple", |field| {
        assert_eq!(field.get("name"), Some(&Value::string("simple")));
        assert_eq!(field.get("description"), Some(&Value::null()));
        assert_eq!(field.get("isDeprecated"), Some(&Value::boolean(false)));
        assert_eq!(field.get("deprecationReason"), Some(&Value::null()));
    });
}

#[test]
fn introspect_interface_field_simple() {
    run_field_info_query("Interface", "simple", |field| {
        assert_eq!(field.get("name"), Some(&Value::string("simple")));
        assert_eq!(field.get("description"), Some(&Value::null()));
        assert_eq!(field.get("isDeprecated"), Some(&Value::boolean(false)));
        assert_eq!(field.get("deprecationReason"), Some(&Value::null()));
    });
}

#[test]
fn introspect_object_field_description() {
    run_field_info_query("Root", "description", |field| {
        assert_eq!(field.get("name"), Some(&Value::string("description")));
        assert_eq!(field.get("description"), Some(&Value::string("Field description")));
        assert_eq!(field.get("isDeprecated"), Some(&Value::boolean(false)));
        assert_eq!(field.get("deprecationReason"), Some(&Value::null()));
    });
}

#[test]
fn introspect_interface_field_description() {
    run_field_info_query("Interface", "description", |field| {
        assert_eq!(field.get("name"), Some(&Value::string("description")));
        assert_eq!(field.get("description"), Some(&Value::string("Field description")));
        assert_eq!(field.get("isDeprecated"), Some(&Value::boolean(false)));
        assert_eq!(field.get("deprecationReason"), Some(&Value::null()));
    });
}

#[test]
fn introspect_object_field_deprecated() {
    run_field_info_query("Root", "deprecated", |field| {
        assert_eq!(field.get("name"), Some(&Value::string("deprecated")));
        assert_eq!(field.get("description"), Some(&Value::null()));
        assert_eq!(field.get("isDeprecated"), Some(&Value::boolean(true)));
        assert_eq!(field.get("deprecationReason"), Some(&Value::string("Deprecation reason")));
    });
}

#[test]
fn introspect_interface_field_deprecated() {
    run_field_info_query("Interface", "deprecated", |field| {
        assert_eq!(field.get("name"), Some(&Value::string("deprecated")));
        assert_eq!(field.get("description"), Some(&Value::null()));
        assert_eq!(field.get("isDeprecated"), Some(&Value::boolean(true)));
        assert_eq!(field.get("deprecationReason"), Some(&Value::string("Deprecation reason")));
    });
}

#[test]
fn introspect_object_field_deprecated_descr() {
    run_field_info_query("Root", "deprecatedDescr", |field| {
        assert_eq!(field.get("name"), Some(&Value::string("deprecatedDescr")));
        assert_eq!(field.get("description"), Some(&Value::string("Field description")));
        assert_eq!(field.get("isDeprecated"), Some(&Value::boolean(true)));
        assert_eq!(field.get("deprecationReason"), Some(&Value::string("Deprecation reason")));
    });
}

#[test]
fn introspect_interface_field_deprecated_descr() {
    run_field_info_query("Interface", "deprecatedDescr", |field| {
        assert_eq!(field.get("name"), Some(&Value::string("deprecatedDescr")));
        assert_eq!(field.get("description"), Some(&Value::string("Field description")));
        assert_eq!(field.get("isDeprecated"), Some(&Value::boolean(true)));
        assert_eq!(field.get("deprecationReason"), Some(&Value::string("Deprecation reason")));
    });
}
