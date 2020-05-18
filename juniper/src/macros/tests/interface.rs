use std::marker::PhantomData;

use crate::{
    ast::InputValue,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    value::{DefaultScalarValue, Object, Value},
};

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
struct WithGenerics<T: Send + Sync> {
    data: T,
}

struct DescriptionFirst;
struct FieldsFirst;
struct InterfacesFirst;

struct CommasWithTrailing;
struct CommasOnMeta;

struct ResolversWithTrailingComma;

struct Root;

#[crate::graphql_object_internal]
impl Concrete {
    async fn simple() -> i32 {
        0
    }
}

graphql_interface!(CustomName: () as "ACustomNamedInterface" |&self| {
    field simple() -> i32 { 0 }

    instance_resolvers: |_| { Concrete => Some(Concrete) }
});

graphql_interface!(<'a> WithLifetime<'a>: () as "WithLifetime" |&self| {
    field simple() -> i32 { 0 }
    instance_resolvers: |_| { Concrete => Some(Concrete) }
});

// FIXME: rewrite interface in proc-macro
// graphql_interface!(<T: Send + Sync> WithGenerics<T>: () as "WithGenerics" |&self| {
//     field simple() -> i32 { 0 }
//     instance_resolvers: |_| { Concrete => Some(Concrete) }
// });

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

#[crate::graphql_object_internal]
impl<'a> Root {
    async fn custom_name() -> CustomName {
        CustomName {}
    }

    async fn with_lifetime() -> WithLifetime<'a> {
        WithLifetime { data: PhantomData }
    }

    // FIXME: rewrite interface in proc-macro
    // async fn with_generics() -> WithGenerics<i32> {
    //     WithGenerics { data: 123 }
    // }

    async fn description_first() -> DescriptionFirst {
        DescriptionFirst {}
    }
    async fn fields_first() -> FieldsFirst {
        FieldsFirst {}
    }
    async fn interfaces_first() -> InterfacesFirst {
        InterfacesFirst {}
    }

    async fn commas_with_trailing() -> CommasWithTrailing {
        CommasWithTrailing {}
    }
    async fn commas_on_meta() -> CommasOnMeta {
        CommasOnMeta {}
    }

    async fn resolvers_with_trailing_comma() -> ResolversWithTrailingComma {
        ResolversWithTrailingComma {}
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
            fields(includeDeprecated: true) {
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

    let fields = type_info
        .get_field_value("fields")
        .expect("fields field missing")
        .as_list_value()
        .expect("fields field not a list value");

    f(type_info, fields);
}

#[tokio::test]
async fn introspect_custom_name() {
    run_type_info_query("ACustomNamedInterface", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("ACustomNamedInterface"))
        );
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn introspect_with_lifetime() {
    run_type_info_query("WithLifetime", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("WithLifetime"))
        );
        assert_eq!(object.get_field_value("description"), Some(&Value::null()));

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

// FIXME: rewrite interface in proc-macro
// #[tokio::test]
// async fn introspect_with_generics() {
//     run_type_info_query("WithGenerics", |object, fields| {
//         assert_eq!(
//             object.get_field_value("name"),
//             Some(&Value::scalar("WithGenerics"))
//         );
//         assert_eq!(object.get_field_value("description"), Some(&Value::null()));

//         assert!(fields.contains(&Value::object(
//             vec![("name", Value::scalar("simple"))]
//                 .into_iter()
//                 .collect(),
//         )));
//     })
//     .await;
// }

#[tokio::test]
async fn introspect_description_first() {
    run_type_info_query("DescriptionFirst", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("DescriptionFirst"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::scalar("A description"))
        );

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn introspect_fields_first() {
    run_type_info_query("FieldsFirst", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("FieldsFirst"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::scalar("A description"))
        );

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn introspect_interfaces_first() {
    run_type_info_query("InterfacesFirst", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("InterfacesFirst"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::scalar("A description"))
        );

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn introspect_commas_with_trailing() {
    run_type_info_query("CommasWithTrailing", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("CommasWithTrailing"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::scalar("A description"))
        );

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn introspect_commas_on_meta() {
    run_type_info_query("CommasOnMeta", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("CommasOnMeta"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::scalar("A description"))
        );

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}

#[tokio::test]
async fn introspect_resolvers_with_trailing_comma() {
    run_type_info_query("ResolversWithTrailingComma", |object, fields| {
        assert_eq!(
            object.get_field_value("name"),
            Some(&Value::scalar("ResolversWithTrailingComma"))
        );
        assert_eq!(
            object.get_field_value("description"),
            Some(&Value::scalar("A description"))
        );

        assert!(fields.contains(&Value::object(
            vec![("name", Value::scalar("simple"))]
                .into_iter()
                .collect(),
        )));
    })
    .await;
}
