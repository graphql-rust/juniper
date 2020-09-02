//! Code generation for `#[graphql_interface]` macro.

use std::mem;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::{
    result::GraphQLScope,
    util::{
        path_eq_single, span_container::SpanContainer, strip_attrs, to_camel_case, unite_attrs,
        unparenthesize,
    },
};

use super::{
    ArgumentMeta, FieldMeta, ImplementerMeta, InterfaceDefinition, InterfaceFieldArgument,
    InterfaceFieldArgumentDefinition, InterfaceFieldDefinition, InterfaceImplementerDefinition,
    InterfaceMeta,
};

/// [`GraphQLScope`] of errors for `#[graphql_interface]` macro.
const ERR: GraphQLScope = GraphQLScope::InterfaceAttr;

/// Expands `#[graphql_interface]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemTrait>(body.clone()) {
        let trait_attrs = unite_attrs(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = strip_attrs("graphql_interface", ast.attrs);
        return expand_on_trait(trait_attrs, ast);
    } else if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        if ast.trait_.is_some() {
            let impl_attrs = unite_attrs(("graphql_interface", &attr_args), &ast.attrs);
            ast.attrs = strip_attrs("graphql_interface", ast.attrs);
            return expand_on_impl(impl_attrs, ast);
        }
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_interface] attribute is applicable to trait definitions and trait \
         implementations only",
    ))
}

