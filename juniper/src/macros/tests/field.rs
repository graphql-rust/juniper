use crate::{
    ast::InputValue,
    executor::FieldResult,
    schema::model::RootNode,
    types::scalars::EmptyMutation,
    value::{DefaultScalarValue, Object, Value},
};

struct Interface;
#[derive(Debug)]
struct Root;

/*

Syntax to validate:

* Object vs. interface
* Description vs. no description
* Deprecated vs. not deprecated
* FieldResult vs. object directly
* Return vs. implicit return

*/

#[crate::graphql_object_internal(
    interfaces = [&Interface],
)]
impl Root {
    fn simple() -> i32 {
        0
    }

    /// Field description
    fn description() -> i32 {
        0
    }

    #[deprecated]
    fn deprecated_outer() -> bool {
        true
    }

    #[deprecated(note = "Deprecation Reason")]
    fn deprecated_outer_with_reason() -> bool {
        true
    }

    #[graphql(deprecated = "Deprecation reason")]
    fn deprecated() -> i32 {
        0
    }

    #[graphql(deprecated = "Deprecation reason", description = "Field description")]
    fn deprecated_descr() -> i32 {
        0
    }

    /// Field description
    fn attr_description() -> i32 {
        0
    }

    /// Field description
    /// with `collapse_docs` behavior
    fn attr_description_collapse() -> i32 {
        0
    }

    /// Get the i32 representation of 0.
    ///
    /// - This comment is longer.
    /// - These two lines are rendered as bullets by GraphiQL.
    ///     - subsection
    fn attr_description_long() -> i32 {
        0
    }

    #[graphql(deprecated)]
    fn attr_deprecated() -> i32 {
        0
    }

    #[graphql(deprecated = "Deprecation reason")]
    fn attr_deprecated_reason() -> i32 {
        0
    }

    /// Field description
    #[graphql(deprecated = "Deprecation reason")]
    fn attr_deprecated_descr() -> i32 {
        0
    }

    fn with_field_result() -> FieldResult<i32> {
        Ok(0)
    }

    fn with_return() -> i32 {
        return 0;
    }

    fn with_return_field_result() -> FieldResult<i32> {
        return Ok(0);
    }
}

graphql_interface!(Interface: () |&self| {
    field simple() -> i32 { 0 }

    field description() -> i32 as "Field description" { 0 }

    field deprecated "Deprecation reason"
        deprecated() -> i32 { 0 }

    field deprecated "Deprecation reason"
        deprecated_descr() -> i32 as "Field description" { 0 }

    /// Field description
    field attr_description() -> i32 { 0 }

    /// Field description
    /// with `collapse_docs` behavior
    field attr_description_collapse() -> i32 { 0 }

    /// Get the i32 representation of 0.
    ///
    /// - This comment is longer.
    /// - These two lines are rendered as bullets by GraphiQL.
    field attr_description_long() -> i32 { 0 }

    #[deprecated]
    field attr_deprecated() -> i32 { 0 }

    #[deprecated(note = "Deprecation reason")]
    field attr_deprecated_reason() -> i32 { 0 }

    /// Field description
    #[deprecated(note = "Deprecation reason")]
    field attr_deprecated_descr() -> i32 { 0 }

    instance_resolvers: |&_| {
        Root => Some(Root {}),
    }
});

async fn run_field_info_query<F>(type_name: &str, field_name: &str, f: F)
where
    F: Fn(&Object<DefaultScalarValue>) -> (),
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
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());
    let vars = vec![("typeName".to_owned(), InputValue::scalar(type_name))]
        .into_iter()
        .collect();

    let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
        .await
        .expect("Execution failed");

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
        .expect("fields not a list");

    let field = fields
        .into_iter()
        .filter(|f| {
            f.as_object_value()
                .expect("Field not an object")
                .get_field_value("name")
                .expect("name field missing from field")
                .as_scalar_value::<String>()
                .expect("name is not a string")
                == field_name
        })
        .next()
        .expect("Field not found")
        .as_object_value()
        .expect("Field is not an object");

    println!("Field: {:#?}", field);

    f(field);
}

