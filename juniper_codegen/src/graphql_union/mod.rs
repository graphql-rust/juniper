//! Code generation for [GraphQL union][1].
//!
//! [1]: https://spec.graphql.org/June2018/#sec-Unions

pub mod attr;
pub mod derive;

use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt as _};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
};

use crate::util::{
    filter_attrs, get_doc_comment, span_container::SpanContainer, Mode, OptionExt as _,
};

/// Attempts to merge an [`Option`]ed `$field` of a `$self` struct with the same `$field` of
/// `$another` struct. If both are [`Some`], then throws a duplication error with a [`Span`] related
/// to the `$another` struct (a later one).
///
/// The type of [`Span`] may be explicitly specified as one of the [`SpanContainer`] methods.
/// By default, [`SpanContainer::span_ident`] is used.
macro_rules! try_merge_opt {
    ($field:ident: $self:ident, $another:ident => $span:ident) => {{
        if let Some(v) = $self.$field {
            $another
                .$field
                .replace(v)
                .none_or_else(|dup| dup_attr_err(dup.$span()))?;
        }
        $another.$field
    }};

    ($field:ident: $self:ident, $another:ident) => {
        try_merge_opt!($field: $self, $another => span_ident)
    };
}

/// Attempts to merge a [`HashMap`]ed `$field` of a `$self` struct with the same `$field` of
/// `$another` struct. If some [`HashMap`] entries are duplicated, then throws a duplication error
/// with a [`Span`] related to the `$another` struct (a later one).
///
/// The type of [`Span`] may be explicitly specified as one of the [`SpanContainer`] methods.
/// By default, [`SpanContainer::span_ident`] is used.
macro_rules! try_merge_hashmap {
    ($field:ident: $self:ident, $another:ident => $span:ident) => {{
        if !$self.$field.is_empty() {
            for (ty, rslvr) in $self.$field {
                $another
                    .$field
                    .insert(ty, rslvr)
                    .none_or_else(|dup| dup_attr_err(dup.$span()))?;
            }
        }
        $another.$field
    }};

    ($field:ident: $self:ident, $another:ident) => {
        try_merge_hashmap!($field: $self, $another => span_ident)
    };
}

/// Creates and returns duplication error pointing to the given `span`.
fn dup_attr_err(span: Span) -> syn::Error {
    syn::Error::new(span, "duplicated attribute")
}

/// Helper alias for the type of [`UnionMeta::external_resolvers`] field.
type UnionMetaResolvers = HashMap<syn::Type, SpanContainer<syn::ExprPath>>;

/// Available metadata (arguments) behind `#[graphql]` (or `#[graphql_union]`) attribute when
/// generating code for [GraphQL union][1] type.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
#[derive(Debug, Default)]
struct UnionMeta {
    /// Explicitly specified name of [GraphQL union][1] type.
    ///
    /// If absent, then Rust type name is used by default.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of [GraphQL union][1] type.
    ///
    /// If absent, then Rust doc comment is used as [description][2], if any.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    pub description: Option<SpanContainer<String>>,

    /// Explicitly specified type of `juniper::Context` to use for resolving this [GraphQL union][1]
    /// type with.
    ///
    /// If absent, then unit type `()` is assumed as type of `juniper::Context`.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type of `juniper::ScalarValue` to use for resolving this
    /// [GraphQL union][1] type with.
    ///
    /// If absent, then generated code will be generic over any `juniper::ScalarValue` type, which,
    /// in turn, requires all [union][1] variants to be generic over any `juniper::ScalarValue` type
    /// too. That's why this type should be specified only if one of the variants implements
    /// `juniper::GraphQLType` in a non-generic way over `juniper::ScalarValue` type.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub scalar: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified external resolver functions for [GraphQL union][1] variants.
    ///
    /// If absent, then macro will try to auto-infer all the possible variants from the type
    /// declaration, if possible. That's why specifying an external resolver function has sense,
    /// when some custom [union][1] variant resolving logic is involved, or variants cannot be
    /// inferred.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub external_resolvers: UnionMetaResolvers,
}

