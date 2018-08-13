use executor::Variables;
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use value::{Value, Object};

/*

Syntax to validate:

* Default name vs. custom name
* Trailing comma vs. no trailing comma
* Description vs. no description on the enum
* Description vs. no description on the enum values themselves
* Deprecation on enum fields

*/

#[derive(GraphQLEnum)]
#[graphql(_internal)]
enum DefaultName {
    Foo,
    Bar,
}

#[derive(GraphQLEnum)]
#[graphql(name = "ANamedEnum", _internal)]
enum Named {
    Foo,
    Bar,
}

#[derive(GraphQLEnum)]
#[graphql(_internal)]
enum NoTrailingComma {
    Foo,
    Bar,
}

#[derive(GraphQLEnum)]
#[graphql(description = "A description of the enum itself", _internal)]
enum EnumDescription {
    Foo,
    Bar,
}

#[derive(GraphQLEnum)]
#[graphql(_internal)]
enum EnumValueDescription {
    #[graphql(description = "The FOO value")]
    Foo,
    #[graphql(description = "The BAR value")]
    Bar,
}

#[derive(GraphQLEnum)]
#[graphql(_internal)]
enum EnumDeprecation {
    #[graphql(deprecated = "Please don't use FOO any more")]
    Foo,
    #[graphql(description = "The BAR value", deprecated = "Please don't use BAR any more")]
    Bar,
}

struct Root;

graphql_object!(Root: () |&self| {
    field default_name() -> DefaultName { DefaultName::Foo }
    field named() -> Named { Named::Foo }
    field no_trailing_comma() -> NoTrailingComma { NoTrailingComma::Foo }
    field enum_description() -> EnumDescription { EnumDescription::Foo }
    field enum_value_description() -> EnumValueDescription { EnumValueDescription::Foo }
    field enum_deprecation() -> EnumDeprecation { EnumDeprecation::Foo }
});

fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn((&Object, &Vec<Value>)) -> (),
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

    let values = type_info
        .get_field_value("enumValues")
        .expect("enumValues field missing")
        .as_list_value()
        .expect("enumValues not a list");

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
        assert_eq!(type_info.get_field_value("name"), Some(&Value::string("DefaultName")));
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("FOO")),
                    ("description", Value::null()),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("BAR")),
                    ("description", Value::null()),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
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
        assert_eq!(type_info.get_field_value("name"), Some(&Value::string("ANamedEnum")));
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("FOO")),
                    ("description", Value::null()),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("BAR")),
                    ("description", Value::null()),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
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
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::string("NoTrailingComma"))
        );
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("FOO")),
                    ("description", Value::null()),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("BAR")),
                    ("description", Value::null()),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
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
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::string("EnumDescription"))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&Value::string("A description of the enum itself"))
        );

        assert_eq!(values.len(), 2);

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("FOO")),
                    ("description", Value::null()),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("BAR")),
                    ("description", Value::null()),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
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
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::string("EnumValueDescription"))
        );
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("FOO")),
                    ("description", Value::string("The FOO value")),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("BAR")),
                    ("description", Value::string("The BAR value")),
                    ("isDeprecated", Value::boolean(false)),
                    ("deprecationReason", Value::null()),
                ].into_iter()
                    .collect(),
            ))
        );
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
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::string("EnumDeprecation"))
        );
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(values.len(), 2);

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("FOO")),
                    ("description", Value::null()),
                    ("isDeprecated", Value::boolean(true)),
                    (
                        "deprecationReason",
                        Value::string("Please don't use FOO any more"),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string("BAR")),
                    ("description", Value::string("The BAR value")),
                    ("isDeprecated", Value::boolean(true)),
                    (
                        "deprecationReason",
                        Value::string("Please don't use BAR any more"),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
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
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::string("EnumDeprecation"))
        );
        assert_eq!(type_info.get_field_value("description"), Some(&Value::null()));

        assert_eq!(values.len(), 0);
    });
}
