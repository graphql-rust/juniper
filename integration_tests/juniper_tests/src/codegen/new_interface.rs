use juniper::{
    execute, graphql_interface, graphql_interface_new, graphql_object, graphql_value, graphql_vars,
    DefaultScalarValue, EmptyMutation, EmptySubscription, Executor, FieldError, FieldResult,
    GraphQLInputObject, GraphQLObject, GraphQLType, IntoFieldError, LookAheadMethods, RootNode,
    ScalarValue,
};

// --------------------------

#[graphql_interface_new(for = [Human, Droid], scalar = S)]
trait Character<S: ScalarValue> {
    async fn id<'a>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str
    where
        S: Send + Sync,
    {
        executor.look_ahead().field_name()
    }

    async fn info<'b>(
        &'b self,
        arg: Option<i32>,
        #[graphql(executor)] another: &Executor<'_, '_, (), S>,
    ) -> &'b str
    where
        S: Send + Sync;
}

struct Human {
    id: String,
    home_planet: String,
}

#[graphql_object(impl = CharacterValue<__S>)]
impl Human {
    async fn id<'a, S: ScalarValue>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str {
        executor.look_ahead().field_name()
    }

    async fn info<'b>(&'b self, _arg: Option<i32>) -> &'b str {
        &self.home_planet
    }
}

struct Droid {
    id: String,
    primary_function: String,
}

#[graphql_object(impl = CharacterValue<__S>)]
impl Droid {
    fn id<'a, S: ScalarValue>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str {
        executor.look_ahead().field_name()
    }

    async fn info<'b, S: ScalarValue>(
        &'b self,
        _arg: Option<i32>,
        _executor: &Executor<'_, '_, (), S>,
    ) -> &'b str {
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
