use proc_macro2::Span;
use syn::spanned::Spanned;

/// Creates "duplicated argument" [`syn::Error`] for the given `name` pointing to the given
/// [`Span`].
#[must_use]
pub fn dup_arg<S: AsSpan>(span: S) -> syn::Error {
    syn::Error::new(span.as_span(), "duplicated attribute argument found")
}

/// Creates "unknown argument" [`syn::Error`] for the given `name` pointing to the given [`Span`].
#[must_use]
pub fn unknown_arg<S: AsSpan>(span: S, name: &str) -> syn::Error {
    syn::Error::new(
        span.as_span(),
        format!("unknown `{}` attribute argument", name),
    )
}

pub trait AsSpan {
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