//! GraphQL implementation for [`str`].
//!
//! [`str`]: primitive@std::str

use std::{borrow::Cow, rc::Rc, sync::Arc};

use futures::future;

use crate::{
    graphql,
    meta::MetaType,
    parser::{ParseError, ScalarToken},
    reflect, resolve, BoxFuture, ExecutionResult, Executor, Registry, ScalarValue, Selection,
};

impl<TI: ?Sized, SV: ScalarValue> resolve::Type<TI, SV> for str {
    fn meta<'r, 'ti: 'r>(registry: &mut Registry<'r, SV>, type_info: &'ti TI) -> MetaType<'r, SV>
    where
        SV: 'r,
    {
        registry.register_scalar_unsized::<Self, _>(type_info)
    }
}

impl<TI: ?Sized> resolve::TypeName<TI> for str {
    fn type_name(_: &TI) -> &'static str {
        <Self as reflect::BaseType>::NAME
    }
}

impl<TI, CX, SV> resolve::Value<TI, CX, SV> for str
where
    TI: ?Sized,
    CX: ?Sized,
    SV: From<String>,
{
    fn resolve_value(
        &self,
        _: Option<&[Selection<'_, SV>]>,
        _: &TI,
        _: &Executor<CX, SV>,
    ) -> ExecutionResult<SV> {
        // TODO: Remove redundant `.to_owned()` allocation by allowing
        //       `ScalarValue` creation from reference?
        Ok(graphql::Value::scalar(self.to_owned()))
    }
}

impl<TI, CX, SV> resolve::ValueAsync<TI, CX, SV> for str
where
    TI: ?Sized,
    CX: ?Sized,
    SV: From<String> + Send,
{
    fn resolve_value_async<'r>(
        &'r self,
        _: Option<&'r [Selection<'_, SV>]>,
        _: &'r TI,
        _: &'r Executor<CX, SV>,
    ) -> BoxFuture<'r, ExecutionResult<SV>> {
        // TODO: Remove redundant `.to_owned()` allocation by allowing
        //       `ScalarValue` creation from reference?
        Box::pin(future::ok(graphql::Value::scalar(self.to_owned())))
    }
}

impl<SV> resolve::ToInputValue<SV> for str
where
    SV: From<String>,
{
    fn to_input_value(&self) -> graphql::InputValue<SV> {
        // TODO: Remove redundant `.to_owned()` allocation by allowing
        //       `ScalarValue` creation from reference?
        graphql::InputValue::scalar(self.to_owned())
    }
}

impl<SV: ScalarValue> resolve::InputValueAsRef<SV> for str {
    type Error = String;

    fn try_from_input_value(v: &graphql::InputValue<SV>) -> Result<&Self, Self::Error> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
    }
}

impl<'me, 'i, SV> resolve::InputValueAs<'i, Cow<'me, Self>, SV> for str
where
    'i: 'me,
    SV: 'i,
    Self: resolve::InputValueAsRef<SV>,
{
    type Error = <Self as resolve::InputValueAsRef<SV>>::Error;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Cow<'me, Self>, Self::Error> {
        <Self as resolve::InputValueAsRef<SV>>::try_from_input_value(v).map(Cow::Borrowed)
    }
}

impl<'i, SV> resolve::InputValueAs<'i, Box<Self>, SV> for str
where
    SV: 'i,
    Self: resolve::InputValueAsRef<SV>,
{
    type Error = <Self as resolve::InputValueAsRef<SV>>::Error;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Box<Self>, Self::Error> {
        <Self as resolve::InputValueAsRef<SV>>::try_from_input_value(v).map(Into::into)
    }
}

impl<'i, SV> resolve::InputValueAs<'i, Rc<Self>, SV> for str
where
    SV: 'i,
    Self: resolve::InputValueAsRef<SV>,
{
    type Error = <Self as resolve::InputValueAsRef<SV>>::Error;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Rc<Self>, Self::Error> {
        <Self as resolve::InputValueAsRef<SV>>::try_from_input_value(v).map(Into::into)
    }
}