impl Parse for UnionMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut output = Self::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<syn::Token![=]>()?;
                    let name = input.parse::<syn::LitStr>()?;
                    output
                        .name
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(name.span()),
                            name.value(),
                        ))
                        .none_or_else(|_| dup_attr_err(ident.span()))?
                }
                "desc" | "description" => {
                    input.parse::<syn::Token![=]>()?;
                    let desc = input.parse::<syn::LitStr>()?;
                    output
                        .description
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(desc.span()),
                            desc.value(),
                        ))
                        .none_or_else(|_| dup_attr_err(ident.span()))?
                }
                "ctx" | "context" | "Context" => {
                    input.parse::<syn::Token![=]>()?;
                    let ctx = input.parse::<syn::Type>()?;
                    output
                        .context
                        .replace(SpanContainer::new(ident.span(), Some(ctx.span()), ctx))
                        .none_or_else(|_| dup_attr_err(ident.span()))?
                }
                "scalar" | "Scalar" | "ScalarValue" => {
                    input.parse::<syn::Token![=]>()?;
                    let scl = input.parse::<syn::Type>()?;
                    output
                        .scalar
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| dup_attr_err(ident.span()))?
                }
                "on" => {
                    let ty = input.parse::<syn::Type>()?;
                    input.parse::<syn::Token![=]>()?;
                    let rslvr = input.parse::<syn::ExprPath>()?;
                    let rslvr_spanned = SpanContainer::new(ident.span(), Some(ty.span()), rslvr);
                    let rslvr_span = rslvr_spanned.span_joined();
                    output
                        .external_resolvers
                        .insert(ty, rslvr_spanned)
                        .none_or_else(|_| dup_attr_err(rslvr_span))?
                }
                _ => {
                    return Err(syn::Error::new(ident.span(), "unknown attribute"));
                }
            }
            if input.lookahead1().peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(output)
    }
}

impl UnionMeta {
    /// Tries to merge two [`UnionMeta`]s into single one, reporting about duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            context: try_merge_opt!(context: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            external_resolvers: try_merge_hashmap!(external_resolvers: self, another => span_joined),
        })
    }

    /// Parses [`UnionMeta`] from the given multiple `name`d attributes placed on type definition.
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

/// Available metadata (arguments) behind `#[graphql]` (or `#[graphql_union]`) attribute when
/// generating code for [GraphQL union][1]'s variant.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
#[derive(Debug, Default)]
struct UnionVariantMeta {
    /// Explicitly specified marker for the variant/field being ignored and not included into
    /// [GraphQL union][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub ignore: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified external resolver function for this [GraphQL union][1] variant.
    ///
    /// If absent, then macro will generate the code which just returns the variant inner value.
    /// Usually, specifying an external resolver function has sense, when some custom resolving
    /// logic is involved.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub external_resolver: Option<SpanContainer<syn::ExprPath>>,
}

impl Parse for UnionVariantMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut output = Self::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                "ignore" | "skip" => output
                    .ignore
                    .replace(SpanContainer::new(ident.span(), None, ident.clone()))
                    .none_or_else(|_| dup_attr_err(ident.span()))?,
                "with" => {
                    input.parse::<syn::Token![=]>()?;
                    let rslvr = input.parse::<syn::ExprPath>()?;
                    output
                        .external_resolver
                        .replace(SpanContainer::new(ident.span(), Some(rslvr.span()), rslvr))
                        .none_or_else(|_| dup_attr_err(ident.span()))?
                }
                _ => {
                    return Err(syn::Error::new(ident.span(), "unknown attribute"));
                }
            }
            if input.lookahead1().peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(output)
    }
}

