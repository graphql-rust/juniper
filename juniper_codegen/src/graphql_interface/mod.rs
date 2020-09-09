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

use crate::{
    common::{
        parse::{
            attr::{err, OptionExt as _},
            GenericsExt as _, ParseBufferExt as _,
        },
        ScalarValueType,
    },
    util::{filter_attrs, get_deprecated, get_doc_comment, span_container::SpanContainer},
};

/// Helper alias for the type of [`InterfaceMeta::external_downcasts`] field.
type InterfaceMetaDowncasts = HashMap<syn::Type, SpanContainer<syn::ExprPath>>;

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

    pub alias: Option<SpanContainer<syn::Ident>>,

    pub asyncness: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified external downcasting functions for [GraphQL interface][1] implementers.
    ///
    /// If absent, then macro will try to auto-infer all the possible variants from the type
    /// declaration, if possible. That's why specifying an external resolver function has sense,
    /// when some custom [union][1] variant resolving logic is involved, or variants cannot be
    /// inferred.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub external_downcasts: InterfaceMetaDowncasts,

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
                "dyn" => {
                    input.parse::<token::Eq>()?;
                    let alias = input.parse::<syn::Ident>()?;
                    output
                        .alias
                        .replace(SpanContainer::new(ident.span(), Some(alias.span()), alias))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "async" => {
                    let span = ident.span();
                    output
                        .asyncness
                        .replace(SpanContainer::new(span, Some(span), ident))
                        .none_or_else(|_| err::dup_arg(span))?;
                }
                "on" => {
                    let ty = input.parse::<syn::Type>()?;
                    input.parse::<token::Eq>()?;
                    let dwncst = input.parse::<syn::ExprPath>()?;
                    let dwncst_spanned = SpanContainer::new(ident.span(), Some(ty.span()), dwncst);
                    let dwncst_span = dwncst_spanned.span_joined();
                    output
                        .external_downcasts
                        .insert(ty, dwncst_spanned)
                        .none_or_else(|_| err::dup_arg(dwncst_span))?
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
            alias: try_merge_opt!(alias: self, another),
            asyncness: try_merge_opt!(asyncness: self, another),
            external_downcasts: try_merge_hashmap!(
                external_downcasts: self, another => span_joined
            ),
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

/// Available metadata (arguments) behind `#[graphql_interface]` attribute placed on a trait
/// implementation block, when generating code for [GraphQL interface][1] type.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
#[derive(Debug, Default)]
struct ImplementerMeta {
    pub scalar: Option<SpanContainer<syn::Type>>,
    pub asyncness: Option<SpanContainer<syn::Ident>>,
}

impl Parse for ImplementerMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut output = Self::default();

        while !input.is_empty() {
            let ident = input.parse_any_ident()?;
            match ident.to_string().as_str() {
                "scalar" | "Scalar" | "ScalarValue" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::Type>()?;
                    output
                        .scalar
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "async" => {
                    let span = ident.span();
                    output
                        .asyncness
                        .replace(SpanContainer::new(span, Some(span), ident))
                        .none_or_else(|_| err::dup_arg(span))?;
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

impl ImplementerMeta {
    /// Tries to merge two [`ImplementerMeta`]s into a single one, reporting about duplicates, if
    /// any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            scalar: try_merge_opt!(scalar: self, another),
            asyncness: try_merge_opt!(asyncness: self, another),
        })
    }

    /// Parses [`ImplementerMeta`] from the given multiple `name`d [`syn::Attribute`]s placed on a
    /// trait implementation block.
    pub fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}

#[derive(Debug, Default)]
struct TraitMethodMeta {
    pub name: Option<SpanContainer<syn::LitStr>>,
    pub description: Option<SpanContainer<syn::LitStr>>,
    pub deprecated: Option<SpanContainer<Option<syn::LitStr>>>,
    pub ignore: Option<SpanContainer<syn::Ident>>,
    pub downcast: Option<SpanContainer<syn::Ident>>,
}

impl Parse for TraitMethodMeta {
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
                "downcast" => output
                    .downcast
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

impl TraitMethodMeta {
    /// Tries to merge two [`FieldMeta`]s into a single one, reporting about duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            deprecated: try_merge_opt!(deprecated: self, another),
            ignore: try_merge_opt!(ignore: self, another),
            downcast: try_merge_opt!(downcast: self, another),
        })
    }

    /// Parses [`FieldMeta`] from the given multiple `name`d [`syn::Attribute`]s placed on a
    /// function/method definition.
    pub fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut meta = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if let Some(ignore) = &meta.ignore {
            if meta.name.is_some()
                || meta.description.is_some()
                || meta.deprecated.is_some()
                || meta.downcast.is_some()
            {
                return Err(syn::Error::new(
                    ignore.span(),
                    "`ignore` attribute argument is not composable with any other arguments",
                ));
            }
        }

        if let Some(downcast) = &meta.downcast {
            if meta.name.is_some()
                || meta.description.is_some()
                || meta.deprecated.is_some()
                || meta.ignore.is_some()
            {
                return Err(syn::Error::new(
                    downcast.span(),
                    "`downcast` attribute argument is not composable with any other arguments",
                ));
            }
        }

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
                    let span = ident.span();
                    output
                        .context
                        .replace(SpanContainer::new(span, Some(span), ident))
                        .none_or_else(|_| err::dup_arg(span))?
                }
                "exec" | "executor" => {
                    let span = ident.span();
                    output
                        .executor
                        .replace(SpanContainer::new(span, Some(span), ident))
                        .none_or_else(|_| err::dup_arg(span))?
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
        let meta = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if let Some(context) = &meta.context {
            if meta.name.is_some()
                || meta.description.is_some()
                || meta.default.is_some()
                || meta.executor.is_some()
            {
                return Err(syn::Error::new(
                    context.span(),
                    "`context` attribute argument is not composable with any other arguments",
                ));
            }
        }

        if let Some(executor) = &meta.executor {
            if meta.name.is_some()
                || meta.description.is_some()
                || meta.default.is_some()
                || meta.context.is_some()
            {
                return Err(syn::Error::new(
                    executor.span(),
                    "`executor` attribute argument is not composable with any other arguments",
                ));
            }
        }

        Ok(meta)
    }
}

