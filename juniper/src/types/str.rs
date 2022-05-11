//! GraphQL implementation for [`str`].
//!
//! [`str`]: primitive@std::str

use std::convert::TryFrom;

use futures::future;

use crate::{
    graphql,
    meta::MetaType,
    parser::{ParseError, ScalarToken},
    resolve, BoxFuture, ExecutionResult, Executor, IntoFieldError, Registry, ScalarValue,
    Selection,
};

impl<Info: ?Sized, S: ScalarValue> resolve::Type<Info, S> for str {
    fn meta<'r>(registry: &mut Registry<'r, S>, info: &Info) -> MetaType<'r, S>
    where
        S: 'r,
    {
       // registry.build_scalar_type_new::<&Self, _>(info).into_meta()
        unimplemented!()
    }
}

impl<Info: ?Sized> resolve::TypeName<Info> for str {
    fn type_name(_: &Info) -> &'static str {
        // TODO: Reuse from `String`.
        "String"
    }
}

impl<Info, Ctx, S> resolve::Value<Info, Ctx, S> for str
where
    Info: ?Sized,
    Ctx: ?Sized,
    S: From<String>,
{
    fn resolve_value(
        &self,
        _: Option<&[Selection<'_, S>]>,
        _: &Info,
        _: &Executor<Ctx, S>,
    ) -> ExecutionResult<S> {
        // TODO: Remove redundant `.to_owned()` allocation by allowing
        //       `ScalarValue` creation from reference?
        Ok(graphql::Value::scalar(self.to_owned()))
    }
}

impl<Info, Ctx, S> resolve::ValueAsync<Info, Ctx, S> for str
where
    Info: ?Sized,
    Ctx: ?Sized,
    S: From<String> + Send,
{
    fn resolve_value_async<'r>(
        &'r self,
        _: Option<&'r [Selection<'_, S>]>,
        _: &'r Info,
        _: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>> {
        // TODO: Remove redundant `.to_owned()` allocation by allowing
        //       `ScalarValue` creation from reference?
        Box::pin(future::ok(graphql::Value::scalar(self.to_owned())))
    }
}

impl<'me, S: ScalarValue> resolve::ScalarToken<S> for str {
    fn parse_scalar_token(token: ScalarToken<'_>) -> Result<S, ParseError<'_>> {
        // TODO: Replace with `resolve::ScalarToken<S>`
        <String as crate::ParseScalarValue<S>>::from_str(token)
    }
}

impl<'me, S: ScalarValue> resolve::InputValueAsRef<S> for str {
    type Error = String;

    fn try_from_input_value(v: &graphql::InputValue<S>) -> Result<&Self, Self::Error> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
    }
}

impl<S> graphql::InputType<S> for str {
    fn assert_input_type() {}
}

impl<S> graphql::OutputType<S> for str {
    fn assert_output_type() {}
}

impl<S> graphql::Scalar<S> for str {
    fn assert_scalar() {}
}