#[tokio::test]
async fn introspect_object_field_simple() {
    run_field_info_query("Root", "simple", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("simple"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_interface_field_simple() {
    run_field_info_query("Interface", "simple", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("simple"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_description() {
    run_field_info_query("Root", "description", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("description"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Field description"))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_interface_field_description() {
    run_field_info_query("Interface", "description", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("description"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Field description"))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_deprecated_outer() {
    run_field_info_query("Root", "deprecatedOuter", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("deprecatedOuter"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null()),
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_deprecated_outer_with_reason() {
    run_field_info_query("Root", "deprecatedOuterWithReason", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("deprecatedOuterWithReason"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::scalar("Deprecation Reason")),
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_deprecated() {
    run_field_info_query("Root", "deprecated", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("deprecated"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::scalar("Deprecation reason"))
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_interface_field_deprecated() {
    run_field_info_query("Interface", "deprecated", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("deprecated"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::scalar("Deprecation reason"))
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_deprecated_descr() {
    run_field_info_query("Root", "deprecatedDescr", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("deprecatedDescr"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Field description"))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::scalar("Deprecation reason"))
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_interface_field_deprecated_descr() {
    run_field_info_query("Interface", "deprecatedDescr", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("deprecatedDescr"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Field description"))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::scalar("Deprecation reason"))
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_attr_description() {
    run_field_info_query("Root", "attrDescription", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDescription"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Field description"))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_interface_field_attr_description() {
    run_field_info_query("Interface", "attrDescription", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDescription"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Field description"))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_attr_description_long() {
    run_field_info_query("Root", "attrDescriptionLong", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDescriptionLong"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Get the i32 representation of 0.\n\n- This comment is longer.\n- These two lines are rendered as bullets by GraphiQL.\n    - subsection"))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    }).await;
}

#[tokio::test]
async fn introspect_interface_field_attr_description_long() {
    run_field_info_query("Interface", "attrDescriptionLong", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDescriptionLong"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Get the i32 representation of 0.\n\n- This comment is longer.\n- These two lines are rendered as bullets by GraphiQL."))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    }).await;
}

#[tokio::test]
async fn introspect_object_field_attr_description_collapse() {
    run_field_info_query("Root", "attrDescriptionCollapse", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDescriptionCollapse"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar(
                "Field description\nwith `collapse_docs` behavior"
            ))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_interface_field_attr_description_collapse() {
    run_field_info_query("Interface", "attrDescriptionCollapse", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDescriptionCollapse"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar(
                "Field description\nwith `collapse_docs` behavior"
            ))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(false))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_attr_deprecated() {
    run_field_info_query("Root", "attrDeprecated", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDeprecated"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_interface_field_attr_deprecated() {
    run_field_info_query("Interface", "attrDeprecated", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDeprecated"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::null())
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_attr_deprecated_reason() {
    run_field_info_query("Root", "attrDeprecatedReason", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDeprecatedReason"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::scalar("Deprecation reason"))
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_interface_field_attr_deprecated_reason() {
    run_field_info_query("Interface", "attrDeprecatedReason", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDeprecatedReason"))
        );
        assert_eq!(field.get_field_value("description"), Some(&Value::null()));
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::scalar("Deprecation reason"))
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_object_field_attr_deprecated_descr() {
    run_field_info_query("Root", "attrDeprecatedDescr", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDeprecatedDescr"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Field description"))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::scalar("Deprecation reason"))
        );
    })
    .await;
}

#[tokio::test]
async fn introspect_interface_field_attr_deprecated_descr() {
    run_field_info_query("Interface", "attrDeprecatedDescr", |field| {
        assert_eq!(
            field.get_field_value("name"),
            Some(&Value::scalar("attrDeprecatedDescr"))
        );
        assert_eq!(
            field.get_field_value("description"),
            Some(&Value::scalar("Field description"))
        );
        assert_eq!(
            field.get_field_value("isDeprecated"),
            Some(&Value::scalar(true))
        );
        assert_eq!(
            field.get_field_value("deprecationReason"),
            Some(&Value::scalar("Deprecation reason"))
        );
    })
    .await;
}
