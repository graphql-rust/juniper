//! GraphQL types behavior machinery.

use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use crate::{
    graphql,
    meta::MetaType,
    parser::{ParseError, ScalarToken},
    reflect, resolve, Registry,
};

/// Default standard behavior of GraphQL types implementation.
#[derive(Debug)]
pub enum Standard {}

/// Transparent wrapper allowing coercion of behavior types and type parameters.
#[repr(transparent)]
pub struct Coerce<T: ?Sized, To: ?Sized = Standard>(PhantomData<AtomicPtr<Box<To>>>, T);

impl<T, To: ?Sized> Coerce<T, To> {
    /// Wraps the provided `value` into a [`Coerce`] wrapper.
    #[must_use]
    pub const fn wrap(value: T) -> Self {
        Self(PhantomData, value)
    }

    /// Unwraps into the inner value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.1
    }
}

/// Wraps the provided `value` into a [`Coerce`] wrapper.
#[must_use]
pub const fn coerce<T, To: ?Sized>(value: T) -> Coerce<T, To> {
    Coerce::wrap(value)
}

impl<T, TI, SV, B1, B2> resolve::Type<TI, SV, B1> for Coerce<T, B2>
where
    T: resolve::Type<TI, SV, B2> + ?Sized,
    TI: ?Sized,
    B1: ?Sized,
    B2: ?Sized,
{
    fn meta<'r, 'ti: 'r>(registry: &mut Registry<'r, SV>, type_info: &'ti TI) -> MetaType<'r, SV>
    where
        SV: 'r,
    {
        T::meta(registry, type_info)
    }
}

impl<T, TI, B1, B2> resolve::TypeName<TI, B1> for Coerce<T, B2>
where
    T: resolve::TypeName<TI, B2> + ?Sized,
    TI: ?Sized,
    B1: ?Sized,
    B2: ?Sized,
{
    fn type_name(type_info: &TI) -> &str {
        T::type_name(type_info)
    }
}

impl<'i, T, SV, B1, B2> resolve::InputValue<'i, SV, B1> for Coerce<T, B2>
where
    T: resolve::InputValue<'i, SV, B2>,
    SV: 'i,
    B1: ?Sized,
    B2: ?Sized,
{
    type Error = T::Error;

    fn try_from_input_value(v: &'i graphql::InputValue<SV>) -> Result<Self, Self::Error> {
        T::try_from_input_value(v).map(Self::wrap)
    }

    fn try_from_implicit_null() -> Result<Self, Self::Error> {
        T::try_from_implicit_null().map(Self::wrap)
    }
}

impl<T, SV, B1, B2> resolve::ScalarToken<SV, B1> for Coerce<T, B2>
where
    T: resolve::ScalarToken<SV, B2> + ?Sized,
    B1: ?Sized,
    B2: ?Sized,
{
    fn parse_scalar_token(token: ScalarToken<'_>) -> Result<SV, ParseError> {
        T::parse_scalar_token(token)
    }
}

impl<T, B1, B2> reflect::BaseType<B1> for Coerce<T, B2>
where
    T: reflect::BaseType<B2> + ?Sized,
    B1: ?Sized,
    B2: ?Sized,
{
    const NAME: reflect::Type = T::NAME;
}

impl<T, B1, B2> reflect::BaseSubTypes<B1> for Coerce<T, B2>
where
    T: reflect::BaseSubTypes<B2> + ?Sized,
    B1: ?Sized,
    B2: ?Sized,
{
    const NAMES: reflect::Types = T::NAMES;
}

impl<T, B1, B2> reflect::WrappedType<B1> for Coerce<T, B2>
where
    T: reflect::WrappedType<B2> + ?Sized,
    B1: ?Sized,
    B2: ?Sized,
{
    const VALUE: reflect::WrappedValue = T::VALUE;
}

impl<T, B1, B2> reflect::Implements<B1> for Coerce<T, B2>
where
    T: reflect::Implements<B2> + ?Sized,
    B1: ?Sized,
    B2: ?Sized,
{
    const NAMES: reflect::Types = T::NAMES;
}
