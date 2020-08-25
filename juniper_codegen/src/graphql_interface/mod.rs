//! Code generation for [GraphQL interface][1].
//!
//! [1]: https://spec.graphql.org/June2018/#sec-Interfaces

pub mod attr;

use std::collections::{HashMap, HashSet};

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt as _};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
    token,
};

use crate::util::{
    err, filter_attrs, get_deprecated, get_doc_comment, span_container::SpanContainer,
    OptionExt as _, ParseBufferExt as _,
};

/*
/// Helper alias for the type of [`InterfaceMeta::external_downcasters`] field.
type InterfaceMetaDowncasters = HashMap<syn::Type, SpanContainer<syn::ExprPath>>;*/

/// Available metadata (arguments) behind `#[graphql]` (or `#[graphql_interface]`) attribute placed
/// on a trait definition, when generating code for [GraphQL interface][1] type.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
#[derive(Debug, Default)]
struct InterfaceMeta {
    /// Explicitly specified name of [GraphQL interface][1] type.
    ///
    /// If absent, then Rust type name is used by default.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of [GraphQL interface][1] type.
    ///
    /// If absent, then Rust doc comment is used as [description][2], if any.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    pub description: Option<SpanContainer<String>>,

    /// Explicitly specified type of `juniper::Context` to use for resolving this
    /// [GraphQL interface][1] type with.
    ///
    /// If absent, then unit type `()` is assumed as type of `juniper::Context`.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type of `juniper::ScalarValue` to use for resolving this
    /// [GraphQL interface][1] type with.
    ///
    /// If absent, then generated code will be generic over any `juniper::ScalarValue` type, which,
    /// in turn, requires all [interface][1] implementers to be generic over any
    /// `juniper::ScalarValue` type too. That's why this type should be specified only if one of the
    /// implementers implements `juniper::GraphQLType` in a non-generic way over
    /// `juniper::ScalarValue` type.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub scalar: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified Rust types of [GraphQL objects][2] implementing this
    /// [GraphQL interface][1] type.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    /// [2]: https://spec.graphql.org/June2018/#sec-Objects
    pub implementers: HashSet<SpanContainer<syn::Type>>,

    /*
    /// Explicitly specified external downcasting functions for [GraphQL interface][1] implementers.
    ///
    /// If absent, then macro will try to auto-infer all the possible variants from the type
    /// declaration, if possible. That's why specifying an external resolver function has sense,
    /// when some custom [union][1] variant resolving logic is involved, or variants cannot be
    /// inferred.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub external_downcasters: InterfaceMetaDowncasters,*/
    /// Indicator whether the generated code is intended to be used only inside the `juniper`
    /// library.
    pub is_internal: bool,
}

impl Parse for InterfaceMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut output = Self::default();

        while !input.is_empty() {
            let ident = input.parse_any_ident()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    let name = input.parse::<syn::LitStr>()?;
                    output
                        .name
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(name.span()),
                            name.value(),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "desc" | "description" => {
                    input.parse::<token::Eq>()?;
                    let desc = input.parse::<syn::LitStr>()?;
                    output
                        .description
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(desc.span()),
                            desc.value(),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "ctx" | "context" | "Context" => {
                    input.parse::<token::Eq>()?;
                    let ctx = input.parse::<syn::Type>()?;
                    output
                        .context
                        .replace(SpanContainer::new(ident.span(), Some(ctx.span()), ctx))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "scalar" | "Scalar" | "ScalarValue" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::Type>()?;
                    output
                        .scalar
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "for" | "implementers" => {
                    input.parse::<token::Eq>()?;
                    for impler in input.parse_maybe_wrapped_and_punctuated::<
                        syn::Type, token::Bracket, token::Comma,
                    >()? {
                        let impler_span = impler.span();
                        output
                            .implementers
                            .replace(SpanContainer::new(ident.span(), Some(impler_span), impler))
                            .none_or_else(|_| err::dup_arg(impler_span))?;
                    }
                }
                "internal" => {
                    output.is_internal = true;
                }
                name => {
                    return Err(err::unknown_arg(&ident, name));
                }
            }
            input.try_parse::<token::Comma>()?;
        }

        Ok(output)
    }
}