struct InterfaceFieldArgumentDefinition {
    pub name: String,
    pub ty: syn::Type,
    pub description: Option<String>,
    pub default: Option<Option<syn::Expr>>,
}

enum MethodArgument {
    Regular(InterfaceFieldArgumentDefinition),
    Context(syn::Type),
    Executor,
}

impl MethodArgument {
    #[must_use]
    pub fn as_regular(&self) -> Option<&InterfaceFieldArgumentDefinition> {
        if let Self::Regular(arg) = self {
            Some(arg)
        } else {
            None
        }
    }

    #[must_use]
    fn context_ty(&self) -> Option<&syn::Type> {
        if let Self::Context(ty) = self {
            Some(ty)
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
    pub arguments: Vec<MethodArgument>,
    pub is_async: bool,
}

enum ImplementerDowncastDefinition {
    Method {
        name: syn::Ident,
        with_context: bool,
    },
    External {
        path: syn::ExprPath,
    },
}

/// Definition of [GraphQL interface][1] implementer for code generation.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
struct ImplementerDefinition {
    /// Rust type that this [GraphQL interface][1] implementer resolves into.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub ty: syn::Type,

    pub downcast: Option<ImplementerDowncastDefinition>,

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
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
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

    pub trait_object: Option<Option<syn::Ident>>,

    pub visibility: syn::Visibility,

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
    pub scalar: ScalarValueType,

    pub fields: Vec<InterfaceFieldDefinition>,

    /// Implementers definitions of this [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub implementers: Vec<ImplementerDefinition>,
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

        let scalar = self.scalar.as_tokens().unwrap_or_else(|| quote! { __S });

        let description = self
            .description
            .as_ref()
            .map(|desc| quote! { .description(#desc) });

        let mut impler_types: Vec<_> = self.implementers.iter().map(|impler| &impler.ty).collect();
        impler_types.sort_unstable_by(|a, b| {
            let (a, b) = (quote! { #a }.to_string(), quote! { #b }.to_string());
            a.cmp(&b)
        });

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
                        .map(|v| quote! { (#v).into() })
                        .unwrap_or_else(|| quote! { <#ty as Default>::default() });
                    quote! { .arg_with_default::<#ty>(#name, &#val, info) }
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

        let fields_marks = self.fields.iter().map(|field| {
            let arguments_marks = field.arguments.iter().filter_map(|arg| {
                let arg_ty = &arg.as_regular()?.ty;
                Some(quote! { <#arg_ty as ::juniper::marker::IsInputType<#scalar>>::mark(); })
            });

            let field_ty = &field.ty;
            let resolved_ty = quote! {
                <#field_ty as ::juniper::IntoResolvable<
                    '_, #scalar, _, <Self as ::juniper::GraphQLValue<#scalar>>::Context,
                >>::Type
            };

            quote! {
                #( #arguments_marks )*
                <#resolved_ty as ::juniper::marker::IsOutputType<#scalar>>::mark();
            }
        });

        let custom_downcast_checks = self.implementers.iter().filter_map(|impler| {
            let impler_ty = &impler.ty;

            let mut ctx_arg = Some(quote! { , ::juniper::FromContext::from(context) });
            let fn_path = match impler.downcast.as_ref()? {
                ImplementerDowncastDefinition::Method { name, with_context } => {
                    if !with_context {
                        ctx_arg = None;
                    }
                    quote! { #ty::#name }
                }
                ImplementerDowncastDefinition::External { path } => {
                    quote! { #path }
                }
            };

            // Doing this may be quite an expensive, because resolving may contain some heavy
            // computation, so we're preforming it twice. Unfortunately, we have no other options
            // here, until the `juniper::GraphQLType` itself will allow to do it in some cleverer
            // way.
            Some(quote! {
                if ({ #fn_path(self #ctx_arg) } as ::std::option::Option<&#impler_ty>).is_some() {
                    return <#impler_ty as ::juniper::GraphQLType<#scalar>>::name(info)
                        .unwrap()
                        .to_string();
                }
            })
        });
        let regular_downcast_check = if self.trait_object.is_some() {
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
            let impler_ty = &impler.ty;

            let mut ctx_arg = Some(quote! { , ::juniper::FromContext::from(context) });
            let fn_path = match impler.downcast.as_ref()? {
                ImplementerDowncastDefinition::Method { name, with_context } => {
                    if !with_context {
                        ctx_arg = None;
                    }
                    quote! { #ty::#name }
                }
                ImplementerDowncastDefinition::External { path } => {
                    quote! { #path }
                }
            };

            Some(quote! {
                if type_name == <
                    #impler_ty as ::juniper::GraphQLType<#scalar>
                >::name(info).unwrap() {
                    return ::juniper::IntoResolvable::into({ #fn_path(self #ctx_arg) }, context)
                        .and_then(|res| match res {
                            Some((ctx, r)) => executor
                                .replaced_context(ctx)
                                .resolve_with_ctx(info, &r),
                            None => Ok(::juniper::Value::null()),
                        });
                }
            })
        });
        let custom_async_downcasts = self.implementers.iter().filter_map(|impler| {
            let impler_ty = &impler.ty;

            let mut ctx_arg = Some(quote! { , ::juniper::FromContext::from(context) });
            let fn_path = match impler.downcast.as_ref()? {
                ImplementerDowncastDefinition::Method { name, with_context } => {
                    if !with_context {
                        ctx_arg = None;
                    }
                    quote! { #ty::#name }
                }
                ImplementerDowncastDefinition::External { path } => {
                    quote! { #path }
                }
            };

            Some(quote! {
                if type_name == <
                    #impler_ty as ::juniper::GraphQLType<#scalar>
                >::name(info).unwrap() {
                    let res = ::juniper::IntoResolvable::into({ #fn_path(self #ctx_arg) }, context);
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
        let (regular_downcast, regular_async_downcast) = if self.trait_object.is_some() {
            let sync = quote! {
                return ::juniper::IntoResolvable::into(self.as_dyn_graphql_value(), context)
                    .and_then(|res| match res {
                        Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(info, &r),
                        None => Ok(::juniper::Value::null()),
                    })
            };
            let r#async = quote! {
                let res = ::juniper::IntoResolvable::into(
                    self.as_dyn_graphql_value_async(), context,
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

        let mut generics = self.generics.clone();
        if self.trait_object.is_some() {
            generics.remove_defaults();
            generics.move_bounds_to_where_clause();
        }
        let (_, ty_generics, _) = generics.split_for_impl();

        let mut ext_generics = generics.clone();
        if self.trait_object.is_some() {
            ext_generics.params.push(parse_quote! { '__obj });
        }
        if self.scalar.is_implicit_generic() {
            ext_generics.params.push(parse_quote! { #scalar });
        }
        if self.scalar.is_generic() {
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
        if self.scalar.is_generic() {
            where_async
                .predicates
                .push(parse_quote! { #scalar: Send + Sync });
        }

        let mut ty_full = quote! { #ty#ty_generics };
        let mut ty_interface = ty_full.clone();
        if self.trait_object.is_some() {
            let mut ty_params = None;
            if !generics.params.is_empty() {
                let params = &generics.params;
                ty_params = Some(quote! { #params, });
            };

            let scalar = if self.scalar.is_explicit_generic() {
                None
            } else {
                Some(&scalar)
            };
            ty_interface = quote! { #ty<#ty_params #scalar> };

            let scalar = scalar.map(|sc| quote! { #sc, });
            ty_full = quote! {
                dyn #ty<#ty_params #scalar Context = #context, TypeInfo = ()> + '__obj + Send + Sync
            };
        }

        let mut dyn_alias = quote! {};
        if let Some(Some(alias)) = self.trait_object.as_ref().as_ref() {
            let doc = format!(
                "Helper alias for the `{}` [trait object][2] implementing [GraphQL interface][1].\
                 \n\n\
                 [1]: https://spec.graphql.org/June2018/#sec-Interfaces\n\
                 [2]: https://doc.rust-lang.org/reference/types/trait-object.html",
                quote! { #ty },
            );

            let (mut ty_params_left, mut ty_params_right) = (None, None);
            if !generics.params.is_empty() {
                let params = &generics.params;
                ty_params_right = Some(quote! { #params, });

                // We should preserve defaults for left side.
                let mut generics = self.generics.clone();
                generics.move_bounds_to_where_clause();
                let params = &generics.params;
                ty_params_left = Some(quote! { , #params });
            };

            let (mut scalar_left, mut scalar_right) = (None, None);
            if !self.scalar.is_explicit_generic() {
                let default_scalar = self.scalar.default_scalar();
                scalar_left = Some(quote! { , S = #default_scalar });
                scalar_right = Some(quote! { S, });
            }

            let vis = &self.visibility;

            dyn_alias = quote! {
                #[allow(unused_qualifications)]
                #[doc = #doc]
                #vis type #alias<'a #ty_params_left #scalar_left> =
                    dyn #ty<#ty_params_right #scalar_right Context = #context, TypeInfo = ()> +
                        'a + Send + Sync;
            }
        }

        let fields_sync_resolvers = self.fields.iter().filter_map(|field| {
            if field.is_async {
                return None;
            }
            let (name, ty, method) = (&field.name, &field.ty, &field.method);
            let arguments = field.arguments.iter().map(|arg| match arg {
                MethodArgument::Regular(arg) => {
                    let (name, ty) = (&arg.name, &arg.ty);
                    let err_text = format!(
                        "Internal error: missing argument `{}` - validation must have failed",
                        &name,
                    );
                    quote! { args.get::<#ty>(#name).expect(#err_text) }
                }
                MethodArgument::Context(_) => quote! {
                    ::juniper::FromContext::from(executor.context())
                },
                MethodArgument::Executor => quote! { &executor },
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
                MethodArgument::Regular(arg) => {
                    let (name, ty) = (&arg.name, &arg.ty);
                    let err_text = format!(
                        "Internal error: missing argument `{}` - validation must have failed",
                        &name,
                    );
                    quote! { args.get::<#ty>(#name).expect(#err_text) }
                }
                MethodArgument::Context(_) => quote! {
                    ::juniper::FromContext::from(executor.context())
                },
                MethodArgument::Executor => quote! { &executor },
            });

            let mut fut = quote! { <Self as #ty_interface>::#method(self#( , #arguments )*) };
            if !field.is_async {
                fut = quote! { ::juniper::futures::future::ready(#fut) };
            }

            quote! {
                #name => Box::pin(::juniper::futures::FutureExt::then(#fut, move |res: #ty| {
                    async move {
                        match ::juniper::IntoResolvable::into(res, executor.context())? {
                            Some((ctx, r)) => {
                                let subexec = executor.replaced_context(ctx);
                                subexec.resolve_with_ctx_async(info, &r).await
                            },
                            None => Ok(::juniper::Value::null()),
                        }
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
                    let context = executor.context();
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
                    let context = executor.context();
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
                    #( #fields_marks )*
                    #( <#impler_types as ::juniper::marker::IsOutputType<#scalar>>::mark(); )*
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
            dyn_alias,
            interface_impl,
            output_type_impl,
            type_impl,
            value_impl,
            value_async_impl,
        ]);
    }
}
