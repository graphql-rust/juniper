//! GraphQL implementation for [`Result`].

use crate::reflect;

impl<T, E, S> reflect::BaseType<S> for Result<T, E>
where
    T: reflect::BaseType<S>,
{
    const NAME: reflect::Type = T::NAME;
}

impl<T, E, S> reflect::BaseSubTypes<S> for Result<T, E>
where
    T: reflect::BaseSubTypes<S>,
{
    const NAMES: reflect::Types = T::NAMES;
}

impl<T, E, S> reflect::WrappedType<S> for Result<T, E>
where
    T: reflect::WrappedType<S>,
{
    const VALUE: reflect::WrappedValue = T::VALUE;
}