impl<'i, SV> resolve::InputValueAs<'i, Arc<Self>, SV> for str
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
where
    String: resolve::ScalarToken<SV>,
{
    fn parse_scalar_token(token: ScalarToken<'_>) -> Result<SV, ParseError> {
        <String as resolve::ScalarToken<SV>>::parse_scalar_token(token)
    }
}

impl<'me, 'i, TI, SV> graphql::InputTypeAs<'i, &'me Self, TI, SV> for str
where
    Self: graphql::Type<TI, SV>
        + resolve::ToInputValue<SV>
        + resolve::InputValueAs<'i, &'me Self, SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_input_type() {}
}

impl<'me, 'i, TI, SV> graphql::InputTypeAs<'i, Cow<'me, Self>, TI, SV> for str
where
    Self: graphql::Type<TI, SV>
        + resolve::ToInputValue<SV>
        + resolve::InputValueAs<'i, Cow<'me, Self>, SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_input_type() {}
}

impl<'i, TI, SV> graphql::InputTypeAs<'i, Box<Self>, TI, SV> for str
where
    Self: graphql::Type<TI, SV>
        + resolve::ToInputValue<SV>
        + resolve::InputValueAs<'i, Box<Self>, SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_input_type() {}
}

impl<'i, TI, SV> graphql::InputTypeAs<'i, Rc<Self>, TI, SV> for str
where
    Self:
        graphql::Type<TI, SV> + resolve::ToInputValue<SV> + resolve::InputValueAs<'i, Rc<Self>, SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_input_type() {}
}

impl<'i, TI, SV> graphql::InputTypeAs<'i, Arc<Self>, TI, SV> for str
where
    Self: graphql::Type<TI, SV>
        + resolve::ToInputValue<SV>
        + resolve::InputValueAs<'i, Arc<Self>, SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_input_type() {}
}

impl<TI, CX, SV> graphql::OutputType<TI, CX, SV> for str
where
    Self: graphql::Type<TI, SV> + resolve::Value<TI, CX, SV> + resolve::ValueAsync<TI, CX, SV>,
    TI: ?Sized,
    CX: ?Sized,
{
    fn assert_output_type() {}
}

impl<'me, 'i, TI, CX, SV> graphql::ScalarAs<'i, &'me Self, TI, CX, SV> for str
where
    Self: graphql::InputTypeAs<'i, &'me Self, TI, SV>
        + graphql::OutputType<TI, CX, SV>
        + resolve::ScalarToken<SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_scalar() {}
}

impl<'me, 'i, TI, CX, SV> graphql::ScalarAs<'i, Cow<'me, Self>, TI, CX, SV> for str
where
    Self: graphql::InputTypeAs<'i, Cow<'me, Self>, TI, SV>
        + graphql::OutputType<TI, CX, SV>
        + resolve::ScalarToken<SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_scalar() {}
}

impl<'i, TI, CX, SV> graphql::ScalarAs<'i, Box<Self>, TI, CX, SV> for str
where
    Self: graphql::InputTypeAs<'i, Box<Self>, TI, SV>
        + graphql::OutputType<TI, CX, SV>
        + resolve::ScalarToken<SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_scalar() {}
}

impl<'i, TI, CX, SV> graphql::ScalarAs<'i, Rc<Self>, TI, CX, SV> for str
where
    Self: graphql::InputTypeAs<'i, Rc<Self>, TI, SV>
        + graphql::OutputType<TI, CX, SV>
        + resolve::ScalarToken<SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_scalar() {}
}

impl<'i, TI, CX, SV> graphql::ScalarAs<'i, Arc<Self>, TI, CX, SV> for str
where
    Self: graphql::InputTypeAs<'i, Arc<Self>, TI, SV>
        + graphql::OutputType<TI, CX, SV>
        + resolve::ScalarToken<SV>,
    TI: ?Sized,
    SV: 'i,
{
    fn assert_scalar() {}
}

impl reflect::BaseType for str {
    const NAME: reflect::Type = <String as reflect::BaseType>::NAME;
}

impl reflect::BaseSubTypes for str {
    const NAMES: reflect::Types = &[<Self as reflect::BaseType>::NAME];
}

impl reflect::WrappedType for str {
    const VALUE: reflect::WrappedValue = reflect::wrap::SINGULAR;
}