impl UnionVariantMeta {
    /// Tries to merge two [`UnionVariantMeta`]s into single one, reporting about duplicates, if
    /// any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            ignore: try_merge_opt!(ignore: self, another),
            external_resolver: try_merge_opt!(external_resolver: self, another),
        })
    }

    /// Parses [`UnionVariantMeta`] from the given multiple `name`d attributes placed on
    /// variant/field/method definition.
    pub fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}

/// Definition of [GraphQL union][1] variant for code generation.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
struct UnionVariantDefinition {
    /// Rust type that this [GraphQL union][1] variant resolves into.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub ty: syn::Type,

    /// Rust code for value resolution of this [GraphQL union][1] variant.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub resolver_code: syn::Expr,

    /// Rust code for checking whether [GraphQL union][1] should be resolved into this variant.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub resolver_check: syn::Expr,

    /// Rust enum variant path that this [GraphQL union][1] variant is associated with.
    ///
    /// It's available only when code generation happens for Rust enums.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub enum_path: Option<TokenStream>,

    /// Rust type of `juniper::Context` that this [GraphQL union][1] variant requires for
    /// resolution.
    ///
    /// It's available only when code generation happens for Rust traits and a trait method contains
    /// context argument.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub context_ty: Option<syn::Type>,

    /// [`Span`] that points to the Rust source code which defines this [GraphQL union][1] variant.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub span: Span,
}

/// Definition of [GraphQL union][1] for code generation.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
struct UnionDefinition {
    /// Name of this [GraphQL union][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub name: String,

    /// Rust type that this [GraphQL union][1] is represented with.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub ty: syn::Type,

    /// Generics of the Rust type that this [GraphQL union][1] is implemented for.
    pub generics: syn::Generics,

    /// Indicator whether code should be generated for a trait object, rather than for a regular
    /// Rust type.
    pub is_trait_object: bool,

    /// Description of this [GraphQL union][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub description: Option<String>,

    /// Rust type of `juniper::Context` to generate `juniper::GraphQLType` implementation with
    /// for this [GraphQL union][1].
    ///
    /// If [`None`] then generated code will use unit type `()` as `juniper::Context`.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub context: Option<syn::Type>,

    /// Rust type of `juniper::ScalarValue` to generate `juniper::GraphQLType` implementation with
    /// for this [GraphQL union][1].
    ///
    /// If [`None`] then generated code will be generic over any `juniper::ScalarValue` type, which,
    /// in turn, requires all [union][1] variants to be generic over any `juniper::ScalarValue` type
    /// too. That's why this type should be specified only if one of the variants implements
    /// `juniper::GraphQLType` in a non-generic way over `juniper::ScalarValue` type.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub scalar: Option<syn::Type>,

    /// Variants definitions of this [GraphQL union][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub variants: Vec<UnionVariantDefinition>,

    /// [`Span`] that points to the Rust source code which defines this [GraphQL union][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub span: Span,

    /// [`Mode`] to generate code in for this [GraphQL union][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub mode: Mode,
}

