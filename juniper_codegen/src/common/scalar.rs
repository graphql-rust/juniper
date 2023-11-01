//! Common functions, definitions and extensions for parsing and code generation
//! related to [`ScalarValue`].
//!
//! [`ScalarValue`]: juniper::ScalarValue

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};

/// Possible values of `#[graphql(scalar = ...)]` attribute.
#[derive(Clone, Debug)]
pub(crate) enum AttrValue {
    /// Concrete Rust type (like `DefaultScalarValue`).
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    Concrete(syn::Type),

    /// Generic Rust type parameter with a bound predicate
    /// (like `S: ScalarValue + Send + Sync`).
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    Generic(syn::PredicateType),
}

impl Parse for AttrValue {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.fork().parse::<syn::WherePredicate>().is_ok() {
            let pred = input.parse().unwrap();
            if let syn::WherePredicate::Type(p) = pred {
                Ok(Self::Generic(p))
            } else {
                Err(syn::Error::new(
                    pred.span(),
                    "only type predicates are allowed here",
                ))
            }
        } else {
            input.parse::<syn::Type>().map(Self::Concrete)
        }
    }
}

impl ToTokens for AttrValue {
    fn to_tokens(&self, into: &mut TokenStream) {
        match self {
            Self::Concrete(ty) => ty.to_tokens(into),
            Self::Generic(pred) => pred.to_tokens(into),
        }
    }
}

/// [`ScalarValue`] parametrization of the code generation.
///
/// [`ScalarValue`]: juniper::ScalarValue
#[derive(Clone, Debug)]
pub(crate) enum Type {
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

    /// [`ScalarValue`] parametrization is assumed to be generic and is not specified
    /// explicitly, or specified as bound predicate (like `S: ScalarValue + Send + Sync`).
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    ImplicitGeneric(Option<syn::PredicateType>),
}

impl ToTokens for Type {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.ty().to_tokens(into)
    }
}

impl Type {
    /// Indicates whether this [`Type`] is generic.
    #[must_use]
    pub(crate) fn is_generic(&self) -> bool {
        matches!(self, Self::ExplicitGeneric(_) | Self::ImplicitGeneric(_))
    }

    /// Indicates whether this [`Type`] is [`Type::ImplicitGeneric`].
    #[must_use]
    pub(crate) fn is_implicit_generic(&self) -> bool {
        matches!(self, Self::ImplicitGeneric(_))
    }

    /// Returns additional trait bounds behind this [`Type`], if any.
    #[must_use]
    pub(crate) fn bounds(&self) -> Option<syn::WherePredicate> {
        if let Self::ImplicitGeneric(Some(pred)) = self {
            Some(syn::WherePredicate::Type(pred.clone()))
        } else {
            None
        }
    }

    /// Returns a type identifier which represents this [`Type`].
    #[must_use]
    pub(crate) fn ty(&self) -> syn::Type {
        match self {
            Self::Concrete(ty) => ty.clone(),
            Self::ExplicitGeneric(ty_param) => parse_quote! { #ty_param },
            Self::ImplicitGeneric(Some(pred)) => pred.bounded_ty.clone(),
            Self::ImplicitGeneric(None) => parse_quote! { __S },
        }
    }

    /// Returns a default [`ScalarValue`] type that is compatible with this [`Type`].
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    #[must_use]
    pub(crate) fn default_ty(&self) -> syn::Type {
        match self {
            Self::Concrete(ty) => ty.clone(),
            Self::ExplicitGeneric(_) | Self::ImplicitGeneric(_) => {
                parse_quote! { ::juniper::DefaultScalarValue }
            }
        }
    }

    /// Parses [`Type`] from the given `explicit` [`AttrValue`] (if any),
    /// checking whether it's contained in the giving `generics`.
    #[must_use]
    pub(crate) fn parse(explicit: Option<&AttrValue>, generics: &syn::Generics) -> Self {
        match explicit {
            Some(AttrValue::Concrete(scalar_ty)) => generics
                .params
                .iter()
                .find_map(|p| {
                    if let syn::GenericParam::Type(tp) = p {
                        let ident = &tp.ident;
                        let ty: syn::Type = parse_quote! { #ident };
                        if &ty == scalar_ty {
                            return Some(&tp.ident);
                        }
                    }
                    None
                })
                .map(|ident| Self::ExplicitGeneric(ident.clone()))
                .unwrap_or_else(|| Self::Concrete(scalar_ty.clone())),
            Some(AttrValue::Generic(pred)) => Self::ImplicitGeneric(Some(pred.clone())),
            None => Self::ImplicitGeneric(None),
        }
    }
}
