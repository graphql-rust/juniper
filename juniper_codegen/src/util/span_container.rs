use std::{
    hash::{Hash, Hasher},
    ops,
};

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;

#[derive(Clone, Debug)]
pub struct SpanContainer<T> {
    expr: Option<Span>,
    ident: Span,
    val: T,
}

impl<T: ToTokens> ToTokens for SpanContainer<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.val.to_tokens(tokens)
    }
}

impl<T> SpanContainer<T> {
    pub fn new(ident: Span, expr: Option<Span>, val: T) -> Self {
        Self { ident, expr, val }
    }

    pub fn span_ident(&self) -> Span {
        self.ident
    }

    pub fn span_joined(&self) -> Span {
        if let Some(s) = self.expr {
            // TODO: Use `Span::join` once stabilized and available on stable:
            //       https://github.com/rust-lang/rust/issues/54725
            // self.ident.join(s).unwrap()

            // At the moment, just return the second, more meaningful part.
            s
        } else {
            self.ident
        }
    }

    pub fn into_inner(self) -> T {
        self.val
    }

    pub fn inner(&self) -> &T {
        &self.val
    }

    pub fn map<U, F: Fn(T) -> U>(self, f: F) -> SpanContainer<U> {
        SpanContainer {
            expr: self.expr,
            ident: self.ident,
            val: f(self.val),
        }
    }
}

impl<T> AsRef<T> for SpanContainer<T> {
    fn as_ref(&self) -> &T {
        &self.val
    }
}

impl<T> ops::Deref for SpanContainer<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<T: PartialEq> PartialEq for SpanContainer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
    }
}

impl<T: Eq> Eq for SpanContainer<T> {}

impl<T: PartialEq> PartialEq<T> for SpanContainer<T> {
    fn eq(&self, other: &T) -> bool {
        &self.val == other
    }
}

impl<T: Hash> Hash for SpanContainer<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.val.hash(state)
    }
}
