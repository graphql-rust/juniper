//! Helper utilities to use in tests all over the crate.

use std::pin::Pin;

use futures::{future, stream, StreamExt as _};

use crate::{ExecutionError, GraphQLError, ScalarValue, Value, ValuesStream};

/// Shortcut for a [`Box`]ed stream of items.
pub type Stream<I> = Pin<Box<dyn futures::Stream<Item = I> + Send>>;

/// Returns a [`Stream`] out of the given single value.
pub fn stream<T: Send + 'static>(val: T) -> Stream<T> {
    Box::pin(stream::once(future::ready(val)))
}

/// Extracts a single next value from the result returned by
/// [`juniper::resolve_into_stream()`] and transforms it into a regular
/// [`Value`].
pub async fn extract_next<'a, S: ScalarValue>(
    input: Result<(Value<ValuesStream<'a, S>>, Vec<ExecutionError<S>>), GraphQLError<'a>>,
) -> Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError<'a>> {
    let (stream, errs) = input?;
    if !errs.is_empty() {
        return Ok((Value::Null, errs));
    }

    if let Value::Object(obj) = stream {
        for (name, mut val) in obj {
            if let Value::Scalar(ref mut stream) = val {
                return match stream.next().await {
                    Some(Ok(val)) => Ok((graphql_value!({ name: val }), vec![])),
                    Some(Err(e)) => Ok((Value::Null, vec![e])),
                    None => Ok((Value::Null, vec![])),
                };
            }
        }
    }

    panic!("expected to get `Value::Object` containing a `Stream`")
}
