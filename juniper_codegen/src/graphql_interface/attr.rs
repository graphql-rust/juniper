//! Code generation for `#[graphql_interface]` macro.

use std::mem;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::{
    common::{
        field,
        parse::{self, TypeExt as _},
        scalar,
    },
    result::GraphQLScope,
    util::{path_eq_single, span_container::SpanContainer, RenameRule},
};

use super::{inject_async_trait, Definition, TraitAttr};

/// [`GraphQLScope`] of errors for `#[graphql_interface]` macro.
const ERR: GraphQLScope = GraphQLScope::InterfaceAttr;

/// Expands `#[graphql_interface]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemTrait>(body) {
        let trait_attrs = parse::attr::unite(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip("graphql_interface", ast.attrs);
        return expand_on_trait(trait_attrs, ast);
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_interface] attribute is applicable to trait definitions only",
    ))
}

/// Expands `#[graphql_interface]` macro placed on trait definition.
fn expand_on_trait(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::ItemTrait,
) -> syn::Result<TokenStream> {
    let attr = TraitAttr::from_attrs("graphql_interface", &attrs)?;

    let trait_ident = &ast.ident;
    let trait_span = ast.span();

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| trait_ident.unraw().to_string());
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| trait_ident.span()),
        );
    }

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    proc_macro_error::abort_if_dirty();

    let renaming = attr
        .rename_fields
        .as_deref()
        .copied()
        .unwrap_or(RenameRule::CamelCase);

    let mut fields = vec![];
    for item in &mut ast.items {
        if let syn::TraitItem::Method(m) = item {
            if let Some(f) = parse_field(m, &renaming) {
                fields.push(f)
            }
        }
    }

    proc_macro_error::abort_if_dirty();

    if fields.is_empty() {
        ERR.emit_custom(trait_span, "must have at least one field");
    }
    if !field::all_different(&fields) {
        ERR.emit_custom(trait_span, "must have a different name for each field");
    }

    proc_macro_error::abort_if_dirty();

    let context = attr
        .context
        .as_deref()
        .cloned()
        .or_else(|| {
            fields.iter().find_map(|f| {
                f.arguments.as_ref().and_then(|f| {
                    f.iter()
                        .find_map(field::MethodArgument::context_ty)
                        .cloned()
                })
            })
        })
        .unwrap_or_else(|| parse_quote! { () });

    let is_async_trait = attr.asyncness.is_some()
        || ast
            .items
            .iter()
            .find_map(|item| match item {
                syn::TraitItem::Method(m) => m.sig.asyncness,
                _ => None,
            })
            .is_some();
    let has_default_async_methods = ast.items.iter().any(|item| match item {
        syn::TraitItem::Method(m) => m.sig.asyncness.and(m.default.as_ref()).is_some(),
        _ => false,
    });

    let enum_alias_ident = attr
        .r#enum
        .as_deref()
        .cloned()
        .unwrap_or_else(|| format_ident!("{}Value", trait_ident.to_string()));
    let enum_ident = attr.r#enum.as_ref().map_or_else(
        || format_ident!("{}ValueEnum", trait_ident.to_string()),
        |c| format_ident!("{}Enum", c.inner().to_string()),
    );

    let description = attr.description.as_ref().map(|c| c.inner().clone());
    let generated_code = Definition {
        trait_generics: ast.generics.clone(),
        vis: ast.vis.clone(),
        enum_ident,
        enum_alias_ident,
        name,
        description,
        context,
        scalar,
        fields,
        implementers: attr
            .implementers
            .iter()
            .map(|c| c.inner().clone())
            .collect(),
    };

    if is_async_trait {
        if has_default_async_methods {
            // Hack for object safety. See details: https://docs.rs/async-trait/#dyn-traits
            ast.supertraits.push(parse_quote! { Sync });
        }
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

/// Parses [`field::Definition`] from the given trait method definition.
///
/// Returns [`None`] if parsing fails, or the method field is ignored.
#[must_use]
fn parse_field(
    method: &mut syn::TraitItemMethod,
    renaming: &RenameRule,
) -> Option<field::Definition> {
    let method_ident = &method.sig.ident;
    let method_attrs = method.attrs.clone();

    // Remove repeated attributes from the method, to omit incorrect expansion.
    method.attrs = mem::take(&mut method.attrs)
        .into_iter()
        .filter(|attr| !path_eq_single(&attr.path, "graphql"))
        .collect();

    let attr = field::Attr::from_attrs("graphql", &method_attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
        .ok()?;

    if attr.ignore.is_some() {
        return None;
    }

    if method.default.is_some() {
        return err_default_impl_block(&method.default);
    }

    let name = attr
        .name
        .as_ref()
        .map(|m| m.as_ref().value())
        .unwrap_or_else(|| renaming.apply(&method_ident.unraw().to_string()));
    if name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
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
                if rcv.reference.is_none() || rcv.mutability.is_some() {
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
                syn::FnArg::Typed(arg) => field::MethodArgument::parse(arg, renaming, &ERR),
            })
            .collect()
    };

    let mut ty = match &method.sig.output {
        syn::ReturnType::Default => parse_quote! { () },
        syn::ReturnType::Type(_, ty) => ty.unparenthesized().clone(),
    };
    ty.lifetimes_anonymized();

    let description = attr.description.as_ref().map(|d| d.as_ref().value());
    let deprecated = attr
        .deprecated
        .as_deref()
        .map(|d| d.as_ref().map(syn::LitStr::value));

    Some(field::Definition {
        name,
        ty,
        description,
        deprecated,
        ident: method_ident.clone(),
        arguments: Some(arguments),
        has_receiver: method.sig.receiver().is_some(),
        is_async: method.sig.asyncness.is_some(),
    })
}

/// Emits "trait method can't have default impl block" [`syn::Error`] pointing
/// to the given `span`.
fn err_default_impl_block<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(span.span(), "trait method can't have default impl block");
    None
}

/// Emits "invalid trait method receiver" [`syn::Error`] pointing to the given
/// `span`.
#[must_use]
fn err_invalid_method_receiver<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(
        span.span(),
        "trait method receiver can only be a shared reference `&self`",
    );
    None
}

/// Emits "no trait method receiver" [`syn::Error`] pointing to the given
/// `span`.
#[must_use]
fn err_no_method_receiver<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(
        span.span(),
        "trait method should have a shared reference receiver `&self`",
    );
    None
}
