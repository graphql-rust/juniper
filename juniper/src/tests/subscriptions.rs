use std::{iter, pin::Pin};

use futures::{stream, StreamExt as _};

use crate::{
    graphql_object, graphql_subscription, graphql_value, http::GraphQLRequest, Context,
    DefaultScalarValue, EmptyMutation, ExecutionError, FieldError, GraphQLObject, Object, RootNode,
    Value,
};

#[derive(Debug, Clone)]
pub struct MyContext(i32);
impl Context for MyContext {}

#[derive(GraphQLObject)]
#[graphql(description = "A humanoid creature in the Star Wars universe")]
#[derive(Clone)]
struct Human {
    id: String,
    name: String,
    home_planet: String,
}

struct MyQuery;

#[graphql_object(context = MyContext)]
impl MyQuery {
    fn test(&self) -> i32 {
        0 // NOTICE: does not serve a purpose
    }
}

type Schema =
    RootNode<'static, MyQuery, EmptyMutation<MyContext>, MySubscription, DefaultScalarValue>;

fn run<O>(f: impl std::future::Future<Output = O>) -> O {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(f)
}

type HumanStream = Pin<Box<dyn futures::Stream<Item = Human> + Send>>;

struct MySubscription;

#[graphql_subscription(context = MyContext)]
impl MySubscription {
    async fn async_human() -> HumanStream {
        Box::pin(stream::once(async {
            Human {
                id: "stream id".into(),
                name: "stream name".into(),
                home_planet: "stream home planet".into(),
            }
        }))
    }

    async fn error_human() -> Result<HumanStream, FieldError> {
        Err(FieldError::new(
            "handler error",
            graphql_value!("more details"),
        ))
    }

    async fn human_with_context(context: &MyContext) -> HumanStream {
        let context_val = context.0;
        Box::pin(stream::once(async move {
            Human {
                id: context_val.to_string(),
                name: context_val.to_string(),
                home_planet: context_val.to_string(),
            }
        }))
    }

    async fn human_with_args(id: String, name: String) -> HumanStream {
        Box::pin(stream::once(async {
            Human {
                id,
                name,
                home_planet: "default home planet".into(),
            }
        }))
    }
}

/// Create all variables, execute subscription
/// and collect returned iterators.
/// Panics if query is invalid (GraphQLError is returned)
fn create_and_execute(
    query: String,
) -> Result<
    (
        Vec<String>,
        Vec<Vec<Result<Value<DefaultScalarValue>, ExecutionError<DefaultScalarValue>>>>,
    ),
    Vec<ExecutionError<DefaultScalarValue>>,
> {
    let request = GraphQLRequest::new(query, None, None);

    let root_node = Schema::new(MyQuery, EmptyMutation::new(), MySubscription);

    let context = MyContext(2);

    let response = run(crate::http::resolve_into_stream(
        &request, &root_node, &context,
    ));

    assert!(response.is_ok());

    let (values, errors) = response.unwrap();

    if !errors.is_empty() {
        return Err(errors);
    }

    // cannot compare with `assert_eq` because
    // stream does not implement Debug
    let response_value_object = match values {
        Value::Object(o) => Some(o),
        _ => None,
    };

    assert!(response_value_object.is_some());

    let response_returned_object = response_value_object.unwrap();

    let fields = response_returned_object.into_iter();

    let mut names = vec![];
    let mut collected_values = vec![];

    for (name, stream_val) in fields {
        names.push(name.clone());

        // since macro returns Value::Scalar(iterator) every time,
        // other variants may be skipped
        match stream_val {
            Value::Scalar(stream) => {
                let collected = run(stream.collect::<Vec<_>>());
                collected_values.push(collected);
            }
            _ => unreachable!(),
        }
    }

    Ok((names, collected_values))
}

#[test]
fn returns_requested_object() {
    let query = r#"subscription {
        asyncHuman(id: "1") {
            id
            name
        }
    }"#;

    let (names, collected_values) =
        create_and_execute(query.into()).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(
        std::iter::from_fn(move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some(("id", graphql_value!("stream id"))),
                2 => Some(("name", graphql_value!("stream name"))),
                _ => None,
            }
        }),
    )))]];

    assert_eq!(names, vec!["asyncHuman"]);
    assert_eq!(collected_values, expected_values);
}

#[test]
fn returns_error() {
    let query = r#"subscription {
        errorHuman(id: "1") {
            id
            name
        }
    }"#;

    let response = create_and_execute(query.into());

    assert!(response.is_err());

    let returned_errors = response.err().unwrap();

    let expected_error = ExecutionError::new(
        crate::parser::SourcePosition::new(23, 1, 8),
        &["errorHuman"],
        FieldError::new("handler error", graphql_value!("more details")),
    );

    assert_eq!(returned_errors, vec![expected_error]);
}

#[test]
fn can_access_context() {
    let query = r#"subscription {
            humanWithContext {
                id
              }
        }"#;

    let (names, collected_values) =
        create_and_execute(query.into()).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some(("id", graphql_value!("2"))),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["humanWithContext"]);
    assert_eq!(collected_values, expected_values);
}

#[test]
fn resolves_typed_inline_fragments() {
    let query = r#"subscription {
             ... on MySubscription {
                asyncHuman(id: "32") {
                  id
                }
             }
           }"#;

    let (names, collected_values) =
        create_and_execute(query.into()).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some(("id", graphql_value!("stream id"))),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["asyncHuman"]);
    assert_eq!(collected_values, expected_values);
}

#[test]
fn resolves_nontyped_inline_fragments() {
    let query = r#"subscription {
             ... {
                asyncHuman(id: "32") {
                  id
                }
             }
           }"#;

    let (names, collected_values) =
        create_and_execute(query.into()).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some(("id", graphql_value!("stream id"))),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["asyncHuman"]);
    assert_eq!(collected_values, expected_values);
}

#[test]
fn can_access_arguments() {
    let query = r#"subscription {
            humanWithArgs(id: "123", name: "args name") {
                id
                name
              }
        }"#;

    let (names, collected_values) =
        create_and_execute(query.into()).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some(("id", graphql_value!("123"))),
                2 => Some(("name", graphql_value!("args name"))),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["humanWithArgs"]);
    assert_eq!(collected_values, expected_values);
}

#[test]
fn type_alias() {
    let query = r#"subscription {
        aliasedHuman: asyncHuman(id: "1") {
            id
            name
        }
    }"#;

    let (names, collected_values) =
        create_and_execute(query.into()).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some(("id", graphql_value!("stream id"))),
                2 => Some(("name", graphql_value!("stream name"))),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["aliasedHuman"]);
    assert_eq!(collected_values, expected_values);
}
