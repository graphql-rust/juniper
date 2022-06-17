//! Default GraphQL behaviors.

use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use crate::{meta::MetaType, resolve, Registry};

/// Default standard behavior of GraphQL types implementation.
#[derive(Debug)]
pub enum Standard {}

/// Coercion of behavior types and type parameters.
pub struct Coerce<T: ?Sized, From: ?Sized = Standard>(PhantomData<AtomicPtr<Box<From>>>, T);

impl<T, From: ?Sized> Coerce<T, From> {
    #[must_use]
    pub const fn wrap(value: T) -> Self {
        Self(PhantomData, value)
    }
}

#[must_use]
pub const fn coerce<T, From: ?Sized>(value: T) -> Coerce<T, From> {
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
