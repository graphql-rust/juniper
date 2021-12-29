use juniper::{
    execute, graphql_interface, graphql_interface_new, graphql_object, graphql_value, graphql_vars,
    DefaultScalarValue, EmptyMutation, EmptySubscription, Executor, FieldError, FieldResult,
    GraphQLInputObject, GraphQLObject, GraphQLType, IntoFieldError, RootNode, ScalarValue,
};

// --------------------------

#[derive(GraphQLInputObject, Debug)]
struct Point {
    x: i32,
}

#[graphql_interface_new(for = Human)]
trait Character {
    async fn id(
        &self,
        #[graphql(default)] first: String,
        #[graphql(default = "second".to_string())] second: String,
        #[graphql(default = "t")] third: String,
    ) -> String;

    fn info(&self, #[graphql(default = Point { x: 1 })] coord: Point) -> i32 {
        coord.x
    }
}

struct Human {
    id: String,
    info: i32,
}

#[graphql_object(impl = CharacterValue)]
impl Human {
    fn info(&self, _coord: Point) -> i32 {
        self.info
    }

    async fn id(&self, first: String, second: String, third: String) -> String {
        format!("{}|{}&{}", first, second, third)
    }
}

struct QueryRoot;

#[graphql_object]
impl QueryRoot {
    fn character(&self) -> CharacterValue {
        Human {
            id: "human-32".to_string(),
            info: 0,
        }
        .into()
    }
}

// --------------

fn schema<'q, C, Q>(query_root: Q) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>>
where
    Q: GraphQLType<DefaultScalarValue, Context = C, TypeInfo = ()> + 'q,
{
    RootNode::new(
        query_root,
        EmptyMutation::<C>::new(),
        EmptySubscription::<C>::new(),
    )
}

fn schema_with_scalar<'q, S, C, Q>(
    query_root: Q,
) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>, S>
where
    Q: GraphQLType<S, Context = C, TypeInfo = ()> + 'q,
    S: ScalarValue + 'q,
{
    RootNode::new_with_scalar_value(
        query_root,
        EmptyMutation::<C>::new(),
        EmptySubscription::<C>::new(),
    )
}
