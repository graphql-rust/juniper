use crate::{DefaultScalarValue, GraphQLType, RootNode, Value, Variables};
use std::default::Default;

pub fn run_query<Query, Mutation, Subscription, Context>(query: &str) -> Value
where
    Query: GraphQLType<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Mutation: GraphQLType<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Subscription: GraphQLType<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Context: Default + Send + Sync,
{
    let schema = RootNode::new(
        Query::default(),
        Mutation::default(),
        Subscription::default()
    );
    let (result, errs) =
        crate::execute(query, None, &schema, &Variables::new(), &Context::default())
            .expect("Execution failed");

    assert_eq!(errs, []);
    result
}

pub fn run_info_query<Query, Mutation, Subscription, Context>(type_name: &str) -> Value
where
    Query: GraphQLType<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Mutation: GraphQLType<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Subscription: GraphQLType<DefaultScalarValue, TypeInfo = (), Context = Context> + Default,
    Context: Default + Send + Sync,
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
    let result = run_query
        ::<Query, Mutation, Subscription, Context>(&query);
    result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .clone()
}
