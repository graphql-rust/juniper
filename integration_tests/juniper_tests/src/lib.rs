#[cfg(test)]
mod arc_fields;
#[cfg(test)]
mod array;
#[cfg(test)]
mod codegen;
#[cfg(test)]
mod custom_scalar;
#[cfg(test)]
mod explicit_null;
#[cfg(test)]
mod infallible_as_field_error;
#[cfg(test)]
mod issue_371;
#[cfg(test)]
mod issue_372;
#[cfg(test)]
mod issue_398;
#[cfg(test)]
mod issue_407;
#[cfg(test)]
mod issue_500;
#[cfg(test)]
mod issue_798;
#[cfg(test)]
mod issue_914;
#[cfg(test)]
mod issue_922;
#[cfg(test)]
mod issue_925;
#[cfg(test)]
mod issue_945;
#[cfg(test)]
mod pre_parse;

#[cfg(test)]
/// Common utilities used across tests.
pub(crate) mod util {
    use futures::StreamExt as _;
    use juniper::{
        graphql_value, DefaultScalarValue, EmptyMutation, EmptySubscription, ExecutionError,
        GraphQLError, GraphQLType, RootNode, ScalarValue, Value, ValuesStream,
    };

    pub(crate) fn schema<'q, C, Q>(
        query_root: Q,
    ) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>>
    where
        Q: GraphQLType<DefaultScalarValue, Context = C, TypeInfo = ()> + 'q,
    {
        RootNode::new(
            query_root,
            EmptyMutation::<C>::new(),
            EmptySubscription::<C>::new(),
        )
    }

    pub(crate) fn schema_with_scalar<'q, S, C, Q>(
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

    /// Extracts a single next value from the result returned by
    /// [`juniper::resolve_into_stream()`] and transforms it into a regular
    /// [`Value`].
    pub(crate) async fn extract_next<'a, S: ScalarValue>(
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

        panic!("Expected to get Value::Object containing a Stream")
    }
}
