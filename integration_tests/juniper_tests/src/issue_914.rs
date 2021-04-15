use juniper::*;

struct Query;

#[derive(GraphQLObject)]
struct Foo {
    bar: Bar,
}

#[derive(GraphQLObject)]
struct Bar {
    a: i32,
    b: i32,
}

#[graphql_object]
impl Query {
    fn foo() -> Foo {
        let bar = Bar { a: 1, b: 2 };
        Foo { bar }
    }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation, EmptySubscription>;

#[tokio::test]
async fn test_fragments_on_nexted_types_dont_override_previous_selections() {
    let query = r#"
        query Query {
            foo {
                ...BarA
                ...BarB
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
    "#;

    let (async_value, errors) = juniper::execute(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .await
    .unwrap();
    assert_eq!(errors.len(), 0);

    let (sync_value, errors) = juniper::execute_sync(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .unwrap();
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
}
