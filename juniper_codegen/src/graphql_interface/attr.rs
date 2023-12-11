//! Code generation for `#[graphql_interface]` macro.

use std::mem;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::common::{
    diagnostic, field,
    parse::{self, TypeExt as _},
    path_eq_single, rename, scalar, SpanContainer,
};

use super::{enum_idents, Attr, Definition};

/// [`diagnostic::Scope`] of errors for `#[graphql_interface]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::InterfaceAttr;

/// Expands `#[graphql_interface]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemTrait>(body.clone()) {
        let trait_attrs = parse::attr::unite(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip(["graphql_interface", "graphql"], ast.attrs);
        return expand_on_trait(trait_attrs, ast);
    }
    if let Ok(mut ast) = syn::parse2::<syn::DeriveInput>(body) {
        let trait_attrs = parse::attr::unite(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip(["graphql_interface", "graphql"], ast.attrs);
        return expand_on_derive_input(trait_attrs, ast);
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_interface] attribute is applicable to trait and struct \
         definitions only",
    ))
}

/// Expands `#[graphql_interface]` macro placed on the given trait definition.
fn expand_on_trait(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::ItemTrait,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs(["graphql_interface", "graphql"], &attrs)?;

    let trait_ident = &ast.ident;
    let trait_span = ast.span();

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| trait_ident.unraw().to_string())
        .into_boxed_str();
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| trait_ident.span()),
        );
    }

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    diagnostic::abort_if_dirty();

    let renaming = attr
        .rename_fields
        .as_deref()
        .copied()
        .unwrap_or(rename::Policy::CamelCase);

    let fields = ast
        .items
        .iter_mut()
        .filter_map(|item| {
            if let syn::TraitItem::Fn(m) = item {
                return parse_trait_method(m, &renaming);
            }
            None
        })
        .collect::<Vec<_>>();

    diagnostic::abort_if_dirty();

    if fields.is_empty() {
        ERR.emit_custom(trait_span, "must have at least one field");
    }
    if !field::all_different(&fields) {
        ERR.emit_custom(trait_span, "must have a different name for each field");
    }

    diagnostic::abort_if_dirty();

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

    let (enum_ident, enum_alias_ident) = enum_idents(trait_ident, attr.r#enum.as_deref());

    let generated_code = Definition {
        generics: ast.generics.clone(),
        vis: ast.vis.clone(),
        enum_ident,
        enum_alias_ident,
        name,
        description: attr.description.map(SpanContainer::into_inner),
        context,
        scalar,
        fields,
        implemented_for: attr
            .implemented_for
            .into_iter()
            .map(SpanContainer::into_inner)
            .collect(),
        implements: attr
            .implements
            .into_iter()
            .map(SpanContainer::into_inner)
            .collect(),
        suppress_dead_code: None,
        src_intra_doc_link: format!("trait@{trait_ident}").into_boxed_str(),
    };

    Ok(quote! {
        // Omit enforcing `# Errors` and `# Panics` sections in GraphQL descriptions.
        #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
        #ast
        #generated_code
    })
}

/// Parses a [`field::Definition`] from the given trait method definition.
///
/// Returns [`None`] if the parsing fails, or the method field is ignored.
#[must_use]
fn parse_trait_method(
    method: &mut syn::TraitItemFn,
    renaming: &rename::Policy,
) -> Option<field::Definition> {
    let method_ident = &method.sig.ident;
    let method_attrs = method.attrs.clone();

    // Remove repeated attributes from the method, to omit incorrect expansion.
    method.attrs = mem::take(&mut method.attrs)
        .into_iter()
        .filter(|attr| !path_eq_single(attr.path(), "graphql"))
        .collect();

    let attr = field::Attr::from_attrs("graphql", &method_attrs)
        .map_err(diagnostic::emit_error)
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

    let arguments = method
        .sig
        .inputs
        .iter_mut()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(arg) => field::MethodArgument::parse(arg, renaming, &ERR),
        })
        .collect();

    let mut ty = match &method.sig.output {
        syn::ReturnType::Default => parse_quote! { () },
        syn::ReturnType::Type(_, ty) => ty.unparenthesized().clone(),
    };
    ty.lifetimes_anonymized();

    Some(field::Definition {
        name,
        ty,
        description: attr.description.map(SpanContainer::into_inner),
        deprecated: attr.deprecated.map(SpanContainer::into_inner),
        ident: method_ident.clone(),
        arguments: Some(arguments),
        has_receiver: method.sig.receiver().is_some(),
        is_async: method.sig.asyncness.is_some(),
    })
}

