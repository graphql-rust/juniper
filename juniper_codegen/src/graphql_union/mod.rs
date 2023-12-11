//! Code generation for [GraphQL union][1].
//!
//! [1]: https://spec.graphql.org/October2021#sec-Unions

pub mod attr;
pub mod derive;

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
    token,
};

use crate::common::{
    filter_attrs, gen,
    parse::{
        attr::{err, OptionExt as _},
        ParseBufferExt as _,
    },
    scalar, AttrNames, Description, SpanContainer,
};

/// Helper alias for the type of [`Attr::external_resolvers`] field.
type AttrResolvers = HashMap<syn::Type, SpanContainer<syn::ExprPath>>;

/// Available arguments behind `#[graphql]` (or `#[graphql_union]`) attribute
/// when generating code for [GraphQL union][1] type.
///
/// [1]: https://spec.graphql.org/October2021#sec-Unions
#[derive(Debug, Default)]
struct Attr {
    /// Explicitly specified name of [GraphQL union][1] type.
    ///
    /// If [`None`], then Rust type name is used by default.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of [GraphQL union][1] type.
    ///
    /// If [`None`], then Rust doc comment will be used as the [description][2],
    /// if any.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<SpanContainer<Description>>,

    /// Explicitly specified type of [`Context`] to use for resolving this
    /// [GraphQL union][1] type with.
    ///
    /// If [`None`], then unit type `()` is assumed as a type of [`Context`].
    ///
    /// [`Context`]: juniper::Context
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type of [`ScalarValue`] to use for resolving this
    /// [GraphQL union][1] type with.
    ///
    /// If [`None`], then generated code will be generic over any
    /// [`ScalarValue`] type, which, in turn, requires all [union][1] variants
    /// to be generic over any [`ScalarValue`] type too. That's why this type
    /// should be specified only if one of the variants implements
    /// [`GraphQLType`] in a non-generic way over [`ScalarValue`] type.
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    scalar: Option<SpanContainer<scalar::AttrValue>>,

    /// Explicitly specified external resolver functions for [GraphQL union][1]
    /// variants.
    ///
    /// If [`None`], then macro will try to auto-infer all the possible variants
    /// from the type declaration, if possible. That's why specifying an
    /// external resolver function has sense, when some custom [union][1]
    /// variant resolving logic is involved, or variants cannot be inferred.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    external_resolvers: AttrResolvers,

    /// Indicator whether the generated code is intended to be used only inside
    /// the [`juniper`] library.
    is_internal: bool,
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
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(name.span()),
                            name.value(),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "desc" | "description" => {
                    input.parse::<token::Eq>()?;
                    let desc = input.parse::<Description>()?;
                    out.description
                        .replace(SpanContainer::new(ident.span(), Some(desc.span()), desc))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "ctx" | "context" | "Context" => {
                    input.parse::<token::Eq>()?;
                    let ctx = input.parse::<syn::Type>()?;
                    out.context
                        .replace(SpanContainer::new(ident.span(), Some(ctx.span()), ctx))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "scalar" | "Scalar" | "ScalarValue" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<scalar::AttrValue>()?;
                    out.scalar
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "on" => {
                    let ty = input.parse::<syn::Type>()?;
                    input.parse::<token::Eq>()?;
                    let rslvr = input.parse::<syn::ExprPath>()?;
                    let rslvr_spanned = SpanContainer::new(ident.span(), Some(ty.span()), rslvr);
                    let rslvr_span = rslvr_spanned.span_joined();
                    out.external_resolvers
                        .insert(ty, rslvr_spanned)
                        .none_or_else(|_| err::dup_arg(rslvr_span))?
                }
                "internal" => {
                    out.is_internal = true;
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
            context: try_merge_opt!(context: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            external_resolvers: try_merge_hashmap!(
                external_resolvers: self, another => span_joined
            ),
            is_internal: self.is_internal || another.is_internal,
        })
    }

    /// Parses an [`Attr`] from the provided multiple [`syn::Attribute`]s with
    /// the specified `names`, placed on a trait or type definition.
    fn from_attrs(names: impl AttrNames, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut meta = filter_attrs(names, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if meta.description.is_none() {
            meta.description = Description::parse_from_doc_attrs(attrs)?;
        }

        Ok(meta)
    }
}

/// Available arguments behind `#[graphql]` attribute when generating code for
/// [GraphQL union][1]'s variant.
///
/// [1]: https://spec.graphql.org/October2021#sec-Unions
#[derive(Debug, Default)]
struct VariantAttr {
    /// Explicitly specified marker for the variant/field being ignored and not
    /// included into [GraphQL union][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    ignore: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified external resolver function for this [GraphQL union][1] variant.
    ///
    /// If absent, then macro will generate the code which just returns the variant inner value.
    /// Usually, specifying an external resolver function has sense, when some custom resolving
    /// logic is involved.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    external_resolver: Option<SpanContainer<syn::ExprPath>>,
}

impl Parse for VariantAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut out = Self::default();
        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "ignore" | "skip" => out
                    .ignore
                    .replace(SpanContainer::new(ident.span(), None, ident.clone()))
                    .none_or_else(|_| err::dup_arg(&ident))?,
                "with" => {
                    input.parse::<token::Eq>()?;
                    let rslvr = input.parse::<syn::ExprPath>()?;
                    out.external_resolver
                        .replace(SpanContainer::new(ident.span(), Some(rslvr.span()), rslvr))
                        .none_or_else(|_| err::dup_arg(&ident))?
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

impl VariantAttr {
    /// Tries to merge two [`VariantAttr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            ignore: try_merge_opt!(ignore: self, another),
            external_resolver: try_merge_opt!(external_resolver: self, another),
        })
    }

