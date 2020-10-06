//! Common functions, definitions and extensions for code generation, used by this crate.

pub(crate) mod gen;
pub(crate) mod parse;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse_quote;

/// [`ScalarValue`] parametrization of the code generation.
///
/// [`ScalarValue`]: juniper::ScalarValue
#[derive(Clone, Debug)]
pub(crate) enum ScalarValueType {
    /// Concrete Rust type is specified as [`ScalarValue`].
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    Concrete(syn::Type),

    /// One of type parameters of the original type is specified as [`ScalarValue`].
    ///
    /// The original type is the type that the code is generated for.
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    ExplicitGeneric(syn::Ident),

    /// [`ScalarValue`] parametrization is assumed to be a generic and is not specified explicitly.
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    ImplicitGeneric,
}

impl ScalarValueType {
    /// Indicates whether this [`ScalarValueType`] is generic.
    #[must_use]
    pub(crate) fn is_generic(&self) -> bool {
        matches!(self, Self::ExplicitGeneric(_) | Self::ImplicitGeneric)
    }

    /// Indicates whether this [`ScalarValueType`] is [`ScalarValueType::ExplicitGeneric`].
    #[must_use]
    pub(crate) fn is_explicit_generic(&self) -> bool {
        matches!(self, Self::ExplicitGeneric(_))
    }

    /// Indicates whether this [`ScalarValueType`] is [`ScalarValueType::ImplicitGeneric`].
    #[must_use]
    pub(crate) fn is_implicit_generic(&self) -> bool {
        matches!(self, Self::ImplicitGeneric)
    }

    /// Returns a type identifier which represents this [`ScalarValueType`].
    #[must_use]
    pub(crate) fn ty(&self) -> syn::Type {
        match self {
            Self::Concrete(ty) => ty.clone(),
            Self::ExplicitGeneric(ty_param) => parse_quote! { #ty_param },
            Self::ImplicitGeneric => parse_quote! { __S },
        }
    }

    /// Returns a type parameter identifier that suits this [`ScalarValueType`].
    #[must_use]
    pub(crate) fn generic_ty(&self) -> syn::Type {
        match self {
            Self::ExplicitGeneric(ty_param) => parse_quote! { #ty_param },
            Self::ImplicitGeneric | Self::Concrete(_) => parse_quote! { __S },
        }
    }

    /// Returns a default [`ScalarValue`] type that is compatible with this [`ScalarValueType`].
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
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
