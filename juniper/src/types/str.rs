//! GraphQL implementation for [`str`].
//!
//! [`str`]: primitive@std::str

use std::{rc::Rc, sync::Arc};

use futures::future;

use crate::{
    graphql,
    meta::MetaType,
    parser::{ParseError, ScalarToken},
    reflect, resolve, BoxFuture, ExecutionResult, Executor, Registry, ScalarValue, Selection,
};

impl<TI: ?Sized, SV: ScalarValue> resolve::Type<TI, SV> for str {
    fn meta<'r>(registry: &mut Registry<'r, SV>, type_info: &TI) -> MetaType<'r, SV>
    where
        SV: 'r,
    {
        let meta = registry
            .build_scalar_type_unsized::<Self, _>(type_info)
            .into_meta();
        registry
            .entry_type::<Self, _>(type_info)
            .or_insert(meta)
            .clone()
    }
}

impl<TI: ?Sized> resolve::TypeName<TI> for str {
    fn type_name(_: &TI) -> &'static str {
        <Self as reflect::BaseType>::NAME
    }
}

/*
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

impl<S> resolve::ToInputValue<S> for str
where
    S: From<String>,
{
    fn to_input_value(&self) -> graphql::InputValue<S> {
        graphql::InputValue::scalar(self.to_owned())
    }
}

 */

impl<SV: ScalarValue> resolve::InputValueAsRef<SV> for str {
    type Error = String;

    fn try_from_input_value(v: &graphql::InputValue<SV>) -> Result<&Self, Self::Error> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
    }
}

impl<'i, SV> resolve::InputValueAs<'i, Box<str>, SV> for str
where
    SV: 'i,
    Self: resolve::InputValueAsRef<SV>,
{
    type Error = <Self as resolve::InputValueAsRef<SV>>::Error;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Box<Self>, Self::Error> {
        <Self as resolve::InputValueAsRef<SV>>::try_from_input_value(v).map(Into::into)
    }
}

impl<'i, SV> resolve::InputValueAs<'i, Rc<str>, SV> for str
where
    SV: 'i,
    Self: resolve::InputValueAsRef<SV>,
{
    type Error = <Self as resolve::InputValueAsRef<SV>>::Error;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Rc<Self>, Self::Error> {
        <Self as resolve::InputValueAsRef<SV>>::try_from_input_value(v).map(Into::into)
    }
}

impl<'i, SV> resolve::InputValueAs<'i, Arc<str>, SV> for str
where
    SV: 'i,
    Self: resolve::InputValueAsRef<SV>,
{
    type Error = <Self as resolve::InputValueAsRef<SV>>::Error;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Arc<Self>, Self::Error> {
        <Self as resolve::InputValueAsRef<SV>>::try_from_input_value(v).map(Into::into)
    }
}

impl<SV> resolve::ScalarToken<SV> for str
//TODO: where String: resolve::ScalarToken<SV>,
where
    String: crate::ParseScalarValue<SV>,
{
    fn parse_scalar_token(token: ScalarToken<'_>) -> Result<SV, ParseError<'_>> {
        // TODO: <String as resolve::ScalarToken<SV>>::parse_scalar_token(token)
        <String as crate::ParseScalarValue<SV>>::from_str(token)
    }
}
/*

impl<'i, Info, S: 'i> graphql::InputType<'i, Info, S> for str
where
    Self: resolve::Type<Info, S> + resolve::ToInputValue<S> + resolve::InputValue<'i, S>,
    Info: ?Sized,
{
    fn assert_input_type() {}
}


impl<S> graphql::OutputType<S> for str {
    fn assert_output_type() {}
}

impl<S> graphql::Scalar<S> for str {
    fn assert_scalar() {}
}*/

impl reflect::BaseType for str {
    const NAME: reflect::Type = "String"; // TODO: <String as reflect::BaseType<BH>>::NAME;
}

impl reflect::BaseSubTypes for str {
    const NAMES: reflect::Types = &[<Self as reflect::BaseType>::NAME];
}

impl reflect::WrappedType for str {
    const VALUE: reflect::WrappedValue = reflect::wrap::SINGULAR;
}
