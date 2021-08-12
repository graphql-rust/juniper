//! Code generation for `#[graphql_object]` macro.

use std::{any::TypeId, marker::PhantomData, mem};

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
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

use super::{Attr, Definition, Query};

/// [`GraphQLScope`] of errors for `#[graphql_object]` macro.
const ERR: GraphQLScope = GraphQLScope::ObjectAttr;

/// Expands `#[graphql_object]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        if ast.trait_.is_none() {
            let impl_attrs = parse::attr::unite(("graphql_object", &attr_args), &ast.attrs);
            ast.attrs = parse::attr::strip("graphql_object", ast.attrs);
            return expand_on_impl::<Query>(Attr::from_attrs("graphql_object", &impl_attrs)?, ast);
        }
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_object] attribute is applicable to non-trait `impl` blocks only",
    ))
}

/// Expands `#[graphql_object]` macro placed on an implementation block.
pub(crate) fn expand_on_impl<Operation>(
    attr: Attr,
    mut ast: syn::ItemImpl,
) -> syn::Result<TokenStream>
where
    Definition<Operation>: ToTokens,
    Operation: 'static,
{
    let type_span = ast.self_ty.span();
    let type_ident = ast.self_ty.topmost_ident().ok_or_else(|| {
        ERR.custom_error(type_span, "could not determine ident for the `impl` type")
    })?;

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| type_ident.unraw().to_string());
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| type_ident.span()),
        );
    }

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    proc_macro_error::abort_if_dirty();

    let renaming = attr
        .rename_fields
        .as_deref()
        .copied()
        .unwrap_or(RenameRule::CamelCase);

    let async_only = TypeId::of::<Operation>() != TypeId::of::<Query>();
    let fields: Vec<_> = ast
        .items
        .iter_mut()
        .filter_map(|item| {
            if let syn::ImplItem::Method(m) = item {
                parse_field(m, async_only, &renaming)
            } else {
                None
            }
        })
        .collect();

    proc_macro_error::abort_if_dirty();

    if fields.is_empty() {
        ERR.emit_custom(type_span, "must have at least one field");
    }
    if !field::all_different(&fields) {
        ERR.emit_custom(type_span, "must have a different name for each field");
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

    let generated_code = Definition::<Operation> {
        name,
        ty: ast.self_ty.unparenthesized().clone(),
        generics: ast.generics.clone(),
        description: attr.description.map(SpanContainer::into_inner),
        context,
        scalar,
        fields,
        interfaces: attr
            .interfaces
            .iter()
            .map(|ty| ty.as_ref().clone())
            .collect(),
        _operation: PhantomData,
    };

    Ok(quote! {
        #ast
        #generated_code
    })
}

/// Parses a [`field::Definition`] from the given Rust [`syn::ImplItemMethod`].
///
/// Returns [`None`] if parsing fails, or the method field is ignored.
#[must_use]
fn parse_field(
    method: &mut syn::ImplItemMethod,
    async_only: bool,
    renaming: &RenameRule,
) -> Option<field::Definition> {
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

    if async_only && method.sig.asyncness.is_none() {
        return err_no_sync_resolvers(&method.sig);
    }

    let method_ident = &method.sig.ident;

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
        if let Some(arg) = method.sig.inputs.first() {
            match arg {
                syn::FnArg::Receiver(rcv) => {
                    if rcv.reference.is_none() || rcv.mutability.is_some() {
                        return err_invalid_method_receiver(rcv);
                    }
                }
                syn::FnArg::Typed(arg) => {
                    if let syn::Pat::Ident(a) = &*arg.pat {
                        if a.ident.to_string().as_str() == "self" {
                            return err_invalid_method_receiver(arg);
                        }
                    }
                }
            }
        }
        method
            .sig
            .inputs
            .iter_mut()
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

/// Emits "invalid method receiver" [`syn::Error`] pointing to the given `span`.
#[must_use]
fn err_invalid_method_receiver<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(
        span.span(),
        "method should have a shared reference receiver `&self`, or no receiver at all",
    );
    None
}

/// Emits "synchronous resolvers are not supported" [`syn::Error`] pointing to
/// the given `span`.
#[must_use]
fn err_no_sync_resolvers<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.custom(span.span(), "synchronous resolvers are not supported")
        .note("Specify that this function is async: `async fn foo()`".into())
        .emit();
    None
}
