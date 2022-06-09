//! Default GraphQL behaviors.

use std::{marker::PhantomData, sync::atomic::AtomicPtr};

/// Default standard behavior of GraphQL types implementation.
#[derive(Debug)]
pub enum Standard {}

pub struct Coerce<T: ?Sized, From: ?Sized = Standard>(PhantomData<AtomicPtr<Box<From>>>, T);

impl<T, From: ?Sized> Coerce<T, From> {
    #[must_use]
    pub const fn wrap(val: T) -> Self {
        Self(PhantomData, val)
    }
}

#[must_use]
pub const fn coerce<T, From: ?Sized>(val: T) -> Coerce<T, From> {
    Coerce::wrap(val)
}
