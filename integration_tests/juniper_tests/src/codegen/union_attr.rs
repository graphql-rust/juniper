use juniper::{graphql_object, graphql_union, GraphQLObject};

#[cfg(test)]
use juniper::{
    self, execute, DefaultScalarValue, EmptyMutation, EmptySubscription, GraphQLType, RootNode,
    Value, Variables,
};

#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[graphql_union(name = "Character")]
#[graphql_union(description = "A Collection of things")]
trait Character<T> {
    fn as_human(&self, _: &()) -> Option<&Human> {
        None
    }
    fn as_droid(&self) -> Option<&Droid> {
        None
    }
    #[graphql_union(ignore)]
    fn some(&self) {}
}

impl<T> Character<T> for Human {
    fn as_human(&self, _: &()) -> Option<&Human> {
        Some(&self)
    }
}

impl<T> Character<T> for Droid {
    fn as_droid(&self) -> Option<&Droid> {
        Some(&self)
    }
}

pub struct Query {
    is_human: bool,
}

#[graphql_object]
impl Query {
    fn context(&self) -> Box<dyn Character<()> + Send + Sync> {
        let ch: Box<dyn Character<()> + Send + Sync> = if self.is_human {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            })
        } else {
            Box::new(Droid {
                id: "droid-99".to_string(),
                primary_function: "run".to_string(),
            })
        };
        ch
    }
}

const DOC: &str = r#"
{
    context {
        ... on Human {
            humanId: id
            homePlanet
        }
        ... on Droid {
            droidId: id
            primaryFunction
        }
    }
}"#;

#[tokio::test]
async fn resolves_human() {
    let schema = RootNode::new(
        Query { is_human: true },
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let actual = execute(DOC, None, &schema, &Variables::new(), &()).await;

    let expected = Ok((
        Value::object(
            vec![(
                "context",
                Value::object(
                    vec![
                        ("humanId", Value::scalar("human-32".to_string())),
                        ("homePlanet", Value::scalar("earth".to_string())),
                    ]
                    .into_iter()
                    .collect(),
                ),
            )]
            .into_iter()
            .collect(),
        ),
        vec![],
    ));

    assert_eq!(actual, expected);
}

#[tokio::test]
async fn resolves_droid() {
    let schema = RootNode::new(
        Query { is_human: false },
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let actual = execute(DOC, None, &schema, &Variables::new(), &()).await;

    let expected = Ok((
        Value::object(
            vec![(
                "context",
                Value::object(
                    vec![
                        ("droidId", Value::scalar("droid-99".to_string())),
                        ("primaryFunction", Value::scalar("run".to_string())),
                    ]
                    .into_iter()
                    .collect(),
                ),
            )]
            .into_iter()
            .collect(),
        ),
        vec![],
    ));

    assert_eq!(actual, expected);
}