impl InterfaceMeta {
    /// Tries to merge two [`InterfaceMeta`]s into a single one, reporting about duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            context: try_merge_opt!(context: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            implementers: try_merge_hashset!(implementers: self, another => span_joined),
            is_internal: self.is_internal || another.is_internal,
        })
    }

    /// Parses [`InterfaceMeta`] from the given multiple `name`d [`syn::Attribute`]s placed on a
    /// trait definition.
    pub fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut meta = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if meta.description.is_none() {
            meta.description = get_doc_comment(attrs);
        }

        Ok(meta)
    }
}

#[derive(Debug, Default)]
struct FieldMeta {
    pub name: Option<SpanContainer<syn::LitStr>>,
    pub description: Option<SpanContainer<syn::LitStr>>,
    pub deprecated: Option<SpanContainer<Option<syn::LitStr>>>,
    pub ignore: Option<SpanContainer<syn::Ident>>,
}

impl Parse for FieldMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut output = Self::default();

        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    let name = input.parse::<syn::LitStr>()?;
                    output
                        .name
                        .replace(SpanContainer::new(ident.span(), Some(name.span()), name))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "desc" | "description" => {
                    input.parse::<token::Eq>()?;
                    let desc = input.parse::<syn::LitStr>()?;
                    output
                        .description
                        .replace(SpanContainer::new(ident.span(), Some(desc.span()), desc))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "deprecated" => {
                    let mut reason = None;
                    if input.is_next::<token::Eq>() {
                        input.parse::<token::Eq>()?;
                        reason = Some(input.parse::<syn::LitStr>()?);
                    }
                    output
                        .deprecated
                        .replace(SpanContainer::new(
                            ident.span(),
                            reason.as_ref().map(|r| r.span()),
                            reason,
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "ignore" | "skip" => output
                    .ignore
                    .replace(SpanContainer::new(ident.span(), None, ident.clone()))
                    .none_or_else(|_| err::dup_arg(&ident))?,
                name => {
                    return Err(err::unknown_arg(&ident, name));
                }
            }
            input.try_parse::<token::Comma>()?;
        }

        Ok(output)
    }
}

impl FieldMeta {
    /// Tries to merge two [`FieldMeta`]s into a single one, reporting about duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            deprecated: try_merge_opt!(deprecated: self, another),
            ignore: try_merge_opt!(ignore: self, another),
        })
    }

    /// Parses [`FieldMeta`] from the given multiple `name`d [`syn::Attribute`]s placed on a
    /// function/method definition.
    pub fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut meta = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if meta.description.is_none() {
            meta.description = get_doc_comment(attrs).map(|sc| {
                let span = sc.span_ident();
                sc.map(|desc| syn::LitStr::new(&desc, span))
            });
        }

        if meta.deprecated.is_none() {
            meta.deprecated = get_deprecated(attrs).map(|sc| {
                let span = sc.span_ident();
                sc.map(|depr| depr.reason.map(|rsn| syn::LitStr::new(&rsn, span)))
            });
        }

        Ok(meta)
    }
}

#[derive(Debug, Default)]
struct ArgumentMeta {
    pub name: Option<SpanContainer<syn::LitStr>>,
    pub description: Option<SpanContainer<syn::LitStr>>,
    pub default: Option<SpanContainer<Option<syn::Expr>>>,
    pub context: Option<SpanContainer<syn::Ident>>,
    pub executor: Option<SpanContainer<syn::Ident>>,
}

impl Parse for ArgumentMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut output = Self::default();

        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    let name = input.parse::<syn::LitStr>()?;
                    output
                        .name
                        .replace(SpanContainer::new(ident.span(), Some(name.span()), name))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "desc" | "description" => {
                    input.parse::<token::Eq>()?;
                    let desc = input.parse::<syn::LitStr>()?;
                    output
                        .description
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
                    output
                        .default
                        .replace(SpanContainer::new(
                            ident.span(),
                            expr.as_ref().map(|e| e.span()),
                            expr,
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "ctx" | "context" | "Context" => {
                    output
                        .context
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(ident.span()),
                            ident.clone(),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "exec" | "executor" => {
                    output
                        .executor
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(ident.span()),
                            ident.clone(),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                name => {
                    return Err(err::unknown_arg(&ident, name));
                }
            }
            input.try_parse::<token::Comma>()?;
        }

        Ok(output)
    }
}

impl ArgumentMeta {
    /// Tries to merge two [`ArgumentMeta`]s into a single one, reporting about duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            default: try_merge_opt!(default: self, another),
            context: try_merge_opt!(context: self, another),
            executor: try_merge_opt!(executor: self, another),
        })
    }

    /// Parses [`ArgumentMeta`] from the given multiple `name`d [`syn::Attribute`]s placed on a
    /// function argument.
    pub fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}

