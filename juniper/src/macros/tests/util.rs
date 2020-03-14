use crate::{DefaultScalarValue, GraphQLTypeAsync, RootNode, Value, Variables};
use std::default::Default;

pub async fn run_query<Query, Mutation, Context>(query: &str) -> Value
where
    Query: GraphQLTypeAsync<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Mutation: GraphQLTypeAsync<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Context: Default + Sync + Send,
{
    let schema = RootNode::new(Query::default(), Mutation::default());
    let (result, errs) =
        crate::execute(query, None, &schema, &Variables::new(), &Context::default())
            .await
            .expect("Execution failed");

    assert_eq!(errs, []);
    result
}

pub async fn run_info_query<Query, Mutation, Context>(type_name: &str) -> Value
where
    Query: GraphQLTypeAsync<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Mutation: GraphQLTypeAsync<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Context: Default + Sync + Send,
{
    let query = format!(
        r#"
    {{
        __type(name: "{}") {{
            name,
            description,
            fields {{
                name
                description
                args {{
                    name
                    description
                    type {{
                        name
                    }}
                }}
            }}
        }}
    }}
    "#,
        type_name
    );
    let result = run_query::<Query, Mutation, Context>(&query).await;
    result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .clone()
}
