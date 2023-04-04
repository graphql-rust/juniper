//! Common functions, definitions and extensions for parsing downcasting functions, used by GraphQL
//! [interfaces][1] and [unions][2] definitions to downcast its type to a concrete implementer type.
//!
//! [1]: https://spec.graphql.org/October2021#sec-Interfaces
//! [2]: https://spec.graphql.org/October2021#sec-Unions

use proc_macro2::Span;
use syn::{ext::IdentExt as _, spanned::Spanned as _};

use crate::common::parse::TypeExt as _;

/// Parses downcasting output type from the downcaster method return type.
///
/// # Errors
///
/// If return type is invalid (not `Option<&OutputType>`), then returns the [`Span`] to display the
/// corresponding error at.
pub(crate) fn output_type(ret_ty: &syn::ReturnType) -> Result<syn::Type, Span> {
    let ret_ty = match &ret_ty {
        syn::ReturnType::Type(_, ty) => &**ty,
        _ => return Err(ret_ty.span()),
    };

    let path = match ret_ty.unparenthesized() {
        syn::Type::Path(syn::TypePath { qself: None, path }) => path,
        _ => return Err(ret_ty.span()),
    };

    let (ident, args) = match path.segments.last() {
        Some(syn::PathSegment {
            ident,
            arguments: syn::PathArguments::AngleBracketed(generic),
        }) => (ident, &generic.args),
        _ => return Err(ret_ty.span()),
    };

    if ident.unraw() != "Option" {
        return Err(ret_ty.span());
    }
    if args.len() != 1 {
        return Err(ret_ty.span());
    }

    let out_ty = match args.first() {
        Some(syn::GenericArgument::Type(inner_ty)) => match inner_ty.unparenthesized() {
            syn::Type::Reference(inner_ty) => {
                if inner_ty.mutability.is_some() {
                    return Err(inner_ty.span());
                }
                inner_ty.elem.unparenthesized().clone()
            }
            _ => return Err(ret_ty.span()),
        },
        _ => return Err(ret_ty.span()),
    };
    Ok(out_ty)
}

/// Parses context type used for downcasting from the downcaster method signature.
///
/// Returns [`None`] if downcaster method doesn't accept context.
///
/// # Errors
///
/// If input arguments are invalid, then returns the [`Span`] to display the corresponding error at.
pub(crate) fn context_ty(sig: &syn::Signature) -> Result<Option<syn::Type>, Span> {
    match sig.receiver() {
        Some(rcv) => {
            if rcv.reference.is_none() || rcv.mutability.is_some() {
                return Err(rcv.span());
            }
        }
        _ => return Err(sig.span()),
    }

    if sig.inputs.len() > 2 {
        return Err(sig.inputs.span());
    }

    let second_arg_ty = match sig.inputs.iter().nth(1) {
        Some(syn::FnArg::Typed(arg)) => &*arg.ty,
        None => return Ok(None),
        _ => return Err(sig.inputs.span()),
    };
    match second_arg_ty.unparenthesized() {
        syn::Type::Reference(ref_ty) => {
            if ref_ty.mutability.is_some() {
                return Err(ref_ty.span());
            }
            Ok(Some(ref_ty.elem.unparenthesized().clone()))
        }
        ty => Err(ty.span()),
    }
}
