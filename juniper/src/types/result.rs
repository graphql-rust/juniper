//! GraphQL implementation for [`Result`].

use crate::reflect;

impl<T, E, BH> reflect::BaseType<BH> for Result<T, E>
where
    T: reflect::BaseType<BH>,
    BH: ?Sized,
{
    const NAME: reflect::Type = T::NAME;
}

impl<T, E, BH> reflect::BaseSubTypes<BH> for Result<T, E>
where
    T: reflect::BaseSubTypes<BH>,
    BH: ?Sized,
{
    const NAMES: reflect::Types = T::NAMES;
}

impl<T, E, BH> reflect::WrappedType<BH> for Result<T, E>
where
    T: reflect::WrappedType<BH>,
    BH: ?Sized,
{
    const VALUE: reflect::WrappedValue = T::VALUE;
}
