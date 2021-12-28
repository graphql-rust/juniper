use juniper::{
    execute, graphql_interface, graphql_interface_new, graphql_object, graphql_value, graphql_vars,
    DefaultScalarValue, EmptyMutation, EmptySubscription, Executor, FieldError, FieldResult,
    GraphQLInputObject, GraphQLObject, GraphQLType, IntoFieldError, RootNode, ScalarValue,
};

// --------------------------

#[graphql_interface(for = [Human, Droid])]
trait Character<'me, A> {
    async fn id<'a>(&'a self) -> &'a str;
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue<()>)]
struct Human {
    id: String,
    home_planet: String,
}

struct Droid {
    id: String,
    primary_function: String,
}

#[graphql_object(impl = CharacterValue<()>)]
impl Droid {
    fn id(&self) -> &str {
        &self.id
    }

    fn primary_function(&self) -> &str {
        &self.primary_function
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
