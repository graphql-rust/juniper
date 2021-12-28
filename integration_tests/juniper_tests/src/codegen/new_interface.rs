use juniper::{
    execute, graphql_interface, graphql_interface_new, graphql_object, graphql_value, graphql_vars,
    DefaultScalarValue, EmptyMutation, EmptySubscription, Executor, FieldError, FieldResult,
    GraphQLInputObject, GraphQLObject, GraphQLType, IntoFieldError, RootNode, ScalarValue,
};

#[graphql_interface_new(for = [Human, Droid])]
trait Character {
    fn id(&self, required: String) -> String;
}

struct Human {
    id: String,
    home_planet: String,
}

#[graphql_object(impl = CharacterValue)]
impl Human {
    fn id(&self, _required: String, _optional: Option<String>) -> &str {
        &self.id
    }

    fn home_planet(&self) -> &str {
        &self.home_planet
    }
}

struct Droid {
    id: String,
    primary_function: String,
}

#[graphql_object(impl = CharacterValue)]
impl Droid {
    fn id(&self, _required: String) -> &str {
        &self.id
    }

    fn primary_function(&self) -> &str {
        &self.primary_function
    }
}

// -----

#[derive(Clone, Copy)]
enum QueryRoot {
    Human,
    Droid,
}

#[graphql_object(scalar = S: ScalarValue + Send + Sync)]
impl QueryRoot {
    fn character(&self) -> CharacterValue {
        match self {
            Self::Human => Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            }
            .into(),
            Self::Droid => Droid {
                id: "droid-99".to_string(),
                primary_function: "run".to_string(),
            }
            .into(),
        }
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

#[tokio::test]
async fn enum_resolves_human() {
    const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id(required: "test")
                    homePlanet
                }
            }
        }"#;

    let schema = schema(QueryRoot::Human);

    assert_eq!(
        execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
        Ok((
            graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn enum_resolves_droid() {
    const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id(required: "test")
                    primaryFunction
                }
            }
        }"#;

    let schema = schema(QueryRoot::Droid);

    assert_eq!(
        execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
        Ok((
            graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
            vec![],
        )),
    );
}
