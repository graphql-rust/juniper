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
            format!("unknown `{}` attribute argument", name),
        )
    }

    pub(crate) trait AsSpan {
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

/// Handy extension of [`Option`] methods used in this crate.
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
