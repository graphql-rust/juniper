//! Common functions, definitions and extensions for parsing and code generation
//! related to [`Behaviour`] type parameter.
//!
//! [`Behaviour`]: juniper::behavior

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
};

use crate::common::SpanContainer;

/// [`Behaviour`] parametrization of the code generation.
///
/// [`Behaviour`]: juniper::behavior
#[derive(Clone, Debug, Default)]
pub(crate) enum Type {
    /// [`behavior::Standard`] should be used in the generated code.
    ///
    /// [`behavior::Standard`]: juniper::behavior::Standard
    #[default]
    Standard,

    /// Concrete custom Rust type should be used as [`Behaviour`] in the
    /// generated code.
    ///
    /// [`Behaviour`]: juniper::behavior
    Custom(syn::Type),
}

impl Parse for Type {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        input.parse::<syn::Type>().map(Self::Custom)
    }
}

impl ToTokens for Type {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.ty().to_tokens(into)
    }
}

impl Type {
    /// Returns a Rust type representing this [`Type`].
    #[must_use]
    pub(crate) fn ty(&self) -> syn::Type {
        match self {
            Self::Standard => parse_quote! { ::juniper::behavior::Standard },
            Self::Custom(ty) => ty.clone(),
        }
    }
}

impl From<Option<SpanContainer<Self>>> for Type {
    fn from(attr: Option<SpanContainer<Self>>) -> Self {
        attr.map(SpanContainer::into_inner).unwrap_or_default()
    }
}