struct InterfaceFieldArgumentDefinition {
    pub name: String,
    pub ty: syn::Type,
    pub description: Option<String>,
    pub default: Option<Option<syn::Expr>>,
}

enum InterfaceFieldArgument {
    Regular(InterfaceFieldArgumentDefinition),
    Context,
    Executor,
}

impl InterfaceFieldArgument {
    #[must_use]
    pub fn as_regular(&self) -> Option<&InterfaceFieldArgumentDefinition> {
        if let Self::Regular(arg) = self {
            Some(arg)
        } else {
            None
        }
    }
}

struct InterfaceFieldDefinition {
    pub name: String,
    pub ty: syn::Type,
    pub description: Option<String>,
    pub deprecated: Option<Option<String>>,
    pub method: syn::Ident,
    pub arguments: Vec<InterfaceFieldArgument>,
    pub is_async: bool,
}

/// Definition of [GraphQL interface][1] implementer for code generation.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
struct InterfaceImplementerDefinition {
    /// Rust type that this [GraphQL interface][1] implementer resolves into.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub ty: syn::Type,

    /// Rust code for downcasting into this [GraphQL interface][1] implementer.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub downcast_code: Option<syn::Expr>,

    /// Rust code for checking whether [GraphQL interface][1] should be downcast into this
    /// implementer.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub downcast_check: Option<syn::Expr>,

    /// Rust type of `juniper::Context` that this [GraphQL interface][1] implementer requires for
    /// downcasting.
    ///
    /// It's available only when code generation happens for Rust traits and a trait method contains
    /// context argument.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub context_ty: Option<syn::Type>,

    /// [`Span`] that points to the Rust source code which defines this [GraphQL interface][1]
    /// implementer.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub span: Span,
}

/// Definition of [GraphQL interface][1] for code generation.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
struct InterfaceDefinition {
    /// Name of this [GraphQL interface][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub name: String,

    /// Rust type that this [GraphQL interface][1] is represented with.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub ty: syn::Type,

    /// Generics of the Rust type that this [GraphQL interface][1] is implemented for.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub generics: syn::Generics,

    /// Indicator whether code should be generated for a trait object, rather than for a regular
    /// Rust type.
    pub is_trait_object: bool,

    /// Description of this [GraphQL interface][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub description: Option<String>,

    /// Rust type of `juniper::Context` to generate `juniper::GraphQLType` implementation with
    /// for this [GraphQL interface][1].
    ///
    /// If [`None`] then generated code will use unit type `()` as `juniper::Context`.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub context: Option<syn::Type>,

    /// Rust type of `juniper::ScalarValue` to generate `juniper::GraphQLType` implementation with
    /// for this [GraphQL interface][1].
    ///
    /// If [`None`] then generated code will be generic over any `juniper::ScalarValue` type, which,
    /// in turn, requires all [interface][1] implementers to be generic over any
    /// `juniper::ScalarValue` type too. That's why this type should be specified only if one of the
    /// implementers implements `juniper::GraphQLType` in a non-generic way over
    /// `juniper::ScalarValue` type.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub scalar: Option<syn::Type>,

    pub fields: Vec<InterfaceFieldDefinition>,

    /// Implementers definitions of this [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub implementers: Vec<InterfaceImplementerDefinition>,
}

