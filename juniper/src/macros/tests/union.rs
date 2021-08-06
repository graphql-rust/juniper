/*

Syntax to validate:

* Order of items: description, instance resolvers
* Optional Generics/lifetimes
* Custom name vs. default name
* Optional commas between items
* Optional trailing commas on instance resolvers
*
*/

use std::marker::PhantomData;

use crate::{
    ast::InputValue,
    graphql_object,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    value::{DefaultScalarValue, Object, Value},
    GraphQLUnion,
};

struct Concrete;

#[graphql_object]
impl Concrete {
    fn simple() -> i32 {
        123
    }
}

#[derive(GraphQLUnion)]
#[graphql(on Concrete = WithLifetime::resolve)]
enum WithLifetime<'a> {
    #[graphql(ignore)]
    Int(PhantomData<&'a i32>),
}

impl<'a> WithLifetime<'a> {
    fn resolve(&self, _: &()) -> Option<&Concrete> {
        if matches!(self, Self::Int(_)) {
            Some(&Concrete)
        } else {
            None
        }
    }
}

struct Root;

#[graphql_object]
impl Root {
    fn with_lifetime(&self) -> WithLifetime<'_> {
        WithLifetime::Int(PhantomData)
    }
}

async fn run_type_info_query<F>(type_name: &str, f: F)
where
    F: Fn(&Object<DefaultScalarValue>, &Vec<Value<DefaultScalarValue>>) -> (),
{
    let doc = r#"
    query ($typeName: String!) {
        __type(name: $typeName) {
            name
            description
            possibleTypes {
                name
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

    let possible_types = type_info
        .get_field_value("possibleTypes")
        .expect("possibleTypes field missing")
        .as_list_value()
        .expect("possibleTypes field not a list value");

    f(type_info, possible_types);
}

#[tokio::test]
async fn introspect_with_lifetime() {
    run_type_info_query("WithLifetime", |union, possible_types| {
        assert_eq!(
            union.get_field_value("name"),
            Some(&Value::scalar("WithLifetime"))
        );
        assert_eq!(union.get_field_value("description"), Some(&Value::null()));

        assert!(possible_types.contains(&Value::object(
            vec![("name", Value::scalar("Concrete"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}
