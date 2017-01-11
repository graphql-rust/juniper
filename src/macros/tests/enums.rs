use std::collections::HashMap;

use executor::Variables;
use value::Value;
use schema::model::RootNode;
use types::scalars::EmptyMutation;


enum DefaultName { Foo, Bar }
enum Named { Foo, Bar }
enum NoTrailingComma { Foo, Bar }
enum EnumDescription { Foo, Bar }
enum EnumValueDescription { Foo, Bar }
enum EnumDeprecation { Foo, Bar }

struct Root;

/*

Syntax to validate:

* Default name vs. custom name
* Trailing comma vs. no trailing comma
* Description vs. no description on the enum
* Description vs. no description on the enum values themselves
* Deprecation on enum fields

*/

graphql_enum!(DefaultName {
    DefaultName::Foo => "FOO",
    DefaultName::Bar => "BAR",
});

graphql_enum!(Named as "ANamedEnum" {
    Named::Foo => "FOO",
    Named::Bar => "BAR",
});

graphql_enum!(NoTrailingComma {
    NoTrailingComma::Foo => "FOO",
    NoTrailingComma::Bar => "BAR"
});

graphql_enum!(EnumDescription {
    description: "A description of the enum itself"

    EnumDescription::Foo => "FOO",
    EnumDescription::Bar => "BAR",
});

graphql_enum!(EnumValueDescription {
    EnumValueDescription::Foo => "FOO" as "The FOO value",
    EnumValueDescription::Bar => "BAR" as "The BAR value",
});

graphql_enum!(EnumDeprecation {
    EnumDeprecation::Foo => "FOO" deprecated "Please don't use FOO any more",
    EnumDeprecation::Bar => "BAR" as "The BAR value" deprecated "Please don't use BAR any more",
});

graphql_object!(Root: () |&self| {
    field default_name() -> DefaultName { DefaultName::Foo }
    field named() -> Named { Named::Foo }
    field no_trailing_comma() -> NoTrailingComma { NoTrailingComma::Foo }
    field enum_description() -> EnumDescription { EnumDescription::Foo }
    field enum_value_description() -> EnumValueDescription { EnumValueDescription::Foo }
    field enum_deprecation() -> EnumDeprecation { EnumDeprecation::Foo }
});

fn run_type_info_query<F>(doc: &str, f: F) where F: Fn((&HashMap<String, Value>, &Vec<Value>)) -> () {
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) = ::execute(doc, None, &schema, &Variables::new(), &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value().expect("Result is not an object")
        .get("__type").expect("__type field missing")
        .as_object_value().expect("__type field not an object value");

    let values = type_info
        .get("enumValues").expect("enumValues field missing")
        .as_list_value().expect("enumValues not a list");

    f((type_info, values));
}

#[test]
fn default_name_introspection() {
    let doc = r#"
    {
        __type(name: "DefaultName") {
            name
            description
            enumValues {
                name
                description
                isDeprecated
                deprecationReason
            }
        }
    }
    "#;

    run_type_info_query(doc, |(type_info, values)| {
        assert_eq!(type_info.get("name"), Some(&Value::string("DefaultName")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("FOO")),
            ("description", Value::null()),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("BAR")),
            ("description", Value::null()),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));
    });
}

#[test]
fn named_introspection() {
    let doc = r#"
    {
        __type(name: "ANamedEnum") {
            name
            description
            enumValues {
                name
                description
                isDeprecated
                deprecationReason
            }
        }
    }
    "#;

    run_type_info_query(doc, |(type_info, values)| {
        assert_eq!(type_info.get("name"), Some(&Value::string("ANamedEnum")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("FOO")),
            ("description", Value::null()),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("BAR")),
            ("description", Value::null()),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));
    });
}

#[test]
fn no_trailing_comma_introspection() {
    let doc = r#"
    {
        __type(name: "NoTrailingComma") {
            name
            description
            enumValues {
                name
                description
                isDeprecated
                deprecationReason
            }
        }
    }
    "#;

    run_type_info_query(doc, |(type_info, values)| {
        assert_eq!(type_info.get("name"), Some(&Value::string("NoTrailingComma")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("FOO")),
            ("description", Value::null()),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("BAR")),
            ("description", Value::null()),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));
    });
}

#[test]
fn enum_description_introspection() {
    let doc = r#"
    {
        __type(name: "EnumDescription") {
            name
            description
            enumValues {
                name
                description
                isDeprecated
                deprecationReason
            }
        }
    }
    "#;

    run_type_info_query(doc, |(type_info, values)| {
        assert_eq!(type_info.get("name"), Some(&Value::string("EnumDescription")));
        assert_eq!(type_info.get("description"), Some(&Value::string("A description of the enum itself")));

        assert_eq!(values.len(), 2);

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("FOO")),
            ("description", Value::null()),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("BAR")),
            ("description", Value::null()),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));
    });
}

#[test]
fn enum_value_description_introspection() {
    let doc = r#"
    {
        __type(name: "EnumValueDescription") {
            name
            description
            enumValues {
                name
                description
                isDeprecated
                deprecationReason
            }
        }
    }
    "#;

    run_type_info_query(doc, |(type_info, values)| {
        assert_eq!(type_info.get("name"), Some(&Value::string("EnumValueDescription")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("FOO")),
            ("description", Value::string("The FOO value")),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("BAR")),
            ("description", Value::string("The BAR value")),
            ("isDeprecated", Value::boolean(false)),
            ("deprecationReason", Value::null()),
        ].into_iter().collect())));
    });
}

#[test]
fn enum_deprecation_introspection() {
    let doc = r#"
    {
        __type(name: "EnumDeprecation") {
            name
            description
            enumValues(includeDeprecated: true) {
                name
                description
                isDeprecated
                deprecationReason
            }
        }
    }
    "#;

    run_type_info_query(doc, |(type_info, values)| {
        assert_eq!(type_info.get("name"), Some(&Value::string("EnumDeprecation")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("FOO")),
            ("description", Value::null()),
            ("isDeprecated", Value::boolean(true)),
            ("deprecationReason", Value::string("Please don't use FOO any more")),
        ].into_iter().collect())));

        assert!(values.contains(&Value::object(vec![
            ("name", Value::string("BAR")),
            ("description", Value::string("The BAR value")),
            ("isDeprecated", Value::boolean(true)),
            ("deprecationReason", Value::string("Please don't use BAR any more")),
        ].into_iter().collect())));
    });
}

#[test]
fn enum_deprecation_no_values_introspection() {
    let doc = r#"
    {
        __type(name: "EnumDeprecation") {
            name
            description
            enumValues {
                name
                description
                isDeprecated
                deprecationReason
            }
        }
    }
    "#;

    run_type_info_query(doc, |(type_info, values)| {
        assert_eq!(type_info.get("name"), Some(&Value::string("EnumDeprecation")));
        assert_eq!(type_info.get("description"), Some(&Value::null()));

        assert_eq!(values.len(), 0);
    });
}
