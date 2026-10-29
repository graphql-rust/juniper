use std::{iter, pin::Pin};

use futures::{StreamExt as _, stream};

use crate::{
    Context, DefaultScalarValue, EmptyMutation, ExecutionError, FieldError, GraphQLObject, Object,
    RootNode, Value, graphql, graphql_object, graphql_subscription, http::GraphQLRequest,
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

type Schema = RootNode<MyQuery, EmptyMutation<MyContext>, MySubscription, DefaultScalarValue>;

fn run<O>(f: impl Future<Output = O>) -> O {
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
            graphql::value!("more details"),
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

/// Errors of a subscription execution or of a single stream item.
type ExecutionErrors = Vec<ExecutionError<DefaultScalarValue>>;

/// Items collected from the stream of a single subscription field.
type StreamItems = Vec<Result<Value<DefaultScalarValue>, ExecutionErrors>>;

/// Executes the subscription `query` and collects the items of each field stream.
///
/// Returns the field names and the collected stream items in the same order. Returns the
/// execution errors if the response contains errors. Panics if the query is invalid or if the
/// response value is not an object of streams.
fn create_and_execute(query: String) -> Result<(Vec<String>, Vec<StreamItems>), ExecutionErrors> {
    let request = GraphQLRequest::new(query, None, None);
    let root_node = Schema::new(MyQuery, EmptyMutation::new(), MySubscription);
    let context = MyContext(2);

    let (value, errors) = run(crate::http::resolve_into_stream(
        &request, &root_node, &context,
    ))
    .expect("subscription query should be valid");
    if !errors.is_empty() {
        return Err(errors);
    }
    let Value::Object(fields) = value else {
        panic!("subscription response value should be an object");
    };

    let mut names = vec![];
    let mut items = vec![];
    for (name, value) in fields {
        let Value::Scalar(stream) = value else {
            panic!("subscription field `{name}` should resolve to a stream");
        };
        names.push(name);
        items.push(run(stream.collect::<Vec<_>>()));
    }
    Ok((names, items))
}

#[test]
fn returns_requested_object() {
    let query = r#"subscription {
        asyncHuman {
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
                1 => Some(("id", graphql::value!("stream id"))),
                2 => Some(("name", graphql::value!("stream name"))),
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
        errorHuman {
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
        FieldError::new("handler error", graphql::value!("more details")),
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
                1 => Some(("id", graphql::value!("2"))),
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
                asyncHuman {
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
                1 => Some(("id", graphql::value!("stream id"))),
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
                asyncHuman {
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
                1 => Some(("id", graphql::value!("stream id"))),
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
                1 => Some(("id", graphql::value!("123"))),
                2 => Some(("name", graphql::value!("args name"))),
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
        aliasedHuman: asyncHuman {
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
                1 => Some(("id", graphql::value!("stream id"))),
                2 => Some(("name", graphql::value!("stream name"))),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["aliasedHuman"]);
    assert_eq!(collected_values, expected_values);
}
