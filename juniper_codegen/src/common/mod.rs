//! Common functions, definitions and extensions for code generation, used by this crate.

pub(crate) mod default;
pub(crate) mod deprecation;
mod description;
pub(crate) mod diagnostic;
pub(crate) mod field;
pub(crate) mod gen;
pub(crate) mod parse;
pub(crate) mod rename;
pub(crate) mod scalar;
mod span_container;

use std::slice;

pub(crate) use self::{description::Description, span_container::SpanContainer};

/// Checks whether the specified [`syn::Path`] equals to one of specified one-segment
/// [`AttrNames::values`].
pub(crate) fn path_eq_single(path: &syn::Path, names: impl AttrNames) -> bool {
    path.segments.len() == 1
        && names
            .values()
            .iter()
            .any(|name| path.segments[0].ident == name)
}

/// Filters the provided [`syn::Attribute`] to contain only ones with the
/// specified `name`.
pub(crate) fn filter_attrs<'a>(
    names: impl AttrNames + 'a,
    attrs: &'a [syn::Attribute],
) -> impl Iterator<Item = &'a syn::Attribute> + 'a {
    attrs
        .iter()
        .filter(move |attr| path_eq_single(attr.path(), names))
}

/// Input-type polymorphism helper for checking names of multiple attribute names.
pub(crate) trait AttrNames: Copy {
    /// Returns values to be checked.
    fn values(&self) -> &[&str];
}

impl AttrNames for &str {
    fn values(&self) -> &[&str] {
        slice::from_ref(self)
    }
}

impl AttrNames for &[&str] {
    fn values(&self) -> &[&str] {
        self
    }
}

impl<const N: usize> AttrNames for [&str; N] {
    fn values(&self) -> &[&str] {
        self
    }
}
