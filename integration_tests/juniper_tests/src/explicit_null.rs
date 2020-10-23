use juniper::*;

pub struct Context;

impl juniper::Context for Context {}

pub struct Query;

#[derive(juniper::GraphQLInputObject)]
struct ObjectInput {
    field: Nullable<i32>,
}

#[graphql_object(Context=Context)]
impl Query {
    fn is_explicit_null(arg: Nullable<i32>) -> bool {
        arg.is_explicit_null()
    }

    fn object_field_is_explicit_null(obj: ObjectInput) -> bool {
        obj.field.is_explicit_null()
    }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

#[tokio::test]
async fn explicit_null() {
    let ctx = Context;

    let query = r#"
    query Foo($emptyObj: ObjectInput!, $literalNullObj: ObjectInput!) {
        literalOneIsExplicitNull: isExplicitNull(arg: 1)
        literalNullIsExplicitNull: isExplicitNull(arg: null)
        noArgIsExplicitNull: isExplicitNull
        literalOneFieldIsExplicitNull: objectFieldIsExplicitNull(obj: {field: 1})
        literalNullFieldIsExplicitNull: objectFieldIsExplicitNull(obj: {field: null})
        noFieldIsExplicitNull: objectFieldIsExplicitNull(obj: {})
        emptyVariableObjectFieldIsExplicitNull: objectFieldIsExplicitNull(obj: $emptyObj)
        literalNullVariableObjectFieldIsExplicitNull: objectFieldIsExplicitNull(obj: $literalNullObj)
    }
    "#;

    let (data, errors) = juniper::execute(
        query,
        None,
        &Schema::new(
            Query,
            EmptyMutation::<Context>::new(),
            EmptySubscription::<Context>::new(),
        ),
        &[
            ("emptyObj".to_string(), InputValue::Object(vec![])),
            (
                "literalNullObj".to_string(),
                InputValue::object(vec![("field", InputValue::null())].into_iter().collect()),
            ),
        ]
        .iter()
        .cloned()
        .collect(),
        &ctx,
    )
    .await
    .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        data,
        graphql_value!({
            "literalOneIsExplicitNull": false,
            "literalNullIsExplicitNull": true,
            "noArgIsExplicitNull": false,
            "literalOneFieldIsExplicitNull": false,
            "literalNullFieldIsExplicitNull": true,
            "noFieldIsExplicitNull": false,
            "emptyVariableObjectFieldIsExplicitNull": false,
            "literalNullVariableObjectFieldIsExplicitNull": true,
        })
    );
}
