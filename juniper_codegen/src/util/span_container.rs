use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::cmp::{Eq, PartialEq};

#[derive(Debug, Clone)]
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

    pub fn span_expr(&self) -> Option<Span> {
        self.expr
    }

    pub fn span_ident(&self) -> Span {
        self.ident
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

impl<T> std::ops::Deref for SpanContainer<T> {
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
