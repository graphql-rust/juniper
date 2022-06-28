//! Common functions, definitions and extensions for parsing and code generation
//! of [GraphQL default values][0]
//!
//! [0]: https://spec.graphql.org/October2021#DefaultValue

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    token,
};

use crate::common::parse::ParseBufferExt as _;

/// Representation of a [GraphQL default value][0] for code generation.
///
/// [0]: https://spec.graphql.org/October2021#DefaultValue
#[derive(Clone, Debug)]
pub(crate) enum Value {
    /// [`Default`] implementation should be used.
    Default,

    /// Explicit [`Expr`]ession to be used as the [default value][0].
    ///
    /// [`Expr`]: syn::Expr
    /// [0]: https://spec.graphql.org/October2021#DefaultValue
    Expr(Box<syn::Expr>),
}

impl Default for Value {
    fn default() -> Self {
        Self::Default
    }
}

impl From<Option<syn::Expr>> for Value {
    fn from(opt: Option<syn::Expr>) -> Self {
        match opt {
            Some(expr) => Self::Expr(Box::new(expr)),
            None => Self::Default,
        }
    }
}

impl Parse for Value {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(input
            .try_parse::<token::Eq>()?
            .map(|_| input.parse::<syn::Expr>())
            .transpose()?
            .into())
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, into: &mut TokenStream) {
        match self {
            Self::Default => quote! {
                ::std::default::Default::default()
            }
            .to_tokens(into),
            Self::Expr(expr) => expr.to_tokens(into),
        }
    }
}