    /// Parses [`VariantAttr`] from the given multiple `name`d
    /// [`syn::Attribute`]s placed on a variant/field/method definition.
    fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}

/// Definition of [GraphQL union][1] for code generation.
///
/// [1]: https://spec.graphql.org/October2021#sec-Unions
struct Definition {
    /// Name of this [GraphQL union][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    name: String,

    /// Rust type that this [GraphQL union][1] is represented with.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    ty: syn::Type,

    /// Generics of the Rust type that this [GraphQL union][1] is implemented
    /// for.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    generics: syn::Generics,

    /// Indicator whether code should be generated for a trait object, rather
    /// than for a regular Rust type.
    is_trait_object: bool,

    /// Description of this [GraphQL union][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    description: Option<Description>,

    /// Rust type of [`Context`] to generate [`GraphQLType`] implementation with
    /// for this [GraphQL union][1].
    ///
    /// [`Context`]: juniper::Context
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    context: syn::Type,

    /// Rust type of [`ScalarValue`] to generate [`GraphQLType`] implementation
    /// with for this [GraphQL union][1].
    ///
    /// If [`None`] then generated code will be generic over any [`ScalarValue`]
    /// type, which, in turn, requires all [union][1] variants to be generic
    /// over any [`ScalarValue`] type too. That's why this type should be
    /// specified only if one of the variants implements [`GraphQLType`] in a
    /// non-generic way over [`ScalarValue`] type.
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    scalar: scalar::Type,

