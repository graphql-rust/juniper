//! Common functions, definitions and extensions for parsing and code generation
//! of [GraphQL deprecation directive][0].
//!
//! [0]: https://spec.graphql.org/October2021#sec--deprecated

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
    token,
};

use crate::common::{parse::ParseBufferExt as _, SpanContainer};

/// [GraphQL deprecation directive][0] defined on a [GraphQL field][1] or a
/// [GraphQL enum value][2] via `#[graphql(deprecated = ...)]` (or
/// `#[deprecated(note = ...)]`) attribute.
///
/// [0]: https://spec.graphql.org/October2021#sec--deprecated
/// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
/// [2]: https://spec.graphql.org/October2021#sec-Enum-Value
#[derive(Debug, Default)]
pub(crate) struct Directive {
    /// Optional [reason][1] attached to this [deprecation][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec--deprecated
    /// [1]: https://spec.graphql.org/October2021#sel-GAHnBZDACEDDGAA_6L
    pub(crate) reason: Option<syn::LitStr>,
}

impl Parse for Directive {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            reason: input
                .try_parse::<token::Eq>()?
                .map(|_| input.parse::<syn::LitStr>())
                .transpose()?,
        })
    }
}

impl Directive {
    /// Tries to parse a [`Directive`] from a `#[deprecated(note = ...)]`
    /// attribute, by looking up for it in the provided [`syn::Attribute`]s.
    ///
    /// # Errors
    ///
    /// If failed to parse a [`Directive`] from a found
    /// `#[deprecated(note = ...)]` attribute.
    pub(crate) fn parse_from_deprecated_attr(
        attrs: &[syn::Attribute],
    ) -> syn::Result<Option<SpanContainer<Self>>> {
        for attr in attrs {
            return Ok(match attr.parse_meta() {
                Ok(syn::Meta::List(ref list)) if list.path.is_ident("deprecated") => {
                    let directive = Self::parse_from_deprecated_meta_list(list)?;
                    Some(SpanContainer::new(
                        list.path.span(),
                        directive.reason.as_ref().map(|r| r.span()),
                        directive,
                    ))
                }
                Ok(syn::Meta::Path(ref path)) if path.is_ident("deprecated") => {
                    Some(SpanContainer::new(path.span(), None, Self::default()))
                }
                _ => continue,
            });
        }
        Ok(None)
    }

    /// Tries to parse a [`Directive`] from the [`syn::MetaList`] of a single
    /// `#[deprecated(note = ...)]` attribute.
    ///
    /// # Errors
    ///
    /// If the `#[deprecated(note = ...)]` attribute has incorrect format.
    fn parse_from_deprecated_meta_list(list: &syn::MetaList) -> syn::Result<Self> {
        for meta in &list.nested {
            if let syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) = meta {
                return if !nv.path.is_ident("note") {
                    Err(syn::Error::new(
                        nv.path.span(),
                        "unrecognized setting on #[deprecated(..)] attribute",
                    ))
                } else if let syn::Lit::Str(strlit) = &nv.lit {
                    Ok(Self {
                        reason: Some(strlit.clone()),
                    })
                } else {
                    Err(syn::Error::new(
                        nv.lit.span(),
                        "only strings are allowed for deprecation",
                    ))
                };
            }
        }
        Ok(Self::default())
    }
}

impl ToTokens for Directive {
    fn to_tokens(&self, into: &mut TokenStream) {
        let reason = self
            .reason
            .as_ref()
            .map_or_else(|| quote! { None }, |text| quote! { Some(#text) });
        quote! {
            .deprecated(::core::option::Option::#reason)
        }
        .to_tokens(into);
    }
}
