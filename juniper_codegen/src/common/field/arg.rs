//! Common functions, definitions and extensions for parsing and code generation
//! of [GraphQL arguments][1]
//!
//! [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments.

use std::mem;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token,
};

use crate::{
    common::{
        parse::{
            attr::{err, OptionExt as _},
            ParseBufferExt as _, TypeExt as _,
        },
        scalar,
    },
    result::GraphQLScope,
    util::{filter_attrs, path_eq_single, span_container::SpanContainer, RenameRule},
};

/// Available metadata (arguments) behind `#[graphql]` attribute placed on a
/// method argument, when generating code for [GraphQL argument][1].
///
/// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
#[derive(Debug, Default)]
pub(crate) struct Attr {
    /// Explicitly specified name of a [GraphQL argument][1] represented by this
    /// method argument.
    ///
    /// If [`None`], then `camelCased` Rust argument name is used by default.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    pub(crate) name: Option<SpanContainer<syn::LitStr>>,

    /// Explicitly specified [description][2] of this [GraphQL argument][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    pub(crate) description: Option<SpanContainer<syn::LitStr>>,

    /// Explicitly specified [default value][2] of this [GraphQL argument][1].
    ///
    /// If the exact default expression is not specified, then the [`Default`]
    /// value is used.
    ///
    /// If [`None`], then this [GraphQL argument][1] is considered as
    /// [required][2].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/June2018/#sec-Required-Arguments
    pub(crate) default: Option<SpanContainer<Option<syn::Expr>>>,

    /// Explicitly specified marker indicating that this method argument doesn't
    /// represent a [GraphQL argument][1], but is a [`Context`] being injected
    /// into a [GraphQL field][2] resolving function.
    ///
    /// If absent, then the method argument still is considered as [`Context`]
    /// if it's named `context` or `ctx`.
    ///
    /// [`Context`]: juniper::Context
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) context: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified marker indicating that this method argument doesn't
    /// represent a [GraphQL argument][1], but is an [`Executor`] being injected
    /// into a [GraphQL field][2] resolving function.
    ///
    /// If absent, then the method argument still is considered as [`Executor`]
    /// if it's named `executor`.
    ///
    /// [`Executor`]: juniper::Executor
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) executor: Option<SpanContainer<syn::Ident>>,
}

impl Parse for Attr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut out = Self::default();
        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    let name = input.parse::<syn::LitStr>()?;
                    out.name
                        .replace(SpanContainer::new(ident.span(), Some(name.span()), name))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "desc" | "description" => {
                    input.parse::<token::Eq>()?;
                    let desc = input.parse::<syn::LitStr>()?;
                    out.description
                        .replace(SpanContainer::new(ident.span(), Some(desc.span()), desc))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "default" => {
                    let mut expr = None;
                    if input.is_next::<token::Eq>() {
                        input.parse::<token::Eq>()?;
                        expr = Some(input.parse::<syn::Expr>()?);
                    } else if input.is_next::<token::Paren>() {
                        let inner;
                        let _ = syn::parenthesized!(inner in input);
                        expr = Some(inner.parse::<syn::Expr>()?);
                    }
                    out.default
                        .replace(SpanContainer::new(
                            ident.span(),
                            expr.as_ref().map(|e| e.span()),
                            expr,
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "ctx" | "context" | "Context" => {
                    let span = ident.span();
                    out.context
                        .replace(SpanContainer::new(span, Some(span), ident))
                        .none_or_else(|_| err::dup_arg(span))?
                }
                "exec" | "executor" => {
                    let span = ident.span();
                    out.executor
                        .replace(SpanContainer::new(span, Some(span), ident))
                        .none_or_else(|_| err::dup_arg(span))?
                }
                name => {
                    return Err(err::unknown_arg(&ident, name));
                }
            }
            input.try_parse::<token::Comma>()?;
        }
        Ok(out)
    }
}

