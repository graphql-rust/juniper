pub(crate) mod parse;

use proc_macro2::Span;

/// Retrieves the innermost non-parenthesized [`syn::Type`] from the given one (unwraps nested
/// [`syn::TypeParen`]s asap).
pub(crate) fn unparenthesize(ty: &syn::Type) -> &syn::Type {
    match ty {
        syn::Type::Paren(ty) => unparenthesize(&*ty.elem),
        _ => ty,
    }
}

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
                    syn::TypeParamBound::Trait(_) => todo!(),
                }
            }
        }

        T::Reference(ref_ty) => {
            if let Some(lt) = ref_ty.lifetime.as_mut() {
                lt.ident = syn::Ident::new("_", Span::call_site());
            }
            anonymize_lifetimes(&mut *ref_ty.elem);
        }

        T::BareFn(_) => todo!(),

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

        T::Infer(_) | T::Macro(_) | T::Never(_) | T::Verbatim(_) | T::__Nonexhaustive => {}
    }
}
