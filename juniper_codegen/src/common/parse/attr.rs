//! Common functions, definitions and extensions for parsing and modifying Rust attributes, used by
//! this crate.

use proc_macro2::{Span, TokenStream};
use syn::parse_quote;

use crate::common::{path_eq_single, AttrNames};

/// Prepends the given `attrs` collection with a new [`syn::Attribute`] generated from the given
/// `attr_path` and `attr_args`.
///
/// This function is generally used for uniting `proc_macro_attribute` with its body attributes.
pub(crate) fn unite(
    (attr_path, attr_args): (&str, &TokenStream),
    attrs: &[syn::Attribute],
) -> Vec<syn::Attribute> {
    let mut full_attrs = Vec::with_capacity(attrs.len() + 1);
    let attr_path = syn::Ident::new(attr_path, Span::call_site());
    full_attrs.push(parse_quote! { #[#attr_path(#attr_args)] });
    full_attrs.extend_from_slice(attrs);
    full_attrs
}

/// Strips all `attr_path` attributes from the given `attrs` collection.
///
/// This function is generally used for removing duplicate attributes during `proc_macro_attribute`
/// expansion, so avoid unnecessary expansion duplication.
pub(crate) fn strip(names: impl AttrNames, attrs: Vec<syn::Attribute>) -> Vec<syn::Attribute> {
    attrs
        .into_iter()
        .filter(|attr| !path_eq_single(attr.path(), names))
        .collect()
}

/// Common errors of parsing Rust attributes, appeared in this crate.
pub(crate) mod err {
    use proc_macro2::Span;
    use syn::spanned::Spanned;

    /// Creates "duplicated argument" [`syn::Error`] for the given `name` pointing to the given
    /// `span`.
    #[must_use]
    pub(crate) fn dup_arg<S: AsSpan>(span: S) -> syn::Error {
        syn::Error::new(span.as_span(), "duplicated attribute argument found")
    }

    /// Creates "unknown argument" [`syn::Error`] for the given `name` pointing to the given `span`.
    #[must_use]
    pub(crate) fn unknown_arg<S: AsSpan>(span: S, name: &str) -> syn::Error {
        syn::Error::new(
            span.as_span(),
            format!("unknown `{name}` attribute argument"),
        )
    }

    /// Helper coercion for [`Span`] and [`Spanned`] types to use in function arguments.
    pub(crate) trait AsSpan {
        /// Returns the coerced [`Span`].
        #[must_use]
        fn as_span(&self) -> Span;
    }

    impl AsSpan for Span {
        #[inline]
        fn as_span(&self) -> Self {
            *self
        }
    }

    impl<T: Spanned> AsSpan for &T {
        #[inline]
        fn as_span(&self) -> Span {
            self.span()
        }
    }
}

/// Handy extension of [`Option`] methods, used in this crate.
pub(crate) trait OptionExt {
    type Inner;

    /// Transforms the `Option<T>` into a `Result<(), E>`, mapping `None` to `Ok(())` and `Some(v)`
    /// to `Err(err(v))`.
    fn none_or_else<E, F>(self, err: F) -> Result<(), E>
    where
        F: FnOnce(Self::Inner) -> E;
}

impl<T> OptionExt for Option<T> {
    type Inner = T;

    fn none_or_else<E, F>(self, err: F) -> Result<(), E>
    where
        F: FnOnce(T) -> E,
    {
        match self {
            Some(v) => Err(err(v)),
            None => Ok(()),
        }
    }
}