impl Attr {
    /// Tries to merge two [`Attr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            default: try_merge_opt!(default: self, another),
            context: try_merge_opt!(context: self, another),
            executor: try_merge_opt!(executor: self, another),
        })
    }

    /// Parses [`Attr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a function argument.
    pub(crate) fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let attr = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if let Some(context) = &attr.context {
            if attr.name.is_some()
                || attr.description.is_some()
                || attr.default.is_some()
                || attr.executor.is_some()
            {
                return Err(syn::Error::new(
                    context.span(),
                    "`context` attribute argument is not composable with any other arguments",
                ));
            }
        }

        if let Some(executor) = &attr.executor {
            if attr.name.is_some()
                || attr.description.is_some()
                || attr.default.is_some()
                || attr.context.is_some()
            {
                return Err(syn::Error::new(
                    executor.span(),
                    "`executor` attribute argument is not composable with any other arguments",
                ));
            }
        }

        Ok(attr)
    }

    /// Checks whether this [`Attr`] doesn't contain arguments related to an
    /// [`OnField`] argument.
    fn ensure_no_regular_arguments(&self) -> syn::Result<()> {
        if let Some(span) = &self.name {
            return Err(Self::err_disallowed(&span, "name"));
        }
        if let Some(span) = &self.description {
            return Err(Self::err_disallowed(&span, "description"));
        }
        if let Some(span) = &self.default {
            return Err(Self::err_disallowed(&span, "default"));
        }
        Ok(())
    }

    /// Emits "argument is not allowed" [`syn::Error`] for the given `arg`
    /// pointing to the given `span`.
    #[must_use]
    fn err_disallowed<S: Spanned>(span: &S, arg: &str) -> syn::Error {
        syn::Error::new(
            span.span(),
            format!(
                "attribute argument `#[graphql({} = ...)]` is not allowed here",
                arg,
            ),
        )
    }
}

/// Representation of a [GraphQL field argument][1] for code generation.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
#[derive(Debug)]
pub(crate) struct OnField {
    /// Rust type that this [GraphQL field argument][1] is represented by.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    pub(crate) ty: syn::Type,

    /// Name of this [GraphQL field argument][2] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    pub(crate) name: String,

    /// [Description][2] of this [GraphQL field argument][1] to put into GraphQL
    /// schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    pub(crate) description: Option<String>,

    /// Default value of this [GraphQL field argument][1] in GraphQL schema.
    ///
    /// If outer [`Option`] is [`None`], then this [argument][1] is a
    /// [required][2] one.
    ///
    /// If inner [`Option`] is [`None`], then the [`Default`] value is used.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/June2018/#sec-Required-Arguments
    pub(crate) default: Option<Option<syn::Expr>>,
}

/// Possible kinds of Rust method arguments for code generation.
#[derive(Debug)]
pub(crate) enum OnMethod {
    /// Regular [GraphQL field argument][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Arguments
    Regular(OnField),

    /// [`Context`] passed into a [GraphQL field][2] resolving method.
    ///
    /// [`Context`]: juniper::Context
    /// [2]: https://spec.graphql.org/June2018/#sec-Language.Fields
    Context(syn::Type),

    /// [`Executor`] passed into a [GraphQL field][2] resolving method.
    ///
    /// [`Executor`]: juniper::Executor
    /// [2]: https://spec.graphql.org/June2018/#sec-Language.Fields
    Executor,
}

impl OnMethod {
    /// Returns this argument as the one [`OnField`], if it represents the one.
    #[must_use]
    pub(crate) fn as_regular(&self) -> Option<&OnField> {
        if let Self::Regular(arg) = self {
            Some(arg)
        } else {
            None
        }
    }

    /// Returns [`syn::Type`] of this [`OnMethod::Context`], if it represents
    /// the one.
    #[must_use]
    pub(crate) fn context_ty(&self) -> Option<&syn::Type> {
        if let Self::Context(ty) = self {
            Some(ty)
        } else {
            None
        }
    }

    /// Returns generated code for the [`marker::IsOutputType::mark`] method,
    /// which performs static checks for this argument, if it represents an
    /// [`OnField`] one.
    ///
    /// [`marker::IsOutputType::mark`]: juniper::marker::IsOutputType::mark
    #[must_use]
    pub(crate) fn method_mark_tokens(&self, scalar: &scalar::Type) -> Option<TokenStream> {
        let ty = &self.as_regular()?.ty;
        Some(quote! {
            <#ty as ::juniper::marker::IsInputType<#scalar>>::mark();
        })
    }

