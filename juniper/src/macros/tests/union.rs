use std::marker::PhantomData;

use ast::InputValue;
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use value::{Value, Object};

/*

Syntax to validate:

* Order of items: description, instance resolvers
* Optional Generics/lifetimes
* Custom name vs. default name
* Optional commas between items
* Optional trailing commas on instance resolvers

 */

struct Concrete;

enum CustomName {
    Concrete(Concrete),
}

enum WithLifetime<'a> {
    Int(PhantomData<&'a i32>),
}
enum WithGenerics<T> {
    Generic(T),
}

enum DescriptionFirst {
    Concrete(Concrete),
}
enum ResolversFirst {
    Concrete(Concrete),
}

enum CommasWithTrailing {
    Concrete(Concrete),
}
enum ResolversWithTrailingComma {
    Concrete(Concrete),
}

struct Root;

graphql_object!(Concrete: () |&self| {
    field simple() -> i32 { 123 }
});

graphql_union!(CustomName: () as "ACustomNamedUnion" |&self| {
    instance_resolvers: |&_| {
        &Concrete => match *self { CustomName::Concrete(ref c) => Some(c) }
    }
});

graphql_union!(<'a> WithLifetime<'a>: () as "WithLifetime" |&self| {
    instance_resolvers: |&_| {
        Concrete => match *self { WithLifetime::Int(_) => Some(Concrete) }
    }
});

graphql_union!(<T> WithGenerics<T>: () as "WithGenerics" |&self| {
    instance_resolvers: |&_| {
        Concrete => match *self { WithGenerics::Generic(_) => Some(Concrete) }
    }
});

graphql_union!(DescriptionFirst: () |&self| {
    description: "A description"
    instance_resolvers: |&_| {
        &Concrete => match *self { DescriptionFirst::Concrete(ref c) => Some(c) }
    }
});

graphql_union!(ResolversFirst: () |&self| {
    instance_resolvers: |&_| {
        &Concrete => match *self { ResolversFirst::Concrete(ref c) => Some(c) }
    }
    description: "A description"
});

graphql_union!(CommasWithTrailing: () |&self| {
    instance_resolvers: |&_| {
        &Concrete => match *self { CommasWithTrailing::Concrete(ref c) => Some(c) }
    },
    description: "A description",
});

graphql_union!(ResolversWithTrailingComma: () |&self| {
    instance_resolvers: |&_| {
        &Concrete => match *self { ResolversWithTrailingComma::Concrete(ref c) => Some(c) },
    }
    description: "A description"
});

graphql_object!(<'a> Root: () as "Root" |&self| {
    field custom_name() -> CustomName { CustomName::Concrete(Concrete) }
    field with_lifetime() -> WithLifetime<'a> { WithLifetime::Int(PhantomData) }
    field with_generics() -> WithGenerics<i32> { WithGenerics::Generic(123) }
    field description_first() -> DescriptionFirst { DescriptionFirst::Concrete(Concrete) }
    field resolvers_first() -> ResolversFirst { ResolversFirst::Concrete(Concrete) }
    field commas_with_trailing() -> CommasWithTrailing { CommasWithTrailing::Concrete(Concrete) }
    field resolvers_with_trailing_comma() -> ResolversWithTrailingComma {
        ResolversWithTrailingComma::Concrete(Concrete)
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
            possibleTypes {
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

    let possible_types = type_info
        .get_field_value("possibleTypes")
        .expect("possibleTypes field missing")
        .as_list_value()
        .expect("possibleTypes field not a list value");

    f(type_info, possible_types);
}

#[test]
fn introspect_custom_name() {
    run_type_info_query("ACustomNamedUnion", |union, possible_types| {
        assert_eq!(union.get_field_value("name"), Some(&Value::string("ACustomNamedUnion")));
        assert_eq!(union.get_field_value("description"), Some(&Value::null()));

        assert!(
            possible_types.contains(&Value::object(
                vec![("name", Value::string("Concrete"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_with_lifetime() {
    run_type_info_query("WithLifetime", |union, possible_types| {
        assert_eq!(union.get_field_value("name"), Some(&Value::string("WithLifetime")));
        assert_eq!(union.get_field_value("description"), Some(&Value::null()));

        assert!(
            possible_types.contains(&Value::object(
                vec![("name", Value::string("Concrete"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_with_generics() {
    run_type_info_query("WithGenerics", |union, possible_types| {
        assert_eq!(union.get_field_value("name"), Some(&Value::string("WithGenerics")));
        assert_eq!(union.get_field_value("description"), Some(&Value::null()));

        assert!(
            possible_types.contains(&Value::object(
                vec![("name", Value::string("Concrete"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_description_first() {
    run_type_info_query("DescriptionFirst", |union, possible_types| {
        assert_eq!(union.get_field_value("name"), Some(&Value::string("DescriptionFirst")));
        assert_eq!(
            union.get_field_value("description"),
            Some(&Value::string("A description"))
        );

        assert!(
            possible_types.contains(&Value::object(
                vec![("name", Value::string("Concrete"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_resolvers_first() {
    run_type_info_query("ResolversFirst", |union, possible_types| {
        assert_eq!(union.get_field_value("name"), Some(&Value::string("ResolversFirst")));
        assert_eq!(
            union.get_field_value("description"),
            Some(&Value::string("A description"))
        );

        assert!(
            possible_types.contains(&Value::object(
                vec![("name", Value::string("Concrete"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_commas_with_trailing() {
    run_type_info_query("CommasWithTrailing", |union, possible_types| {
        assert_eq!(
            union.get_field_value("name"),
            Some(&Value::string("CommasWithTrailing"))
        );
        assert_eq!(
            union.get_field_value("description"),
            Some(&Value::string("A description"))
        );

        assert!(
            possible_types.contains(&Value::object(
                vec![("name", Value::string("Concrete"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_resolvers_with_trailing_comma() {
    run_type_info_query("ResolversWithTrailingComma", |union, possible_types| {
        assert_eq!(
            union.get_field_value("name"),
            Some(&Value::string("ResolversWithTrailingComma"))
        );
        assert_eq!(
            union.get_field_value("description"),
            Some(&Value::string("A description"))
        );

        assert!(
            possible_types.contains(&Value::object(
                vec![("name", Value::string("Concrete"))]
                    .into_iter()
                    .collect(),
            ))
        );
    });
}
