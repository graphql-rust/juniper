use crate::{
    graphql_value, graphql_vars,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    value::{DefaultScalarValue, Object, Value},
    GraphQLEnum,
};

/*

Syntax to validate:

* Default name vs. custom name
* Trailing comma vs. no trailing comma
* Description vs. no description on the enum
* Description vs. no description on the enum values themselves
* Deprecation on enum fields

*/

#[derive(GraphQLEnum)]
enum DefaultName {
    Foo,
    Bar,
}

#[derive(GraphQLEnum)]
#[graphql(name = "ANamedEnum")]
enum Named {
    Foo,
    Bar,
}

#[derive(GraphQLEnum)]
enum NoTrailingComma {
    Foo,
    Bar,
}

#[derive(GraphQLEnum)]
#[graphql(description = "A description of the enum itself")]
enum EnumDescription {
    Foo,
    Bar,
}

#[derive(GraphQLEnum)]
enum EnumValueDescription {
    #[graphql(description = "The FOO value")]
    Foo,
    #[graphql(description = "The BAR value")]
    Bar,
}

#[derive(GraphQLEnum)]
enum EnumDeprecation {
    #[graphql(deprecated = "Please don't use FOO any more")]
    Foo,
    #[graphql(
        description = "The BAR value",
        deprecated = "Please don't use BAR any more"
    )]
    Bar,
}

struct Root;

#[crate::graphql_object]
impl Root {
    fn default_name() -> DefaultName {
        DefaultName::Foo
    }
    fn named() -> Named {
        Named::Foo
    }
    fn no_trailing_comma() -> NoTrailingComma {
        NoTrailingComma::Foo
    }
    fn enum_description() -> EnumDescription {
        EnumDescription::Foo
    }
    fn enum_value_description() -> EnumValueDescription {
        EnumValueDescription::Foo
    }
    fn enum_deprecation() -> EnumDeprecation {
        EnumDeprecation::Foo
    }
}

async fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn((&Object<DefaultScalarValue>, &Vec<Value<DefaultScalarValue>>)),
{
    let schema = RootNode::new(
        Root,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = crate::execute(doc, None, &schema, &graphql_vars! {}, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {result:#?}");

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

#[tokio::test]
async fn default_name_introspection() {
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
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("DefaultName")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(values.len(), 2);

        assert!(values.contains(&graphql_value!({
            "name": "FOO",
            "description": null,
            "isDeprecated": false,
            "deprecationReason": null,
        })));

        assert!(values.contains(&graphql_value!({
            "name": "BAR",
            "description": null,
            "isDeprecated": false,
            "deprecationReason": null,
        })));
    })
    .await;
}

#[tokio::test]
async fn named_introspection() {
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
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!("ANamedEnum")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(values.len(), 2);

        assert!(values.contains(&graphql_value!({
            "name": "FOO",
            "description": null,
            "isDeprecated": false,
            "deprecationReason": null,
        })));

        assert!(values.contains(&graphql_value!({
            "name": "BAR",
            "description": null,
            "isDeprecated": false,
            "deprecationReason": null,
        })));
    })
    .await;
}

#[tokio::test]
async fn no_trailing_comma_introspection() {
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
            Some(&graphql_value!("NoTrailingComma")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(values.len(), 2);

        assert!(values.contains(&graphql_value!({
            "name": "FOO",
            "description": null,
            "isDeprecated": false,
            "deprecationReason": null,
        })));

        assert!(values.contains(&graphql_value!({
            "name": "BAR",
            "description": null,
            "isDeprecated": false,
            "deprecationReason": null,
        })));
    })
    .await;
}

#[tokio::test]
async fn enum_description_introspection() {
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
            Some(&graphql_value!("EnumDescription")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!("A description of the enum itself")),
        );

        assert_eq!(values.len(), 2);

        assert!(values.contains(&graphql_value!({
            "name": "FOO",
            "description": null,
            "isDeprecated": false,
            "deprecationReason": null,
        })));

        assert!(values.contains(&graphql_value!({
            "name": "BAR",
            "description": null,
            "isDeprecated": false,
            "deprecationReason": null,
        })));
    })
    .await;
}

#[tokio::test]
async fn enum_value_description_introspection() {
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
            Some(&graphql_value!("EnumValueDescription")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(values.len(), 2);

        assert!(values.contains(&graphql_value!({
            "name": "FOO",
            "description": "The FOO value",
            "isDeprecated": false,
            "deprecationReason": null,
        })));

        assert!(values.contains(&graphql_value!({
            "name": "BAR",
            "description": "The BAR value",
            "isDeprecated": false,
            "deprecationReason": null,
        })));
    })
    .await;
}

#[tokio::test]
async fn enum_deprecation_introspection() {
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
            Some(&graphql_value!("EnumDeprecation")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(values.len(), 2);

        assert!(values.contains(&graphql_value!({
            "name": "FOO",
            "description": null,
            "isDeprecated": true,
            "deprecationReason": "Please don't use FOO any more",
        })));

        assert!(values.contains(&graphql_value!({
            "name": "BAR",
            "description": "The BAR value",
            "isDeprecated": true,
            "deprecationReason": "Please don't use BAR any more",
        })));
    })
    .await;
}

#[tokio::test]
async fn enum_deprecation_no_values_introspection() {
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
            Some(&graphql_value!("EnumDeprecation")),
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(&graphql_value!(null)),
        );

        assert_eq!(values.len(), 0);
    })
    .await;
}