    /// Variants definitions of this [GraphQL union][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    variants: Vec<VariantDefinition>,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.impl_graphql_union_tokens().to_tokens(into);
        self.impl_output_type_tokens().to_tokens(into);
        self.impl_graphql_type_tokens().to_tokens(into);
        self.impl_graphql_value_tokens().to_tokens(into);
        self.impl_graphql_value_async_tokens().to_tokens(into);
        self.impl_reflection_traits_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Returns prepared [`syn::Generics::split_for_impl`] for [`GraphQLType`]
    /// trait (and similar) implementation of this [GraphQL union][1].
    ///
    /// If `for_async` is `true`, then additional predicates are added to suit
    /// the [`GraphQLAsyncValue`] trait (and similar) requirements.
    ///
    /// [`GraphQLAsyncValue`]: juniper::GraphQLAsyncValue
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    #[must_use]
    fn impl_generics(
        &self,
        for_async: bool,
    ) -> (TokenStream, TokenStream, Option<syn::WhereClause>) {
        let (_, ty_generics, _) = self.generics.split_for_impl();
        let ty = &self.ty;

        let mut ty_full = quote! { #ty #ty_generics };
        if self.is_trait_object {
            ty_full =
                quote! { dyn #ty_full + '__obj + ::core::marker::Send + ::core::marker::Sync };
        }

        let mut generics = self.generics.clone();

        if self.is_trait_object {
            generics.params.push(parse_quote! { '__obj });
        }

        let scalar = &self.scalar;
        if scalar.is_implicit_generic() {
            generics.params.push(parse_quote! { #scalar });
        }
        if scalar.is_generic() {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { #scalar: ::juniper::ScalarValue });
        }
        if let Some(bound) = scalar.bounds() {
            generics.make_where_clause().predicates.push(bound);
        }

        if for_async {
            let self_ty = if !self.is_trait_object && self.generics.lifetimes().next().is_some() {
                // Modify lifetime names to omit "lifetime name `'a` shadows a
                // lifetime name that is already in scope" error.
                let mut generics = self.generics.clone();
                for lt in generics.lifetimes_mut() {
                    let ident = lt.lifetime.ident.unraw();
                    lt.lifetime.ident = format_ident!("__fa__{ident}");
                }

                let lifetimes = generics.lifetimes().map(|lt| &lt.lifetime);
                let ty = &self.ty;
                let (_, ty_generics, _) = generics.split_for_impl();

                quote! { for<#( #lifetimes ),*> #ty #ty_generics }
            } else {
                quote! { Self }
            };
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { #self_ty: ::core::marker::Sync });

            if scalar.is_generic() {
                generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote! { #scalar: ::core::marker::Send + ::core::marker::Sync });
            }
        }

        let (impl_generics, _, where_clause) = generics.split_for_impl();
        (
            quote! { #impl_generics },
            quote! { #ty_full },
            where_clause.cloned(),
        )
    }

    /// Returns generated code implementing [`GraphQLUnion`] trait for this
    /// [GraphQL union][1].
    ///
    /// [`GraphQLUnion`]: juniper::GraphQLUnion
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    #[must_use]
    fn impl_graphql_union_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_full, where_clause) = self.impl_generics(false);

        let variant_tys: Vec<_> = self.variants.iter().map(|var| &var.ty).collect();
        let all_variants_unique = (variant_tys.len() > 1).then(|| {
            quote! { ::juniper::sa::assert_type_ne_all!(#( #variant_tys ),*); }
        });

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::marker::GraphQLUnion<#scalar> for #ty_full #where_clause
            {
                fn mark() {
                    #all_variants_unique
                    #( <#variant_tys as ::juniper::marker::GraphQLObject<#scalar>>::mark(); )*
                }
            }
        }
    }

    /// Returns generated code implementing [`marker::IsOutputType`] trait for
    /// this [GraphQL union][1].
    ///
    /// [`marker::IsOutputType`]: juniper::marker::IsOutputType
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    #[must_use]
    fn impl_output_type_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_full, where_clause) = self.impl_generics(false);

        let variant_tys = self.variants.iter().map(|var| &var.ty);

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::marker::IsOutputType<#scalar> for #ty_full #where_clause
            {
                fn mark() {
                    #( <#variant_tys as ::juniper::marker::IsOutputType<#scalar>>::mark(); )*
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL union][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    #[must_use]
    fn impl_graphql_type_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_full, where_clause) = self.impl_generics(false);

        let name = &self.name;
        let description = &self.description;

        let variant_tys = self.variants.iter().map(|var| &var.ty);

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLType<#scalar> for #ty_full #where_clause
            {
                fn name(
                    _ : &Self::TypeInfo,
                ) -> ::core::option::Option<&'static ::core::primitive::str> {
                    ::core::option::Option::Some(#name)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut ::juniper::Registry<'r, #scalar>
                ) -> ::juniper::meta::MetaType<'r, #scalar>
                where #scalar: 'r,
                {
                    let types = [
                        #( registry.get_type::<#variant_tys>(info), )*
                    ];
                    registry.build_union_type::<#ty_full>(info, &types)
                        #description
                        .into_meta()
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValue`] trait for this
    /// [GraphQL union][1].
    ///
    /// [`GraphQLValue`]: juniper::GraphQLValue
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    #[must_use]
    fn impl_graphql_value_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;
        let context = &self.context;

        let (impl_generics, ty_full, where_clause) = self.impl_generics(false);

        let name = &self.name;

        let match_variant_names = self
            .variants
            .iter()
            .map(|v| v.method_concrete_type_name_tokens(scalar));

        let variant_resolvers = self
            .variants
            .iter()
            .map(|v| v.method_resolve_into_type_tokens(scalar));

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLValue<#scalar> for #ty_full #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn type_name<'__i>(
                    &self,
                    info: &'__i Self::TypeInfo,
                ) -> ::core::option::Option<&'__i ::core::primitive::str> {
                    <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                }

                fn concrete_type_name(
                    &self,
                    context: &Self::Context,
                    info: &Self::TypeInfo,
                ) -> ::std::string::String {
                    #( #match_variant_names )*
                    ::core::panic!(
                        "GraphQL union `{}` cannot be resolved into any of its \
                         variants in its current state",
                        #name,
                    );
                }

                fn resolve_into_type(
                    &self,
                    info: &Self::TypeInfo,
                    type_name: &::core::primitive::str,
                    _: ::core::option::Option<&[::juniper::Selection<'_, #scalar>]>,
                    executor: &::juniper::Executor<'_, '_, Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    let context = executor.context();
                    #( #variant_resolvers )*
                    return ::core::result::Result::Err(::juniper::FieldError::from(::std::format!(
                        "Concrete type `{}` is not handled by instance \
                         resolvers on GraphQL union `{}`",
                        type_name, #name,
                    )));
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValueAsync`] trait for this
    /// [GraphQL union][1].
    ///
    /// [`GraphQLValueAsync`]: juniper::GraphQLValueAsync
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    #[must_use]
    fn impl_graphql_value_async_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_full, where_clause) = self.impl_generics(true);

        let name = &self.name;

        let variant_async_resolvers = self
            .variants
            .iter()
            .map(|v| v.method_resolve_into_type_async_tokens(scalar));

        quote! {
            #[allow(non_snake_case)]
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLValueAsync<#scalar> for #ty_full #where_clause
            {
                fn resolve_into_type_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    type_name: &::core::primitive::str,
                    _: ::core::option::Option<&'b [::juniper::Selection<'b, #scalar>]>,
                    executor: &'b ::juniper::Executor<'b, 'b, Self::Context, #scalar>
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    let context = executor.context();
                    #( #variant_async_resolvers )*
                    return ::juniper::macros::helper::err_fut(::std::format!(
                        "Concrete type `{}` is not handled by instance \
                         resolvers on GraphQL union `{}`",
                        type_name, #name,
                    ));
                }
            }
        }
    }

    /// Returns generated code implementing [`BaseType`], [`BaseSubTypes`] and
    /// [`WrappedType`] traits for this [GraphQL union][1].
    ///
    /// [`BaseSubTypes`]: juniper::macros::reflect::BaseSubTypes
    /// [`BaseType`]: juniper::macros::reflect::BaseType
    /// [`WrappedType`]: juniper::macros::reflect::WrappedType
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    #[must_use]
    pub(crate) fn impl_reflection_traits_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;
        let name = &self.name;
        let variants = self.variants.iter().map(|var| &var.ty);
        let (impl_generics, ty, where_clause) = self.impl_generics(false);

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::macros::reflect::BaseType<#scalar>
                for #ty
                #where_clause
            {
                const NAME: ::juniper::macros::reflect::Type = #name;
            }

            #[automatically_derived]
            impl #impl_generics ::juniper::macros::reflect::BaseSubTypes<#scalar>
                for #ty
                #where_clause
            {
                const NAMES: ::juniper::macros::reflect::Types = &[
                    <Self as ::juniper::macros::reflect::BaseType<#scalar>>::NAME,
                    #(<#variants as ::juniper::macros::reflect::BaseType<#scalar>>::NAME),*
                ];
            }

            #[automatically_derived]
            impl #impl_generics ::juniper::macros::reflect::WrappedType<#scalar>
                for #ty
                #where_clause
            {
                const VALUE: ::juniper::macros::reflect::WrappedValue = 1;
            }
        }
    }
}

/// Definition of [GraphQL union][1] variant for code generation.
///
/// [1]: https://spec.graphql.org/October2021#sec-Unions
struct VariantDefinition {
    /// Rust type that this [GraphQL union][1] variant resolves into.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    ty: syn::Type,

    /// Rust code for value resolution of this [GraphQL union][1] variant.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    resolver_code: syn::Expr,

    /// Rust code for checking whether [GraphQL union][1] should be resolved
    /// into this variant.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    resolver_check: syn::Expr,

    /// Rust type of [`Context`] that this [GraphQL union][1] variant requires
    /// for resolution.
    ///
    /// It's available only when code generation happens for Rust traits and a
    /// trait method contains context argument.
    ///
    /// [`Context`]: juniper::Context
    /// [1]: https://spec.graphql.org/October2021#sec-Unions
    context: Option<syn::Type>,
}

impl VariantDefinition {
    /// Returns generated code for the [`GraphQLValue::concrete_type_name`][0]
    /// method, which returns name of the underlying GraphQL type contained in
    /// this [`VariantDefinition`].
    ///
    /// [0]: juniper::GraphQLValue::concrete_type_name
    #[must_use]
    fn method_concrete_type_name_tokens(&self, scalar: &scalar::Type) -> TokenStream {
        let ty = &self.ty;
        let check = &self.resolver_check;

        quote! {
            if #check {
                return <#ty as ::juniper::GraphQLType<#scalar>>::name(info)
                    .unwrap()
                    .to_string();
            }
        }
    }

    /// Returns generated code for the [`GraphQLValue::resolve_into_type`][0]
    /// method, which resolves the underlying GraphQL type contained in this
    /// [`VariantDefinition`] synchronously.
    ///
    /// [0]: juniper::GraphQLValue::resolve_into_type
    #[must_use]
    fn method_resolve_into_type_tokens(&self, scalar: &scalar::Type) -> TokenStream {
        let ty = &self.ty;
        let ty_name = ty.to_token_stream().to_string();
        let expr = &self.resolver_code;
        let resolving_code = gen::sync_resolving_code();

        quote! {
            if type_name == <#ty as ::juniper::GraphQLType<#scalar>>::name(info)
                .ok_or_else(|| ::juniper::macros::helper::err_unnamed_type(#ty_name))?
            {
                let res = { #expr };
                return #resolving_code;
            }
        }
    }

    /// Returns generated code for the
    /// [`GraphQLValueAsync::resolve_into_type_async`][0] method, which
    /// resolves the underlying GraphQL type contained in this
    /// [`VariantDefinition`] asynchronously.
    ///
    /// [0]: juniper::GraphQLValueAsync::resolve_into_type_async
    #[must_use]
    fn method_resolve_into_type_async_tokens(&self, scalar: &scalar::Type) -> TokenStream {
        let ty = &self.ty;
        let ty_name = ty.to_token_stream().to_string();
        let expr = &self.resolver_code;
        let resolving_code = gen::async_resolving_code(None);

        quote! {
            match <#ty as ::juniper::GraphQLType<#scalar>>::name(info) {
                ::core::option::Option::Some(name) => {
                    if type_name == name {
                        let fut = ::juniper::futures::future::ready({ #expr });
                        return #resolving_code;
                    }
                }
                ::core::option::Option::None => {
                    return ::juniper::macros::helper::err_unnamed_type_fut(#ty_name);
                }
            }
        }
    }
}

/// Emerges [`Attr::external_resolvers`] into the given [GraphQL union][1]
/// `variants`.
///
/// If duplication happens, then resolving code is overwritten with the one from
/// `external_resolvers`.
///
/// [1]: https://spec.graphql.org/October2021#sec-Unions
fn emerge_union_variants_from_attr(
    variants: &mut Vec<VariantDefinition>,
    external_resolvers: AttrResolvers,
) {
    if external_resolvers.is_empty() {
        return;
    }

    for (ty, rslvr) in external_resolvers {
        let resolver_fn = rslvr.into_inner();
        let resolver_code = parse_quote! {
            #resolver_fn(self, ::juniper::FromContext::from(context))
        };
        // Doing this may be quite an expensive, because resolving may contain
        // some heavy computation, so we're preforming it twice. Unfortunately,
        // we have no other options here, until the `juniper::GraphQLType`
        // itself will allow to do it in some cleverer way.
        let resolver_check = parse_quote! {
            ({ #resolver_code } as ::core::option::Option<&#ty>).is_some()
        };

        if let Some(var) = variants.iter_mut().find(|v| v.ty == ty) {
            var.resolver_code = resolver_code;
            var.resolver_check = resolver_check;
        } else {
            variants.push(VariantDefinition {
                ty,
                resolver_code,
                resolver_check,
                context: None,
            })
        }
    }
}

/// Checks whether all [GraphQL union][1] `variants` represent a different Rust
/// type.
///
/// # Notice
///
/// This is not an optimal implementation, as it's possible to bypass this check
/// by using a full qualified path instead (`crate::Test` vs `Test`). Since this
/// requirement is mandatory, the static assertion [`assert_type_ne_all!`][2] is
/// used to enforce this requirement in the generated code. However, due to the
/// bad error message this implementation should stay and provide guidance.
///
/// [1]: https://spec.graphql.org/October2021#sec-Unions
/// [2]: juniper::sa::assert_type_ne_all
fn all_variants_different(variants: &[VariantDefinition]) -> bool {
    let mut types: Vec<_> = variants.iter().map(|var| &var.ty).collect();
    types.dedup();
    types.len() == variants.len()
}
