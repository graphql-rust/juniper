//! Common functions, definitions and extensions for parsing and code generation
//! of [GraphQL description][0].
//!
//! [0]: https://spec.graphql.org/October2021#sec-Descriptions

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
};

use crate::common::SpanContainer;

/// [GraphQL description][0] defined on a GraphQL definition via
/// `#[graphql(description = ...)]` (or `#[doc = ...]`) attribute.
///
/// [0]: https://spec.graphql.org/October2021#sec-Descriptions
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Description(syn::LitStr);

impl Parse for Description {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        input.parse::<syn::LitStr>().map(Self)
    }
}

impl Description {
    /// Tries to parse a [`Description`] from a `#[doc = ...]` attribute (or
    /// Rust doc comment), by looking up for it in the provided
    /// [`syn::Attribute`]s.
    ///
    /// # Errors
    ///
    /// If failed to parse a [`Description`] from a found `#[doc = ...]`
    /// attribute.
    pub(crate) fn parse_from_doc_attrs(
        attrs: &[syn::Attribute],
    ) -> syn::Result<Option<SpanContainer<Self>>> {
        let (mut first_span, mut descriptions) = (None, Vec::new());
        for attr in attrs {
            match attr.meta {
                syn::Meta::NameValue(ref nv) if nv.path.is_ident("doc") => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(strlit),
                        ..
                    }) = &nv.value
                    {
                        if first_span.is_none() {
                            first_span = Some(strlit.span());
                        }
                        descriptions.push(strlit.value());
                    } else {
                        return Err(syn::Error::new(
                            nv.value.span(),
                            "#[doc] attributes may only have a string literal",
                        ));
                    }
                }
                _ => continue,
            }
        }
        Ok(first_span.map(|span| {
            SpanContainer::new(
                span,
                None,
                Self(syn::LitStr::new(&Self::concatenate(&descriptions), span)),
            )
        }))
    }

    /// Concatenates [`Description`] strings into a single one.
    fn concatenate(descriptions: &[String]) -> String {
        let last_index = descriptions.len() - 1;
        descriptions
            .iter()
            .map(|s| s.as_str().trim_end())
            .map(|s| {
                // Trim leading space.
                s.strip_prefix(' ').unwrap_or(s)
            })
            .enumerate()
            .fold(String::new(), |mut buffer, (index, s)| {
                // Add newline, except when string ends in a continuation
                // backslash or is the last line.
                if index == last_index {
                    buffer.push_str(s);
                } else if s.ends_with('\\') {
                    buffer.push_str(s.trim_end_matches('\\'));
                    buffer.push(' ');
                } else {
                    buffer.push_str(s);
                    buffer.push('\n');
                }
                buffer
            })
    }
}

impl ToTokens for Description {
    fn to_tokens(&self, into: &mut TokenStream) {
        let desc = &self.0;

        quote! {
            .description(#desc)
        }
        .to_tokens(into);
    }
}

#[cfg(test)]
mod parse_from_doc_attrs_test {
    use quote::quote;
    use syn::parse_quote;

    use super::Description;

    #[test]
    fn single() {
        let desc = Description::parse_from_doc_attrs(&[parse_quote! { #[doc = "foo"] }])
            .unwrap()
            .unwrap()
            .into_inner();
        assert_eq!(
            quote! { #desc }.to_string(),
            quote! { .description("foo") }.to_string(),
        );
    }

    #[test]
    fn many() {
        let desc = Description::parse_from_doc_attrs(&[
            parse_quote! { #[doc = "foo"] },
            parse_quote! { #[doc = "\n"] },
            parse_quote! { #[doc = "bar"] },
        ])
        .unwrap()
        .unwrap()
        .into_inner();
        assert_eq!(
            quote! { #desc }.to_string(),
            quote! { .description("foo\n\nbar") }.to_string(),
        );
    }

    #[test]
    fn not_doc() {
        let desc = Description::parse_from_doc_attrs(&[parse_quote! { #[blah = "foo"] }]).unwrap();
        assert_eq!(desc, None);
    }
}

#[cfg(test)]
mod concatenate_test {
    use super::Description;

    /// Forms a [`Vec`] of [`String`]s out of the provided [`str`]s
    /// [`Iterator`].
    fn to_strings<'i>(source: impl IntoIterator<Item = &'i str>) -> Vec<String> {
        source.into_iter().map(Into::into).collect()
    }

    #[test]
    fn single() {
        assert_eq!(Description::concatenate(&to_strings(["foo"])), "foo");
    }

    #[test]
    fn multiple() {
        assert_eq!(
            Description::concatenate(&to_strings(["foo", "bar"])),
            "foo\nbar",
        );
    }

    #[test]
    fn trims_spaces() {
        assert_eq!(
            Description::concatenate(&to_strings([" foo ", "bar ", " baz"])),
            "foo\nbar\nbaz",
        );
    }

    #[test]
    fn empty() {
        assert_eq!(
            Description::concatenate(&to_strings(["foo", "", "bar"])),
            "foo\n\nbar",
        );
    }

    #[test]
    fn newline_spaces() {
        assert_eq!(
            Description::concatenate(&to_strings(["foo ", "", " bar"])),
            "foo\n\nbar",
        );
    }

    #[test]
    fn continuation_backslash() {
        assert_eq!(
            Description::concatenate(&to_strings(["foo\\", "x\\", "y", "bar"])),
            "foo x y\nbar",
        );
    }
}
