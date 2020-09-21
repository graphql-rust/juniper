pub(crate) mod parse;
pub(crate) mod gen;

use proc_macro2::{Span, TokenStream};
use syn::parse_quote;
use quote::ToTokens;

pub(crate) fn anonymize_lifetimes(ty: &mut syn::Type) {
    use syn::{GenericArgument as GA, Type as T};

    match ty {
        T::Array(syn::TypeArray { elem, .. })
        | T::Group(syn::TypeGroup { elem, .. })
        | T::Paren(syn::TypeParen { elem, .. })
        | T::Ptr(syn::TypePtr { elem, .. })
        | T::Slice(syn::TypeSlice { elem, .. }) => anonymize_lifetimes(&mut *elem),

        T::Tuple(syn::TypeTuple { elems, .. }) => {
            for ty in elems.iter_mut() {
                anonymize_lifetimes(ty);
            }
        }

        T::ImplTrait(syn::TypeImplTrait { bounds, .. })
        | T::TraitObject(syn::TypeTraitObject { bounds, .. }) => {
            for bound in bounds.iter_mut() {
                match bound {
                    syn::TypeParamBound::Lifetime(lt) => {
                        lt.ident = syn::Ident::new("_", Span::call_site())
                    }
                    syn::TypeParamBound::Trait(_) => {
                        todo!("Anonymizing lifetimes in trait is not yet supported")
                    }
                }
            }
        }

        T::Reference(ref_ty) => {
            if let Some(lt) = ref_ty.lifetime.as_mut() {
                lt.ident = syn::Ident::new("_", Span::call_site());
            }
            anonymize_lifetimes(&mut *ref_ty.elem);
        }

        T::Path(ty) => {
            for seg in ty.path.segments.iter_mut() {
                match &mut seg.arguments {
                    syn::PathArguments::AngleBracketed(angle) => {
                        for arg in angle.args.iter_mut() {
                            match arg {
                                GA::Lifetime(lt) => {
                                    lt.ident = syn::Ident::new("_", Span::call_site());
                                }
                                GA::Type(ty) => anonymize_lifetimes(ty),
                                GA::Binding(b) => anonymize_lifetimes(&mut b.ty),
                                GA::Constraint(_) | GA::Const(_) => {}
                            }
                        }
                    }
                    syn::PathArguments::Parenthesized(args) => {
                        for ty in args.inputs.iter_mut() {
                            anonymize_lifetimes(ty);
                        }
                        if let syn::ReturnType::Type(_, ty) = &mut args.output {
                            anonymize_lifetimes(&mut *ty);
                        }
                    }
                    syn::PathArguments::None => {}
                }
            }
        }

        // These types unlikely will be used as GraphQL types.
        T::BareFn(_)
        | T::Infer(_)
        | T::Macro(_)
        | T::Never(_)
        | T::Verbatim(_)
        | T::__Nonexhaustive => {}
    }
}

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