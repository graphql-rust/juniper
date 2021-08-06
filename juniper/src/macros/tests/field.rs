/*

Syntax to validate:

* Object vs. interface
* Description vs. no description
* Deprecated vs. not deprecated
* FieldResult vs. object directly
* Return vs. implicit return

*/

#![allow(deprecated)]

use crate::{
    ast::InputValue,
    executor::FieldResult,
    graphql_interface, graphql_object,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    value::{DefaultScalarValue, Object, Value},
};

#[derive(Debug)]
struct Root;

#[graphql_object(impl = InterfaceValue)]
impl Root {
    /// Get the i32 representation of 0.
    ///
    /// - This comment is longer.
    /// - These two lines are rendered as bullets by GraphiQL.
    ///     - subsection
    fn attr_description_long() -> i32 {
        0
    }

    fn with_field_result() -> FieldResult<i32> {
        Ok(0)
    }

    fn with_return_field_result() -> FieldResult<i32> {
        Ok(0)
    }
}

#[graphql_interface]
impl Interface for Root {
    fn attr_description_long(&self) -> i32 {
        0
    }
}

#[graphql_interface(for = Root)]
trait Interface {
    /// Get the i32 representation of 0.
    ///
    /// - This comment is longer.
    /// - These two lines are rendered as bullets by GraphiQL.
    fn attr_description_long(&self) -> i32;
}

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
    let schema = RootNode::new(
        Root {},
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );
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