/// Expands `#[graphql_interface]` macro placed on the given struct.
fn expand_on_derive_input(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::DeriveInput,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs(["graphql_interface", "graphql"], &attrs)?;

    let struct_ident = &ast.ident;
    let struct_span = ast.span();

    let data = match &mut ast.data {
        syn::Data::Struct(data) => data,
        syn::Data::Enum(_) | syn::Data::Union(_) => {
            return Err(ERR.custom_error(
                ast.span(),
                "#[graphql_interface] attribute is applicable to trait and \
                 struct definitions only",
            ));
        }
    };

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| struct_ident.unraw().to_string())
        .into_boxed_str();
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| struct_ident.span()),
        );
    }

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    diagnostic::abort_if_dirty();

    let renaming = attr
        .rename_fields
        .as_deref()
        .copied()
        .unwrap_or(rename::Policy::CamelCase);

    let fields = data
        .fields
        .iter_mut()
        .filter_map(|f| parse_struct_field(f, &renaming))
        .collect::<Vec<_>>();

    diagnostic::abort_if_dirty();

    if fields.is_empty() {
        ERR.emit_custom(struct_span, "must have at least one field");
    }
    if !field::all_different(&fields) {
        ERR.emit_custom(struct_span, "must have a different name for each field");
    }

    diagnostic::abort_if_dirty();

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

    let (enum_ident, enum_alias_ident) = enum_idents(struct_ident, attr.r#enum.as_deref());
    let generated_code = Definition {
        generics: ast.generics.clone(),
        vis: ast.vis.clone(),
        enum_ident,
        enum_alias_ident,
        name,
        description: attr.description.map(SpanContainer::into_inner),
        context,
        scalar,
        fields,
        implemented_for: attr
            .implemented_for
            .into_iter()
            .map(SpanContainer::into_inner)
            .collect(),
        implements: attr
            .implements
            .into_iter()
            .map(SpanContainer::into_inner)
            .collect(),
        suppress_dead_code: None,
        src_intra_doc_link: format!("struct@{struct_ident}").into_boxed_str(),
    };

    Ok(quote! {
        #[allow(dead_code)]
        #ast
        #generated_code
    })
}

/// Parses a [`field::Definition`] from the given struct field definition.
///
/// Returns [`None`] if the parsing fails, or the struct field is ignored.
#[must_use]
fn parse_struct_field(
    field: &mut syn::Field,
    renaming: &rename::Policy,
) -> Option<field::Definition> {
    let field_ident = field.ident.as_ref().or_else(|| err_unnamed_field(&field))?;
    let field_attrs = field.attrs.clone();

    // Remove repeated attributes from the method, to omit incorrect expansion.
    field.attrs = mem::take(&mut field.attrs)
        .into_iter()
        .filter(|attr| !path_eq_single(attr.path(), "graphql"))
        .collect();

    let attr = field::Attr::from_attrs("graphql", &field_attrs)
        .map_err(diagnostic::emit_error)
        .ok()?;

    if attr.ignore.is_some() {
        return None;
    }

    let name = attr
        .name
        .as_ref()
        .map(|m| m.as_ref().value())
        .unwrap_or_else(|| renaming.apply(&field_ident.unraw().to_string()));
    if name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| field_ident.span()),
        );
        return None;
    }

    let mut ty = field.ty.clone();
    ty.lifetimes_anonymized();

    Some(field::Definition {
        name,
        ty,
        description: attr.description.map(SpanContainer::into_inner),
        deprecated: attr.deprecated.map(SpanContainer::into_inner),
        ident: field_ident.clone(),
        arguments: None,
        has_receiver: false,
        is_async: false,
    })
}

/// Emits "trait method can't have default implementation" [`syn::Error`]
/// pointing to the given `span`.
fn err_default_impl_block<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(
        span.span(),
        "trait method can't have default implementation",
    );
    None
}

/// Emits "expected named struct field" [`syn::Error`] pointing to the given
/// `span`.
pub(crate) fn err_unnamed_field<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(span.span(), "expected named struct field");
    None
}