impl ToTokens for InterfaceDefinition {
    fn to_tokens(&self, into: &mut TokenStream) {
        let name = &self.name;
        let ty = &self.ty;

        let context = self
            .context
            .as_ref()
            .map(|ctx| quote! { #ctx })
            .unwrap_or_else(|| quote! { () });

        let scalar = self
            .scalar
            .as_ref()
            .map(|scl| quote! { #scl })
            .unwrap_or_else(|| quote! { __S });

        let description = self
            .description
            .as_ref()
            .map(|desc| quote! { .description(#desc) });

        let impler_types: Vec<_> = self.implementers.iter().map(|impler| &impler.ty).collect();

        let all_implers_unique = if impler_types.len() > 1 {
            Some(quote! { ::juniper::sa::assert_type_ne_all!(#(#impler_types),*); })
        } else {
            None
        };

        let fields_meta = self.fields.iter().map(|field| {
            let (name, ty) = (&field.name, &field.ty);

            let description = field
                .description
                .as_ref()
                .map(|desc| quote! { .description(#desc) });

            let deprecated = field.deprecated.as_ref().map(|reason| {
                let reason = reason
                    .as_ref()
                    .map(|rsn| quote! { Some(#rsn) })
                    .unwrap_or_else(|| quote! { None });
                quote! { .deprecated(#reason) }
            });

            let arguments = field.arguments.iter().filter_map(|arg| {
                let arg = arg.as_regular()?;

                let (name, ty) = (&arg.name, &arg.ty);

                let description = arg
                    .description
                    .as_ref()
                    .map(|desc| quote! { .description(#desc) });

                let method = if let Some(val) = &arg.default {
                    let val = val
                        .as_ref()
                        .map(|v| quote! { #v })
                        .unwrap_or_else(|| quote! { <#ty as Default>::default() });
                    quote! { .arg_with_default::<#ty>(#name, &(#val), info) }
                } else {
                    quote! { .arg::<#ty>(#name, info) }
                };

                Some(quote! { .argument(registry#method#description) })
            });

            quote! {
                registry.field_convert::<#ty, _, Self::Context>(#name, info)
                    #( #arguments )*
                    #description
                    #deprecated
            }
        });

        let custom_downcast_checks = self.implementers.iter().filter_map(|impler| {
            let impler_check = impler.downcast_check.as_ref()?;
            let impler_ty = &impler.ty;

            Some(quote! {
                if #impler_check {
                    return <#impler_ty as ::juniper::GraphQLType<#scalar>>::name(info)
                        .unwrap().to_string();
                }
            })
        });
        let regular_downcast_check = if self.is_trait_object {
            quote! {
                self.as_dyn_graphql_value().concrete_type_name(context, info)
            }
        } else {
            quote! {
                panic!(
                    "GraphQL interface {} cannot be downcast into any of its implementers in its \
                     current state",
                    #name,
                );
            }
        };

        let custom_downcasts = self.implementers.iter().filter_map(|impler| {
            let downcast_code = impler.downcast_code.as_ref()?;
            let impler_ty = &impler.ty;

            let get_name = quote! {
                (<#impler_ty as ::juniper::GraphQLType<#scalar>>::name(info))
            };
            Some(quote! {
                if type_name == #get_name.unwrap() {
                    return ::juniper::IntoResolvable::into(
                        { #downcast_code },
                        executor.context()
                    )
                    .and_then(|res| match res {
                        Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(info, &r),
                        None => Ok(::juniper::Value::null()),
                    });
                }
            })
        });
        let custom_async_downcasts = self.implementers.iter().filter_map(|impler| {
            let downcast_code = impler.downcast_code.as_ref()?;
            let impler_ty = &impler.ty;

            let get_name = quote! {
                (<#impler_ty as ::juniper::GraphQLType<#scalar>>::name(info))
            };
            Some(quote! {
                if type_name == #get_name.unwrap() {
                    let res = ::juniper::IntoResolvable::into(
                        { #downcast_code },
                        executor.context()
                    );
                    return ::juniper::futures::future::FutureExt::boxed(async move {
                        match res? {
                            Some((ctx, r)) => {
                                let subexec = executor.replaced_context(ctx);
                                subexec.resolve_with_ctx_async(info, &r).await
                            },
                            None => Ok(::juniper::Value::null()),
                        }
                    });
                }
            })
        });
        let (regular_downcast, regular_async_downcast) = if self.is_trait_object {
            let sync = quote! {
                return ::juniper::IntoResolvable::into(
                    self.as_dyn_graphql_value(),
                    executor.context(),
                )
                .and_then(|res| match res {
                    Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(info, &r),
                    None => Ok(::juniper::Value::null()),
                })
            };
            let r#async = quote! {
                let res = ::juniper::IntoResolvable::into(
                    self.as_dyn_graphql_value_async(),
                    executor.context(),
                );
                return ::juniper::futures::future::FutureExt::boxed(async move {
                    match res? {
                        Some((ctx, r)) => {
                            let subexec = executor.replaced_context(ctx);
                            subexec.resolve_with_ctx_async(info, &r).await
                        },
                        None => Ok(::juniper::Value::null()),
                    }
                });
            };
            (sync, r#async)
        } else {
            let panic = quote! {
                panic!(
                    "Concrete type {} cannot be downcast from on GraphQL interface {}",
                    type_name, #name,
                );
            };
            (panic.clone(), panic)
        };

        let (_, ty_generics, _) = self.generics.split_for_impl();

        let mut ext_generics = self.generics.clone();
        if self.is_trait_object {
            ext_generics.params.push(parse_quote! { '__obj });
        }
        if self.scalar.is_none() {
            ext_generics.params.push(parse_quote! { #scalar });
            ext_generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { #scalar: ::juniper::ScalarValue });
        }
        let (ext_impl_generics, _, where_clause) = ext_generics.split_for_impl();

        let mut where_async = where_clause
            .cloned()
            .unwrap_or_else(|| parse_quote! { where });
        where_async.predicates.push(parse_quote! { Self: Sync });
        if self.scalar.is_none() {
            where_async
                .predicates
                .push(parse_quote! { #scalar: Send + Sync });
        }

        let mut ty_full = quote! { #ty#ty_generics };
        let mut ty_interface = quote! { #ty#ty_generics };
        if self.is_trait_object {
            let mut ty_params = None;
            if !self.generics.params.is_empty() {
                let params = &self.generics.params;
                ty_params = Some(quote! { #params, });
            };
            ty_full = quote! {
                dyn #ty<#ty_params #scalar, Context = #context, TypeInfo = ()> +
                    '__obj + Send + Sync
            };
            ty_interface = quote! { #ty<#ty_params #scalar> };
        }

        let fields_sync_resolvers = self.fields.iter().filter_map(|field| {
            if field.is_async {
                return None;
            }
            let (name, ty, method) = (&field.name, &field.ty, &field.method);
            let arguments = field.arguments.iter().map(|arg| match arg {
                InterfaceFieldArgument::Regular(arg) => {
                    let (name, ty) = (&arg.name, &arg.ty);
                    let err_text = format!(
                        "Internal error: missing argument `{}` - validation must have failed",
                        &name,
                    );
                    quote! { args.get::<#ty>(#name).expect(#err_text) }
                }
                InterfaceFieldArgument::Context => quote! { executor.context() },
                InterfaceFieldArgument::Executor => quote! { &executor },
            });

            Some(quote! {
                #name => {
                    let res: #ty = <Self as #ty_interface>::#method(self#( , #arguments )*);
                    ::juniper::IntoResolvable::into(res, executor.context())
                        .and_then(|res| match res {
                            Some((ctx, r)) => executor
                                .replaced_context(ctx)
                                .resolve_with_ctx(info, &r),
                            None => Ok(::juniper::Value::null()),
                        })
                },
            })
        });
        let fields_sync_panic = {
            let names = self
                .fields
                .iter()
                .filter_map(|field| {
                    if field.is_async {
                        Some(&field.name)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if names.is_empty() {
                None
            } else {
                Some(quote! {
                    #( #names )|* => panic!(
                        "Tried to resolve async field `{}` on type `{}` with a sync resolver",
                        field,
                        <Self as ::juniper::GraphQLType<#scalar>>::name(info).unwrap(),
                    ),
                })
            }
        };

        let fields_async_resolvers = self.fields.iter().map(|field| {
            let (name, ty) = (&field.name, &field.ty);

            let method = &field.method;
            let arguments = field.arguments.iter().map(|arg| match arg {
                InterfaceFieldArgument::Regular(arg) => {
                    let (name, ty) = (&arg.name, &arg.ty);
                    let err_text = format!(
                        "Internal error: missing argument `{}` - validation must have failed",
                        &name,
                    );
                    quote! { args.get::<#ty>(#name).expect(#err_text) }
                }
                InterfaceFieldArgument::Context => quote! { executor.context() },
                InterfaceFieldArgument::Executor => quote! { &executor },
            });

            let mut fut = quote! { <Self as #ty_interface>::#method(self#( , #arguments )*) };
            if !field.is_async {
                fut = quote! { ::juniper::futures::future::ready(#fut) };
            }

            quote! {
                #name => Box::pin(::juniper::futures::FutureExt::then(#fut, move |res: #ty| async move {
                    match ::juniper::IntoResolvable::into(res, executor.context())? {
                        Some((ctx, r)) => {
                            let subexec = executor.replaced_context(ctx);
                            subexec.resolve_with_ctx_async(info, &r).await
                        },
                        None => Ok(::juniper::Value::null()),
                    }
                })),
            }
        });

        let type_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics ::juniper::GraphQLType<#scalar> for #ty_full
                #where_clause
            {
                fn name(_ : &Self::TypeInfo) -> Option<&'static str> {
                    Some(#name)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut ::juniper::Registry<'r, #scalar>
                ) -> ::juniper::meta::MetaType<'r, #scalar>
                where #scalar: 'r,
                {
                    // Ensure all implementer types are registered.
                    #( let _ = registry.get_type::<#impler_types>(info); )*

                    let fields = [
                        #( #fields_meta, )*
                    ];
                    registry.build_interface_type::<#ty_full>(info, &fields)
                        #description
                        .into_meta()
                }
            }
        };

        let value_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics ::juniper::GraphQLValue<#scalar> for #ty_full
                #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn type_name<'__i>(&self, info: &'__i Self::TypeInfo) -> Option<&'__i str> {
                    <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                }

                fn resolve_field(
                    &self,
                    info: &Self::TypeInfo,
                    field: &str,
                    args: &::juniper::Arguments<#scalar>,
                    executor: &::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    match field {
                        #( #fields_sync_resolvers )*
                        #fields_sync_panic
                        _ => panic!(
                            "Field `{}` not found on type `{}`",
                            field,
                            <Self as ::juniper::GraphQLType<#scalar>>::name(info).unwrap(),
                        ),
                    }
                }

                fn concrete_type_name(
                    &self,
                    context: &Self::Context,
                    info: &Self::TypeInfo,
                ) -> String {
                    #( #custom_downcast_checks )*
                    #regular_downcast_check
                }

                fn resolve_into_type(
                    &self,
                    info: &Self::TypeInfo,
                    type_name: &str,
                    _: Option<&[::juniper::Selection<#scalar>]>,
                    executor: &::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    #( #custom_downcasts )*
                    #regular_downcast
                }
            }
        };

        let value_async_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics ::juniper::GraphQLValueAsync<#scalar> for #ty_full
                #where_async
            {
                fn resolve_field_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    field: &'b str,
                    args: &'b ::juniper::Arguments<#scalar>,
                    executor: &'b ::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    match field {
                        #( #fields_async_resolvers )*
                        _ => panic!(
                            "Field `{}` not found on type `{}`",
                            field,
                            <Self as ::juniper::GraphQLType<#scalar>>::name(info).unwrap(),
                        ),
                    }
                }

                fn resolve_into_type_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    type_name: &str,
                    _: Option<&'b [::juniper::Selection<'b, #scalar>]>,
                    executor: &'b ::juniper::Executor<'b, 'b, Self::Context, #scalar>
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    #( #custom_async_downcasts )*
                    #regular_async_downcast
                }
            }
        };

        let output_type_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics ::juniper::marker::IsOutputType<#scalar> for #ty_full
                #where_clause
            {
                fn mark() {
                    #( <#impler_types as ::juniper::marker::GraphQLObjectType<#scalar>>::mark(); )*
                }
            }
        };

        let interface_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics ::juniper::marker::GraphQLInterface<#scalar> for #ty_full
                #where_clause
            {
                fn mark() {
                    #all_implers_unique

                    #( <#impler_types as ::juniper::marker::GraphQLObjectType<#scalar>>::mark(); )*
                }
            }
        };

        into.append_all(&[
            interface_impl,
            output_type_impl,
            type_impl,
            value_impl,
            value_async_impl,
        ]);
    }
}