/// Expands `#[graphql_interface]` macro placed on trait definition.
pub fn expand_on_trait(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::ItemTrait,
) -> syn::Result<TokenStream> {
    let meta = InterfaceMeta::from_attrs("graphql_interface", &attrs)?;

    let trait_ident = &ast.ident;

    let name = meta
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| trait_ident.unraw().to_string());
    if !meta.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            meta.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| trait_ident.span()),
        );
    }

    let context = meta.context.map(SpanContainer::into_inner);
    //.or_else(|| variants.iter().find_map(|v| v.context_ty.as_ref()).cloned());

    let implementers = meta
        .implementers
        .iter()
        .map(|ty| {
            let span = ty.span_ident();
            InterfaceImplementerDefinition {
                ty: ty.as_ref().clone(),
                downcast_code: None,
                downcast_check: None,
                context_ty: None,
                span,
            }
        })
        .collect();

    proc_macro_error::abort_if_dirty();

    let fields = ast
        .items
        .iter_mut()
        .filter_map(|item| match item {
            syn::TraitItem::Method(m) => parse_field_from_trait_method(m),
            _ => None,
        })
        .collect();

    proc_macro_error::abort_if_dirty();

    let is_async_trait = meta.asyncness.is_some()
        || ast
            .items
            .iter()
            .find_map(|item| match item {
                syn::TraitItem::Method(m) => m.sig.asyncness,
                _ => None,
            })
            .is_some();
    let has_default_async_methods = ast
        .items
        .iter()
        .find(|item| match item {
            syn::TraitItem::Method(m) => m.sig.asyncness.and(m.default.as_ref()).is_some(),
            _ => false,
        })
        .is_some();

    let generated_code = InterfaceDefinition {
        name,
        ty: parse_quote! { #trait_ident },
        trait_object: Some(meta.alias.map(|a| a.as_ref().clone())),
        visibility: ast.vis.clone(),
        description: meta.description.map(SpanContainer::into_inner),
        context,
        scalar: meta.scalar.map(SpanContainer::into_inner),
        generics: ast.generics.clone(),
        fields,
        implementers,
    };

    ast.generics.params.push(parse_quote! {
        GraphQLScalarValue: ::juniper::ScalarValue = ::juniper::DefaultScalarValue
    });
    ast.supertraits.push(parse_quote! {
        ::juniper::AsDynGraphQLValue<GraphQLScalarValue>
    });
    if is_async_trait && has_default_async_methods {
        // Hack for object safety. See details: https://docs.rs/async-trait/#dyn-traits
        ast.supertraits.push(parse_quote! { Sync });
    }
    ast.attrs
        .push(parse_quote! { #[allow(unused_qualifications, clippy::type_repetition_in_bounds)] });
    if is_async_trait {
        inject_async_trait(
            &mut ast.attrs,
            ast.items.iter_mut().filter_map(|i| {
                if let syn::TraitItem::Method(m) = i {
                    Some(&mut m.sig)
                } else {
                    None
                }
            }),
            &ast.generics,
        );
    }

    Ok(quote! {
        #ast

        #generated_code
    })
}

/// Expands `#[graphql_interface]` macro placed on trait implementation block.
pub fn expand_on_impl(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::ItemImpl,
) -> syn::Result<TokenStream> {
    let meta = ImplementerMeta::from_attrs("graphql_interface", &attrs)?;

    let is_async_trait = meta.asyncness.is_some()
        || ast
            .items
            .iter()
            .find_map(|item| match item {
                syn::ImplItem::Method(m) => m.sig.asyncness,
                _ => None,
            })
            .is_some();

    let is_generic_scalar = meta.scalar.is_none();

    ast.attrs
        .push(parse_quote! { #[allow(unused_qualifications, clippy::type_repetition_in_bounds)] });

    if is_generic_scalar {
        ast.generics.params.push(parse_quote! {
            GraphQLScalarValue: ::juniper::ScalarValue + Send + Sync
        });
    }

    let (_, trait_path, _) = ast.trait_.as_mut().unwrap();
    let trait_params = &mut trait_path.segments.last_mut().unwrap().arguments;
    if let syn::PathArguments::None = trait_params {
        *trait_params = syn::PathArguments::AngleBracketed(parse_quote! { <> });
    }
    if let syn::PathArguments::AngleBracketed(a) = trait_params {
        a.args.push(if is_generic_scalar {
            parse_quote! { GraphQLScalarValue }
        } else {
            syn::GenericArgument::Type(meta.scalar.clone().unwrap().into_inner())
        });
    }

    if is_async_trait {
        inject_async_trait(
            &mut ast.attrs,
            ast.items.iter_mut().filter_map(|i| {
                if let syn::ImplItem::Method(m) = i {
                    Some(&mut m.sig)
                } else {
                    None
                }
            }),
            &ast.generics,
        );
    }

    Ok(quote! { #ast })
}

fn inject_async_trait<'m, M>(attrs: &mut Vec<syn::Attribute>, methods: M, generics: &syn::Generics)
where
    M: IntoIterator<Item = &'m mut syn::Signature>,
{
    attrs.push(parse_quote! { #[allow(clippy::type_repetition_in_bounds)] });
    attrs.push(parse_quote! { #[::juniper::async_trait] });

    for method in methods.into_iter() {
        if method.asyncness.is_some() {
            let where_clause = &mut method.generics.make_where_clause().predicates;
            for p in &generics.params {
                let ty_param = match p {
                    syn::GenericParam::Type(t) => {
                        let ty_param = &t.ident;
                        quote! { #ty_param }
                    }
                    syn::GenericParam::Lifetime(l) => {
                        let ty_param = &l.lifetime;
                        quote! { #ty_param }
                    }
                    syn::GenericParam::Const(_) => continue,
                };
                where_clause.push(parse_quote! { #ty_param: 'async_trait });
            }
        }
    }
}

fn parse_field_from_trait_method(
    method: &mut syn::TraitItemMethod,
) -> Option<InterfaceFieldDefinition> {
    let method_attrs = method.attrs.clone();

    // Remove repeated attributes from the method, to omit incorrect expansion.
    method.attrs = mem::take(&mut method.attrs)
        .into_iter()
        .filter(|attr| !path_eq_single(&attr.path, "graphql_interface"))
        .collect();

    let meta = FieldMeta::from_attrs("graphql_interface", &method_attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
        .ok()?;

    if meta.ignore.is_some() {
        return None;
    }

    let method_ident = &method.sig.ident;

    let name = meta
        .name
        .as_ref()
        .map(|m| m.as_ref().value())
        .unwrap_or_else(|| to_camel_case(&method_ident.unraw().to_string()));
    if name.starts_with("__") {
        ERR.no_double_underscore(
            meta.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| method_ident.span()),
        );
        return None;
    }

    let arguments = {
        if method.sig.inputs.is_empty() {
            return err_no_method_receiver(&method.sig.inputs);
        }
        let mut args_iter = method.sig.inputs.iter_mut();
        match args_iter.next().unwrap() {
            syn::FnArg::Receiver(rcv) => {
                if !rcv.reference.is_some() || rcv.mutability.is_some() {
                    return err_invalid_method_receiver(rcv);
                }
            }
            syn::FnArg::Typed(arg) => {
                if let syn::Pat::Ident(a) = &*arg.pat {
                    if a.ident.to_string().as_str() != "self" {
                        return err_invalid_method_receiver(arg);
                    }
                }
                return err_no_method_receiver(arg);
            }
        };
        args_iter
            .filter_map(|arg| match arg {
                syn::FnArg::Receiver(_) => None,
                syn::FnArg::Typed(arg) => parse_field_argument_from_method_argument(arg),
            })
            .collect()
    };

    let ty = match &method.sig.output {
        syn::ReturnType::Default => parse_quote! { () },
        syn::ReturnType::Type(_, ty) => unparenthesize(&*ty).clone(),
    };

    let description = meta.description.as_ref().map(|d| d.as_ref().value());
    let deprecated = meta
        .deprecated
        .as_ref()
        .map(|d| d.as_ref().as_ref().map(syn::LitStr::value));

    Some(InterfaceFieldDefinition {
        name,
        ty,
        description,
        deprecated,
        method: method_ident.clone(),
        arguments,
        is_async: method.sig.asyncness.is_some(),
    })
}

fn parse_field_argument_from_method_argument(
    argument: &mut syn::PatType,
) -> Option<InterfaceFieldArgument> {
    let argument_attrs = argument.attrs.clone();

    // Remove repeated attributes from the method, to omit incorrect expansion.
    argument.attrs = mem::take(&mut argument.attrs)
        .into_iter()
        .filter(|attr| !path_eq_single(&attr.path, "graphql_interface"))
        .collect();

    let meta = ArgumentMeta::from_attrs("graphql_interface", &argument_attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
        .ok()?;

    if meta.context.is_some() {
        if let Some(span) = &meta.executor {
            return err_arg_both_context_and_executor(&span);
        }
        ensure_no_regular_field_argument_meta(&meta)?;
        return Some(InterfaceFieldArgument::Context);
    }

    if meta.executor.is_some() {
        if let Some(span) = &meta.context {
            return err_arg_both_context_and_executor(&span);
        }
        ensure_no_regular_field_argument_meta(&meta)?;
        return Some(InterfaceFieldArgument::Executor);
    }

    if let syn::Pat::Ident(name) = &*argument.pat {
        let arg = match name.ident.unraw().to_string().as_str() {
            "context" | "ctx" => Some(InterfaceFieldArgument::Context),
            "executor" => Some(InterfaceFieldArgument::Executor),
            _ => None,
        };
        if arg.is_some() {
            ensure_no_regular_field_argument_meta(&meta)?;
            return arg;
        }
    }

    let name = if let Some(name) = meta.name.as_ref() {
        name.as_ref().value()
    } else if let syn::Pat::Ident(name) = &*argument.pat {
        to_camel_case(&name.ident.unraw().to_string())
    } else {
        ERR.custom(
            argument.pat.span(),
            "trait method argument should be declared as a single identifier",
        )
        .note(String::from(
            "use `#[graphql_interface(name = ...)]` attribute to specify custom argument's name \
             without requiring it being a single identifier",
        ))
        .emit();
        return None;
    };
    if name.starts_with("__") {
        ERR.no_double_underscore(
            meta.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| argument.pat.span()),
        );
        return None;
    }

    Some(InterfaceFieldArgument::Regular(
        InterfaceFieldArgumentDefinition {
            name,
            ty: argument.ty.as_ref().clone(),
            description: meta.description.as_ref().map(|d| d.as_ref().value()),
            default: meta.default.as_ref().map(|v| v.as_ref().clone()),
        },
    ))
}

fn ensure_no_regular_field_argument_meta(meta: &ArgumentMeta) -> Option<()> {
    if let Some(span) = &meta.name {
        return err_invalid_arg_meta(&span, "name");
    }
    if let Some(span) = &meta.description {
        return err_invalid_arg_meta(&span, "description");
    }
    if let Some(span) = &meta.default {
        return err_invalid_arg_meta(&span, "default");
    }
    Some(())
}

fn err_invalid_method_receiver<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.custom(
        span.span(),
        "trait method receiver can only be a shared reference `&self`",
    )
    .emit();
    return None;
}

fn err_no_method_receiver<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.custom(
        span.span(),
        "trait method should have a shared reference receiver `&self`",
    )
    .emit();
    return None;
}

fn err_arg_both_context_and_executor<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.custom(
        span.span(),
        "trait method argument cannot be both `juniper::Context` and `juniper::Executor` at the \
         same time",
    )
    .emit();
    return None;
}

fn err_invalid_arg_meta<T, S: Spanned>(span: &S, attr: &str) -> Option<T> {
    ERR.custom(
        span.span(),
        format!(
            "attribute `#[graphql_interface({} = ...)]` is not allowed here",
            attr
        ),
    )
    .emit();
    return None;
}