    /// Returns generated code for the [`GraphQLType::meta`] method, which
    /// registers this argument in [`Registry`], if it represents an [`OnField`]
    /// argument.
    ///
    /// [`GraphQLType::meta`]: juniper::GraphQLType::meta
    /// [`Registry`]: juniper::Registry
    #[must_use]
    pub(crate) fn method_meta_tokens(&self) -> Option<TokenStream> {
        let arg = self.as_regular()?;

        let (name, ty) = (&arg.name, &arg.ty);

        let description = arg
            .description
            .as_ref()
            .map(|desc| quote! { .description(#desc) });

        let method = if let Some(val) = &arg.default {
            let val = val
                .as_ref()
                .map(|v| quote! { (#v).into() })
                .unwrap_or_else(|| quote! { <#ty as Default>::default() });
            quote! { .arg_with_default::<#ty>(#name, &#val, info) }
        } else {
            quote! { .arg::<#ty>(#name, info) }
        };

        Some(quote! { .argument(registry#method#description) })
    }

    /// Returns generated code for the [`GraphQLValue::resolve_field`] method,
    /// which provides the value of this [`OnMethod`] argument to be passed into
    /// a trait method call.
    ///
    /// [`GraphQLValue::resolve_field`]: juniper::GraphQLValue::resolve_field
    #[must_use]
    pub(crate) fn method_resolve_field_tokens(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Regular(arg) => {
                let (name, ty) = (&arg.name, &arg.ty);
                let err_text = format!(
                    "Internal error: missing argument `{}` - validation must have failed",
                    &name,
                );
                quote! {
                    args.get::<#ty>(#name)
                        .or_else(::juniper::FromInputValue::<#scalar>::from_implicit_null)
                        .expect(#err_text)
                }
            }

            Self::Context(_) => quote! {
                ::juniper::FromContext::from(executor.context())
            },

            Self::Executor => quote! { &executor },
        }
    }

    /// Parses an [`OnMethod`] argument from the given Rust method argument
    /// definition.
    ///
    /// Returns [`None`] if parsing fails and emits parsing errors into the
    /// given `scope`.
    pub(crate) fn parse(
        argument: &mut syn::PatType,
        renaming: &RenameRule,
        scope: &GraphQLScope,
    ) -> Option<Self> {
        let orig_attrs = argument.attrs.clone();

        // Remove repeated attributes from the method, to omit incorrect expansion.
        argument.attrs = mem::take(&mut argument.attrs)
            .into_iter()
            .filter(|attr| !path_eq_single(&attr.path, "graphql"))
            .collect();

        let attr = Attr::from_attrs("graphql", &orig_attrs)
            .map_err(|e| proc_macro_error::emit_error!(e))
            .ok()?;

        if attr.context.is_some() {
            return Some(Self::Context(argument.ty.unreferenced().clone()));
        }
        if attr.executor.is_some() {
            return Some(Self::Executor);
        }
        if let syn::Pat::Ident(name) = &*argument.pat {
            let arg = match name.ident.unraw().to_string().as_str() {
                "context" | "ctx" | "_context" | "_ctx" => {
                    Some(Self::Context(argument.ty.unreferenced().clone()))
                }
                "executor" | "_executor" => Some(Self::Executor),
                _ => None,
            };
            if arg.is_some() {
                attr.ensure_no_regular_arguments()
                    .map_err(|e| scope.error(e).emit())
                    .ok()?;
                return arg;
            }
        }

        let name = if let Some(name) = attr.name.as_ref() {
            name.as_ref().value()
        } else if let syn::Pat::Ident(name) = &*argument.pat {
            renaming.apply(&name.ident.unraw().to_string())
        } else {
            scope
                .custom(
                    argument.pat.span(),
                    "method argument should be declared as a single identifier",
                )
                .note(String::from(
                    "use `#[graphql(name = ...)]` attribute to specify custom argument's \
                     name without requiring it being a single identifier",
                ))
                .emit();
            return None;
        };
        if name.starts_with("__") {
            scope.no_double_underscore(
                attr.name
                    .as_ref()
                    .map(SpanContainer::span_ident)
                    .unwrap_or_else(|| argument.pat.span()),
            );
            return None;
        }

        Some(Self::Regular(OnField {
            name,
            ty: argument.ty.as_ref().clone(),
            description: attr.description.as_ref().map(|d| d.as_ref().value()),
            default: attr.default.as_ref().map(|v| v.as_ref().clone()),
        }))
    }
}
