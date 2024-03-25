//! Checks that multiple fragments on sub types don't override each other.
//! See [#914](https://github.com/graphql-rust/juniper/issues/914) for details.

#![allow(clippy::disallowed_names)]

use juniper::{graphql_object, graphql_vars, EmptyMutation, EmptySubscription, GraphQLObject};

struct Query;

#[derive(GraphQLObject)]
struct Foo {
    bar: Bar,
}

#[derive(GraphQLObject)]
struct Bar {
    a: i32,
    b: i32,
    baz: Baz,
}

#[derive(GraphQLObject)]
struct Baz {
    c: i32,
    d: i32,
}

#[graphql_object]
impl Query {
    fn foo() -> Foo {
        let baz = Baz { c: 1, d: 2 };
        let bar = Bar { a: 1, b: 2, baz };
        Foo { bar }
    }
}

type Schema = juniper::RootNode<Query, EmptyMutation, EmptySubscription>;

#[tokio::test]
async fn fragments_with_nested_objects_dont_override_previous_selections() {
    let query = r#"
        query Query {
            foo {
                ...BarA
                ...BarB
                ...BazC
                ...BazD
            }
        }

        fragment BarA on Foo {
            bar {
                a
            }
        }

        fragment BarB on Foo {
            bar {
                b
            }
        }

        fragment BazC on Foo {
            bar {
                baz {
                    c
                }
            }
        }

        fragment BazD on Foo {
            bar {
                baz {
                    d
                }
            }
        }
    "#;

    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (async_value, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();
    assert_eq!(errors.len(), 0);

    let (sync_value, errors) =
        juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()).unwrap();
    assert_eq!(errors.len(), 0);

    assert_eq!(async_value, sync_value);

    let bar = async_value
        .as_object_value()
        .unwrap()
        .get_field_value("foo")
        .unwrap()
        .as_object_value()
        .unwrap()
        .get_field_value("bar")
        .unwrap()
        .as_object_value()
        .unwrap();
    assert!(bar.contains_field("a"), "Field a should be selected");
    assert!(bar.contains_field("b"), "Field b should be selected");

    let baz = bar
        .get_field_value("baz")
        .unwrap()
        .as_object_value()
        .unwrap();
    assert!(baz.contains_field("c"), "Field c should be selected");
    assert!(baz.contains_field("d"), "Field d should be selected");
}
