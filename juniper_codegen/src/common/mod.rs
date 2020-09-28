//! Common functions, definitions and extensions for code generation, used by this crate.

pub(crate) mod gen;
pub(crate) mod parse;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse_quote;

///
#[derive(Clone, Debug)]
pub(crate) enum ScalarValueType {
    Concrete(syn::Type),
    ExplicitGeneric(syn::Ident),
    ImplicitGeneric,
}

impl ScalarValueType {
    #[must_use]
    pub(crate) fn is_generic(&self) -> bool {
        matches!(self, Self::ExplicitGeneric(_) | Self::ImplicitGeneric)
    }

    #[must_use]
    pub(crate) fn is_explicit_generic(&self) -> bool {
        matches!(self, Self::ExplicitGeneric(_))
    }

    #[must_use]
    pub(crate) fn is_implicit_generic(&self) -> bool {
        matches!(self, Self::ImplicitGeneric)
    }

    #[must_use]
    pub(crate) fn ty(&self) -> syn::Type {
        match self {
            Self::Concrete(ty) => ty.clone(),
            Self::ExplicitGeneric(ty_param) => parse_quote! { #ty_param },
            Self::ImplicitGeneric => parse_quote! { __S },
        }
    }

    #[must_use]
    pub(crate) fn generic_ty(&self) -> syn::Type {
        match self {
            Self::ExplicitGeneric(ty_param) => parse_quote! { #ty_param },
            Self::ImplicitGeneric | Self::Concrete(_) => parse_quote! { __S },
        }
    }

    #[must_use]
    pub(crate) fn default_ty(&self) -> syn::Type {
        match self {
            Self::Concrete(ty) => ty.clone(),
            Self::ExplicitGeneric(_) | Self::ImplicitGeneric => {
                parse_quote! { ::juniper::DefaultScalarValue }
            }
        }
    }
}

impl ToTokens for ScalarValueType {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.ty().to_tokens(into)
    }
}
