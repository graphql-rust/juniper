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

use super::{Attr, Definition};

/// [`GraphQLScope`] of errors for `#[graphql_interface]` macro.
const ERR: GraphQLScope = GraphQLScope::InterfaceAttr;

/// Expands `#[graphql_interface]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemTrait>(body.clone()) {
        let trait_attrs = parse::attr::unite(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip("graphql_interface", ast.attrs);
        return expand_on_trait(trait_attrs, ast);
    }
    if let Ok(mut ast) = syn::parse2::<syn::DeriveInput>(body) {
        let trait_attrs = parse::attr::unite(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip("graphql_interface", ast.attrs);
        return expand_on_derive_input(trait_attrs, ast);
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_interface] attribute is applicable to trait and struct definitions only",
    ))
}

/// Expands `#[graphql_interface]` macro placed on trait definition.
fn expand_on_trait(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::ItemTrait,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs("graphql_interface", &attrs)?;

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

    let fields = ast
        .items
        .iter_mut()
        .filter_map(|item| {
            if let syn::TraitItem::Method(m) = item {
                if let Some(f) = parse_trait_method(m, &renaming) {
                    return Some(f);
                }
            }
            None
        })
        .collect::<Vec<_>>();

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

    let enum_alias_ident = attr
        .r#enum
        .as_deref()
        .cloned()
        .unwrap_or_else(|| format_ident!("{}Value", trait_ident.to_string()));
    let enum_ident = attr.r#enum.as_ref().map_or_else(
        || format_ident!("{}ValueEnum", trait_ident.to_string()),
        |c| format_ident!("{}Enum", c.inner().to_string()),
    );

    let generated_code = Definition {
        generics: ast.generics.clone(),
        vis: ast.vis.clone(),
        enum_ident,
        enum_alias_ident,
        name,
        description: attr.description.as_deref().cloned(),
        context,
        scalar,
        fields,
        implemented_for: attr
            .implemented_for
            .iter()
            .map(|c| c.inner().clone())
            .collect(),
        implements: attr.implements.iter().map(|c| c.inner().clone()).collect(),
    };

    Ok(quote! {
        #ast
        #generated_code
    })
}

/// Parses a [`field::Definition`] from the given trait method definition.
///
/// Returns [`None`] if parsing fails, or the method field is ignored.
#[must_use]
fn parse_trait_method(
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

/// Expands `#[graphql_interface]` macro placed on trait definition.
fn expand_on_derive_input(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::DeriveInput,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs("graphql_interface", &attrs)?;

    let trait_ident = &ast.ident;
    let trait_span = ast.span();

    let data = match &mut ast.data {
        syn::Data::Struct(data) => data,
        syn::Data::Enum(_) | syn::Data::Union(_) => {
            return Err(ERR.custom_error(
                ast.span(),
                "#[graphql_interface] attribute is applicable \
                 to trait and struct definitions only",
            ));
        }
    };

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

    let fields = data
        .fields
        .iter_mut()
        .filter_map(|f| parse_struct_field(f, &renaming))
        .collect::<Vec<_>>();

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

    let enum_alias_ident = attr
        .r#enum
        .as_deref()
        .cloned()
        .unwrap_or_else(|| format_ident!("{}Value", trait_ident.to_string()));
    let enum_ident = attr.r#enum.as_ref().map_or_else(
        || format_ident!("{}ValueEnum", trait_ident.to_string()),
        |c| format_ident!("{}Enum", c.inner().to_string()),
    );

    let generated_code = Definition {
        generics: ast.generics.clone(),
        vis: ast.vis.clone(),
        enum_ident,
        enum_alias_ident,
        name,
        description: attr.description.as_deref().cloned(),
        context,
        scalar,
        fields,
        implemented_for: attr
            .implemented_for
            .iter()
            .map(|c| c.inner().clone())
            .collect(),
        implements: attr.implements.iter().map(|c| c.inner().clone()).collect(),
    };

    Ok(quote! {
        #[allow(dead_code)]
        #ast
        #generated_code
    })
}

/// Parses a [`field::Definition`] from the given trait method definition.
///
/// Returns [`None`] if parsing fails, or the method field is ignored.
#[must_use]
fn parse_struct_field(field: &mut syn::Field, renaming: &RenameRule) -> Option<field::Definition> {
    let field_ident = field.ident.as_ref().or_else(|| err_unnamed_field(&field))?;
    let field_attrs = field.attrs.clone();

    // Remove repeated attributes from the method, to omit incorrect expansion.
    field.attrs = mem::take(&mut field.attrs)
        .into_iter()
        .filter(|attr| !path_eq_single(&attr.path, "graphql"))
        .collect();

    let attr = field::Attr::from_attrs("graphql", &field_attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
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
fn err_unnamed_field<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(span.span(), "expected named struct field");
    None
}