impl ToTokens for UnionDefinition {
    fn to_tokens(&self, into: &mut TokenStream) {
        let crate_path = self.mode.crate_path();

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
        let default_scalar = self
            .scalar
            .as_ref()
            .map(|scl| quote! { #scl })
            .unwrap_or_else(|| quote! { #crate_path::DefaultScalarValue });

        let description = self
            .description
            .as_ref()
            .map(|desc| quote! { .description(#desc) });

        let var_types: Vec<_> = self.variants.iter().map(|var| &var.ty).collect();

        let all_variants_unique = if var_types.len() > 1 {
            Some(quote! { #crate_path::sa::assert_type_ne_all!(#(#var_types),*); })
        } else {
            None
        };

        let match_names = self.variants.iter().map(|var| {
            let var_ty = &var.ty;
            let var_check = &var.resolver_check;
            quote! {
                if #var_check {
                    return <#var_ty as #crate_path::GraphQLType<#scalar>>::name(&())
                        .unwrap().to_string();
                }
            }
        });

        let match_resolves: Vec<_> = self.variants.iter().map(|var| &var.resolver_code).collect();
        let resolve_into_type = self.variants.iter().zip(match_resolves.iter()).map(|(var, expr)| {
            let var_ty = &var.ty;

            let get_name = quote! { (<#var_ty as #crate_path::GraphQLType<#scalar>>::name(&())) };
            quote! {
                if type_name == #get_name.unwrap() {
                    return #crate_path::IntoResolvable::into(
                        { #expr },
                        executor.context()
                    )
                    .and_then(|res| match res {
                        Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(&(), &r),
                        None => Ok(#crate_path::Value::null()),
                    });
                }
            }
        });
        let resolve_into_type_async =
            self.variants
                .iter()
                .zip(match_resolves.iter())
                .map(|(var, expr)| {
                    let var_ty = &var.ty;

                    let get_name = quote! {
                        (<#var_ty as #crate_path::GraphQLType<#scalar>>::name(&()))
                    };
                    quote! {
                        if type_name == #get_name.unwrap() {
                            let res = #crate_path::IntoResolvable::into(
                                { #expr },
                                executor.context()
                            );
                            return #crate_path::futures::future::FutureExt::boxed(async move {
                                match res? {
                                    Some((ctx, r)) => {
                                        let subexec = executor.replaced_context(ctx);
                                        subexec.resolve_with_ctx_async(&(), &r).await
                                    },
                                    None => Ok(#crate_path::Value::null()),
                                }
                            });
                        }
                    }
                });

        let (_, ty_generics, _) = self.generics.split_for_impl();

        let mut base_generics = self.generics.clone();
        if self.is_trait_object {
            base_generics.params.push(parse_quote! { '__obj });
        }
        let (impl_generics, _, _) = base_generics.split_for_impl();

        let mut ext_generics = base_generics.clone();
        if self.scalar.is_none() {
            ext_generics.params.push(parse_quote! { #scalar });
            ext_generics
                .where_clause
                .get_or_insert_with(|| parse_quote! { where })
                .predicates
                .push(parse_quote! { #scalar: #crate_path::ScalarValue });
        }
        let (ext_impl_generics, _, where_clause) = ext_generics.split_for_impl();

        let mut where_async = where_clause
            .cloned()
            .unwrap_or_else(|| parse_quote! { where });
        where_async
            .predicates
            .push(parse_quote! { Self: Send + Sync });
        if self.scalar.is_none() {
            where_async
                .predicates
                .push(parse_quote! { #scalar: Send + Sync });
        }

        let mut ty_full = quote! { #ty#ty_generics };
        if self.is_trait_object {
            ty_full = quote! { dyn #ty_full + '__obj + Send + Sync };
        }

        let type_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics #crate_path::GraphQLType<#scalar> for #ty_full
                #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn name(_ : &Self::TypeInfo) -> Option<&str> {
                    Some(#name)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut #crate_path::Registry<'r, #scalar>
                ) -> #crate_path::meta::MetaType<'r, #scalar>
                where #scalar: 'r,
                {
                    let types = &[
                        #( registry.get_type::<&#var_types>(&(())), )*
                    ];
                    registry.build_union_type::<#ty_full>(info, types)
                    #description
                    .into_meta()
                }

                fn concrete_type_name(
                    &self,
                    context: &Self::Context,
                    _: &Self::TypeInfo,
                ) -> String {
                    #( #match_names )*
                    panic!(
                        "GraphQL union {} cannot be resolved into any of its variants in its \
                         current state",
                        #name,
                    );
                }

                fn resolve_into_type(
                    &self,
                    _: &Self::TypeInfo,
                    type_name: &str,
                    _: Option<&[#crate_path::Selection<#scalar>]>,
                    executor: &#crate_path::Executor<Self::Context, #scalar>,
                ) -> #crate_path::ExecutionResult<#scalar> {
                    let context = executor.context();
                    #( #resolve_into_type )*
                    panic!(
                        "Concrete type {} is not handled by instance resolvers on GraphQL union {}",
                        type_name, #name,
                    );
                }
            }
        };

        let async_type_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics #crate_path::GraphQLTypeAsync<#scalar> for #ty_full
                #where_async
            {
                fn resolve_into_type_async<'b>(
                    &'b self,
                    _: &'b Self::TypeInfo,
                    type_name: &str,
                    _: Option<&'b [#crate_path::Selection<'b, #scalar>]>,
                    executor: &'b #crate_path::Executor<'b, 'b, Self::Context, #scalar>
                ) -> #crate_path::BoxFuture<'b, #crate_path::ExecutionResult<#scalar>> {
                    let context = executor.context();
                    #( #resolve_into_type_async )*
                    panic!(
                        "Concrete type {} is not handled by instance resolvers on GraphQL union {}",
                        type_name, #name,
                    );
                }
            }
        };

        let output_type_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics #crate_path::marker::IsOutputType<#scalar> for #ty_full
                #where_clause
            {
                fn mark() {
                    #( <#var_types as #crate_path::marker::GraphQLObjectType<#scalar>>::mark(); )*
                }
            }
        };

        let union_impl = quote! {
            #[automatically_derived]
            impl#impl_generics #crate_path::marker::GraphQLUnion<#default_scalar> for #ty_full {
                fn mark() {
                    #all_variants_unique

                    #( <#var_types as #crate_path::marker::GraphQLObjectType<
                        #default_scalar,
                    >>::mark(); )*
                }
            }
        };

        into.append_all(&[union_impl, output_type_impl, type_impl, async_type_impl]);
    }
}

/// Emerges [`UnionMeta::external_resolvers`] into the given [GraphQL union][1] `variants`.
///
/// If duplication happens, then resolving code is overwritten with the one from
/// `external_resolvers`.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
fn emerge_union_variants_from_meta(
    variants: &mut Vec<UnionVariantDefinition>,
    external_resolvers: UnionMetaResolvers,
    mode: Mode,
) {
    if external_resolvers.is_empty() {
        return;
    }

    let crate_path = mode.crate_path();

    for (ty, rslvr) in external_resolvers {
        let span = rslvr.span_joined();

        let resolver_fn = rslvr.into_inner();
        let resolver_code = parse_quote! {
            #resolver_fn(self, #crate_path::FromContext::from(context))
        };
        // Doing this may be quite an expensive, because resolving may contain some heavy
        // computation, so we're preforming it twice. Unfortunately, we have no other options here,
        // until the `juniper::GraphQLType` itself will allow to do it in some cleverer way.
        let resolver_check = parse_quote! {
            ({ #resolver_code } as ::std::option::Option<&#ty>).is_some()
        };

        if let Some(var) = variants.iter_mut().find(|v| v.ty == ty) {
            var.resolver_code = resolver_code;
            var.resolver_check = resolver_check;
            var.span = span;
        } else {
            variants.push(UnionVariantDefinition {
                ty,
                resolver_code,
                resolver_check,
                enum_path: None,
                context_ty: None,
                span,
            })
        }
    }
}

/// Checks whether all [GraphQL union][1] `variants` represent a different Rust type.
///
/// # Notice
///
/// This is not an optimal implementation, as it's possible to bypass this check by using a full
/// qualified path instead (`crate::Test` vs `Test`). Since this requirement is mandatory, the
/// static assertion [`assert_type_ne_all!`][2] is used to enforce this requirement in the generated
/// code. However, due to the bad error message this implementation should stay and provide
/// guidance.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
/// [2]: https://docs.rs/static_assertions/latest/static_assertions/macro.assert_type_ne_all.html
fn all_variants_different(variants: &Vec<UnionVariantDefinition>) -> bool {
    let mut types: Vec<_> = variants.iter().map(|var| &var.ty).collect();
    types.dedup();
    types.len() == variants.len()
}
