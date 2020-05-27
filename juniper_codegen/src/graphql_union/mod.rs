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

/// Available metadata behind `#[graphql]` (or `#[graphql_union]`) attribute when generating code
/// for [GraphQL union][1] type.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
#[derive(Debug, Default)]
struct UnionMeta {
    /// Explicitly specified name of [GraphQL union][1] type.
    ///
    /// If absent, then `PascalCase`d Rust type name is used by default.
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
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type of `juniper::ScalarValue` to use for resolving this
    /// [GraphQL union][1] type with.
    ///
    /// If absent, then generated code will be generic over any `juniper::ScalarValue` type, which,
    /// in turn, requires all [union][1] variants to be generic over any `juniper::ScalarValue` type
    /// too. That's why this type should be specified only if one of the variants implements
    /// `juniper::GraphQLType` in non-generic over `juniper::ScalarValue` type way.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub scalar: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified custom resolver functions for [GraphQL union][1] variants.
    ///
    /// If absent, then macro will try to auto-infer all the possible variants from the type
    /// declaration, if possible. That's why specifying a custom resolver function has sense, when
    /// some custom [union][1] variant resolving logic is involved, or variants cannot be inferred.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub custom_resolvers: HashMap<syn::Type, SpanContainer<syn::ExprPath>>,
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
                        .none_or_else(|_| syn::Error::new(ident.span(), "duplicated attribute"))?
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
                        .none_or_else(|_| syn::Error::new(ident.span(), "duplicated attribute"))?
                }
                "ctx" | "context" | "Context" => {
                    input.parse::<syn::Token![=]>()?;
                    let ctx = input.parse::<syn::Type>()?;
                    output
                        .context
                        .replace(SpanContainer::new(ident.span(), Some(ctx.span()), ctx))
                        .none_or_else(|_| syn::Error::new(ident.span(), "duplicated attribute"))?
                }
                "scalar" | "Scalar" | "ScalarValue" => {
                    input.parse::<syn::Token![=]>()?;
                    let scl = input.parse::<syn::Type>()?;
                    output
                        .scalar
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| syn::Error::new(ident.span(), "duplicated attribute"))?
                }
                "on" => {
                    let ty = input.parse::<syn::Type>()?;
                    input.parse::<syn::Token![=]>()?;
                    let rslvr = input.parse::<syn::ExprPath>()?;
                    let rslvr_spanned = SpanContainer::new(ident.span(), Some(ty.span()), rslvr);
                    let rslvr_span = rslvr_spanned.span_joined();
                    output
                        .custom_resolvers
                        .insert(ty, rslvr_spanned)
                        .none_or_else(|_| syn::Error::new(rslvr_span, "duplicated attribute"))?
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
    fn try_merge(self, mut other: Self) -> syn::Result<Self> {
        Ok(Self {
            name: {
                if let Some(v) = self.name {
                    other.name.replace(v).none_or_else(|dup| {
                        syn::Error::new(dup.span_ident(), "duplicated attribute")
                    })?;
                }
                other.name
            },
            description: {
                if let Some(v) = self.description {
                    other.description.replace(v).none_or_else(|dup| {
                        syn::Error::new(dup.span_ident(), "duplicated attribute")
                    })?;
                }
                other.description
            },
            context: {
                if let Some(v) = self.context {
                    other.context.replace(v).none_or_else(|dup| {
                        syn::Error::new(dup.span_ident(), "duplicated attribute")
                    })?;
                }
                other.context
            },
            scalar: {
                if let Some(v) = self.scalar {
                    other.scalar.replace(v).none_or_else(|dup| {
                        syn::Error::new(dup.span_ident(), "duplicated attribute")
                    })?;
                }
                other.scalar
            },
            custom_resolvers: {
                if !self.custom_resolvers.is_empty() {
                    for (ty, rslvr) in self.custom_resolvers {
                        other
                            .custom_resolvers
                            .insert(ty, rslvr)
                            .none_or_else(|dup| {
                                syn::Error::new(dup.span_joined(), "duplicated attribute")
                            })?;
                    }
                }
                other.custom_resolvers
            },
        })
    }

    /// Parses [`UnionMeta`] from the given attributes placed on type definition.
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

/// Available metadata behind `#[graphql]` (or `#[graphql_union]`) attribute when generating code
/// for [GraphQL union][1]'s variant.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
#[derive(Debug, Default)]
struct UnionVariantMeta {
    /// Explicitly specified marker for the variant/field being ignored and not included into
    /// [GraphQL union][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub ignore: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified custom resolver function for this [GraphQL union][1] variant.
    ///
    /// If absent, then macro will generate the code which just returns the variant inner value.
    /// Usually, specifying a custom resolver function has sense, when some custom resolving logic
    /// is involved.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub custom_resolver: Option<SpanContainer<syn::ExprPath>>,
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
                    .none_or_else(|_| syn::Error::new(ident.span(), "duplicated attribute"))?,
                "with" => {
                    input.parse::<syn::Token![=]>()?;
                    let rslvr = input.parse::<syn::ExprPath>()?;
                    output
                        .custom_resolver
                        .replace(SpanContainer::new(ident.span(), Some(rslvr.span()), rslvr))
                        .none_or_else(|_| syn::Error::new(ident.span(), "duplicated attribute"))?
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
    fn try_merge(self, mut other: Self) -> syn::Result<Self> {
        Ok(Self {
            ignore: {
                if let Some(v) = self.ignore {
                    other.ignore.replace(v).none_or_else(|dup| {
                        syn::Error::new(dup.span_ident(), "duplicated attribute")
                    })?;
                }
                other.ignore
            },
            custom_resolver: {
                if let Some(v) = self.custom_resolver {
                    other.custom_resolver.replace(v).none_or_else(|dup| {
                        syn::Error::new(dup.span_ident(), "duplicated attribute")
                    })?;
                }
                other.custom_resolver
            },
        })
    }

    /// Parses [`UnionVariantMeta`] from the given attributes placed on variant/field definition.
    pub fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}

struct UnionVariantDefinition {
    pub ty: syn::Type,
    pub resolver_code: syn::Expr,
    pub resolver_check: syn::Expr,
    pub enum_path: Option<TokenStream>,
    pub span: Span,
}

struct UnionDefinition {
    pub name: String,
    pub ty: syn::Type,
    pub is_trait_object: bool,
    pub description: Option<String>,
    pub context: Option<syn::Type>,
    pub scalar: Option<syn::Type>,
    pub generics: syn::Generics,
    pub variants: Vec<UnionVariantDefinition>,
    pub span: Span,
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

                    let get_name =
                        quote! { (<#var_ty as #crate_path::GraphQLType<#scalar>>::name(&())) };
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
            ty_full = quote! { dyn #ty_full + '__obj };
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
            impl#impl_generics #crate_path::marker::GraphQLUnion for #ty_full {
                fn mark() {
                    #( <#var_types as #crate_path::marker::GraphQLObjectType<
                        #default_scalar,
                    >>::mark(); )*
                }
            }
        };

        into.append_all(&[union_impl, output_type_impl, type_impl, async_type_impl]);
    }
}
