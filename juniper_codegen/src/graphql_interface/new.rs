//! Code generation for [GraphQL interface][1].
//!
//! [1]: https://spec.graphql.org/June2018/#sec-Interfaces

use std::{collections::HashSet, convert::TryInto as _};

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned as _,
    token,
    visit::Visit,
};

use crate::{
    common::{
        field, gen,
        parse::{
            attr::{err, OptionExt as _},
            GenericsExt as _, ParseBufferExt as _,
        },
        scalar,
    },
    util::{filter_attrs, get_doc_comment, span_container::SpanContainer, RenameRule},
};

/// Available arguments behind `#[graphql_interface]` attribute placed on a
/// trait definition, when generating code for [GraphQL interface][1] type.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
#[derive(Debug, Default)]
pub(crate) struct TraitAttr {
    /// Explicitly specified name of [GraphQL interface][1] type.
    ///
    /// If [`None`], then Rust trait name is used by default.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of [GraphQL interface][1] type.
    ///
    /// If [`None`], then Rust doc comment is used as [description][2], if any.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    pub(crate) description: Option<SpanContainer<String>>,

    /// Explicitly specified identifier of the type alias of Rust enum type
    /// behind the trait, being an actual implementation of a
    /// [GraphQL interface][1] type.
    ///
    /// If [`None`], then `{trait_name}Value` identifier will be used.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) r#enum: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified Rust types of [GraphQL objects][2] implementing
    /// this [GraphQL interface][1] type.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    /// [2]: https://spec.graphql.org/June2018/#sec-Objects
    pub(crate) implementers: HashSet<SpanContainer<syn::TypePath>>,

    /// Explicitly specified type of [`Context`] to use for resolving this
    /// [GraphQL interface][1] type with.
    ///
    /// If [`None`], then unit type `()` is assumed as a type of [`Context`].
    ///
    /// [`Context`]: juniper::Context
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type (or type parameter with its bounds) of
    /// [`ScalarValue`] to resolve this [GraphQL interface][1] type with.
    ///
    /// If [`None`], then generated code will be generic over any
    /// [`ScalarValue`] type, which, in turn, requires all [interface][1]
    /// implementers to be generic over any [`ScalarValue`] type too. That's why
    /// this type should be specified only if one of the implementers implements
    /// [`GraphQLType`] in a non-generic way over [`ScalarValue`] type.
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) scalar: Option<SpanContainer<scalar::AttrValue>>,

    /// Explicitly specified marker indicating that the Rust trait should be
    /// transformed into [`async_trait`].
    ///
    /// If [`None`], then trait will be transformed into [`async_trait`] only if
    /// it contains async methods.
    pub(crate) asyncness: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified [`RenameRule`] for all fields of this
    /// [GraphQL interface][1] type.
    ///
    /// If [`None`] then the default rule will be [`RenameRule::CamelCase`].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) rename_fields: Option<SpanContainer<RenameRule>>,

    /// Indicator whether the generated code is intended to be used only inside
    /// the [`juniper`] library.
    pub(crate) is_internal: bool,
}

impl Parse for TraitAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut out = Self::default();
        while !input.is_empty() {
            let ident = input.parse_any_ident()?;
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
                    let desc = input.parse::<syn::LitStr>()?;
                    out.description
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
                "for" | "implementers" => {
                    input.parse::<token::Eq>()?;
                    for impler in input.parse_maybe_wrapped_and_punctuated::<
                        syn::TypePath, token::Bracket, token::Comma,
                    >()? {
                        let impler_span = impler.span();
                        out
                            .implementers
                            .replace(SpanContainer::new(ident.span(), Some(impler_span), impler))
                            .none_or_else(|_| err::dup_arg(impler_span))?;
                    }
                }
                "enum" => {
                    input.parse::<token::Eq>()?;
                    let alias = input.parse::<syn::Ident>()?;
                    out.r#enum
                        .replace(SpanContainer::new(ident.span(), Some(alias.span()), alias))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "async" => {
                    let span = ident.span();
                    out.asyncness
                        .replace(SpanContainer::new(span, Some(span), ident))
                        .none_or_else(|_| err::dup_arg(span))?;
                }
                "rename_all" => {
                    input.parse::<token::Eq>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    out.rename_fields
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(val.span()),
                            val.try_into()?,
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?;
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

impl TraitAttr {
    /// Tries to merge two [`TraitAttr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            context: try_merge_opt!(context: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            implementers: try_merge_hashset!(implementers: self, another => span_joined),
            r#enum: try_merge_opt!(r#enum: self, another),
            asyncness: try_merge_opt!(asyncness: self, another),
            rename_fields: try_merge_opt!(rename_fields: self, another),
            is_internal: self.is_internal || another.is_internal,
        })
    }

    /// Parses [`TraitAttr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a trait definition.
    pub(crate) fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut attr = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if attr.description.is_none() {
            attr.description = get_doc_comment(attrs);
        }

        Ok(attr)
    }
}

/// Definition of [GraphQL interface][1] for code generation.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
pub(crate) struct Definition {
    /// [`syn::Generics`] of the trait describing the [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) trait_generics: syn::Generics,

    /// [`syn::Visibility`] of the trait describing the [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) vis: syn::Visibility,

    /// Name of the generic enum describing all [`implementers`]. It's generic
    /// to derive [`Clone`], [`Copy`] and [`Debug`] on it.
    ///
    /// [`implementers`]: Self::implementers
    /// [`Debug`]: std::fmt::Debug
    pub(crate) enum_ident: syn::Ident,

    /// Name of the type alias for [`enum_ident`] with [`implementers`].
    ///
    /// [`enum_ident`]: Self::enum_ident
    /// [`implementers`]: Self::implementers
    pub(crate) enum_alias_ident: syn::Ident,

    /// Name of this [GraphQL interface][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) name: String,

    /// Description of this [GraphQL interface][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) description: Option<String>,

    /// Rust type of [`Context`] to generate [`GraphQLType`] implementation with
    /// for this [GraphQL interface][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`Context`]: juniper::Context
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) context: syn::Type,

    /// [`ScalarValue`] parametrization to generate [`GraphQLType`]
    /// implementation with for this [GraphQL interface][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) scalar: scalar::Type,

    /// Defined [GraphQL fields][2] of this [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    /// [2]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) fields: Vec<field::Definition>,

    /// Defined [`Implementer`]s of this [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub(crate) implementers: Vec<syn::TypePath>,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.generate_enum().to_tokens(into);
        self.impl_graphql_interface_tokens().to_tokens(into);
        self.impl_output_type_tokens().to_tokens(into);
        self.impl_graphql_type_tokens().to_tokens(into);
        self.impl_graphql_value_tokens().to_tokens(into);
        self.impl_graphql_value_async_tokens().to_tokens(into);
        self.impl_traits_for_reflection().to_tokens(into);
        self.impl_field_meta().to_tokens(into);
        self.impl_field().to_tokens(into);
        self.impl_async_field().to_tokens(into);
    }
}

impl Definition {
    /// Generates enum describing all [`implementers`].
    ///
    /// [`implementers`]: Self::implementers
    #[must_use]
    fn generate_enum(&self) -> TokenStream {
        let vis = &self.vis;
        let enum_ident = &self.enum_ident;
        let alias_ident = &self.enum_alias_ident;

        let variant_gens_pars = self
            .implementers
            .iter()
            .enumerate()
            .map::<syn::GenericParam, _>(|(id, _)| {
                let par = format_ident!("__I{}", id);
                parse_quote!( #par )
            });

        let variants_idents = self
            .implementers
            .iter()
            .filter_map(|ty| ty.path.segments.last().map(|seg| &seg.ident));

        let trait_gens = &self.trait_generics;
        let (trait_impl_gens, trait_ty_gens, trait_where_clause) =
            self.trait_generics.split_for_impl();

        let (trait_gens_lifetimes, trait_gens_tys) = trait_gens
            .params
            .clone()
            .into_iter()
            .partition::<Punctuated<_, _>, _>(|par| {
            matches!(par, syn::GenericParam::Lifetime(_))
        });

        let enum_gens = {
            let mut enum_gens = trait_gens.clone();
            enum_gens.params = trait_gens_lifetimes.clone();
            enum_gens.params.extend(variant_gens_pars.clone());
            enum_gens.params.extend(trait_gens_tys.clone());
            enum_gens
        };

        let enum_alias_gens = {
            let mut enum_alias_gens = trait_gens.clone();
            enum_alias_gens.move_bounds_to_where_clause();
            enum_alias_gens
        };

        let enum_to_alias_gens = {
            trait_gens_lifetimes
                .into_iter()
                .map(|par| match par {
                    syn::GenericParam::Lifetime(def) => {
                        let lifetime = &def.lifetime;
                        quote! { #lifetime }
                    }
                    rest => quote! { #rest },
                })
                .chain(self.implementers.iter().map(ToTokens::to_token_stream))
                .chain(trait_gens_tys.into_iter().map(|par| match par {
                    syn::GenericParam::Type(ty) => {
                        let par_ident = &ty.ident;
                        quote! { #par_ident }
                    }
                    rest => quote! { #rest },
                }))
        };

        let phantom_variant = self.has_phantom_variant().then(|| {
            let phantom_params = trait_gens.params.iter().filter_map(|p| {
                let ty = match p {
                    syn::GenericParam::Type(ty) => {
                        let ident = &ty.ident;
                        quote! { #ident }
                    }
                    syn::GenericParam::Lifetime(lt) => {
                        let lifetime = &lt.lifetime;
                        quote! { &#lifetime () }
                    }
                    syn::GenericParam::Const(_) => return None,
                };
                Some(quote! {
                    ::std::marker::PhantomData<::std::sync::atomic::AtomicPtr<Box<#ty>>>
                })
            });
            quote! { __Phantom(#(#phantom_params),*) }
        });

        let from_impls = self.implementers.iter().zip(variants_idents.clone()).map(
            |(ty, ident)| {
                quote! {
                    impl#trait_impl_gens ::std::convert::From<#ty> for #alias_ident#trait_ty_gens
                        #trait_where_clause
                    {
                        fn from(v: #ty) -> Self {
                            Self::#ident(v)
                        }
                    }
                }
            },
        );

        quote! {
            #[derive(Clone, Copy, Debug)]
            #vis enum #enum_ident#enum_gens {
                #(#variants_idents(#variant_gens_pars),)*
                #phantom_variant
            }

            #vis type #alias_ident#enum_alias_gens =
                #enum_ident<#(#enum_to_alias_gens),*>;

            #(#from_impls)*
        }
    }

    /// Returns generated code implementing [`GraphQLInterface`] trait for this
    /// [GraphQL interface][1].
    ///
    /// [`GraphQLInterface`]: juniper::GraphQLInterface
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    #[must_use]
    fn impl_graphql_interface_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;

        let gens = self.impl_generics(false);
        let (impl_generics, _, where_clause) = gens.split_for_impl();
        let (_, ty_generics, _) = self.trait_generics.split_for_impl();

        let impler_tys = &self.implementers;
        let all_implers_unique = (impler_tys.len() > 1).then(|| {
            quote! { ::juniper::sa::assert_type_ne_all!(#( #impler_tys ),*); }
        });

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::marker::GraphQLInterface<#scalar> for
                #ty#ty_generics
                #where_clause
            {
                fn mark() {
                    #all_implers_unique
                    #( <#impler_tys as ::juniper::marker::GraphQLObject<#scalar>>::mark(); )*
                }
            }
        }
    }

    /// Returns generated code implementing [`marker::IsOutputType`] trait for
    /// this [GraphQL interface][1].
    ///
    /// [`marker::IsOutputType`]: juniper::marker::IsOutputType
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    #[must_use]
    fn impl_output_type_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.trait_generics.split_for_impl();

        let fields_marks = self
            .fields
            .iter()
            .map(|f| f.method_mark_tokens(false, scalar));

        let impler_tys = self.implementers.iter();

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::marker::IsOutputType<#scalar> for
                #ty#ty_generics
                #where_clause
            {
                fn mark() {
                    #( #fields_marks )*
                    #( <#impler_tys as ::juniper::marker::IsOutputType<#scalar>>::mark(); )*
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL interface][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    #[must_use]
    fn impl_graphql_type_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.trait_generics.split_for_impl();

        let name = &self.name;
        let description = self
            .description
            .as_ref()
            .map(|desc| quote! { .description(#desc) });

        // Sorting is required to preserve/guarantee the order of implementers registered in schema.
        let mut impler_tys = self.implementers.clone();
        impler_tys.sort_unstable_by(|a, b| {
            let (a, b) = (quote!(#a).to_string(), quote!(#b).to_string());
            a.cmp(&b)
        });

        let fields_meta = self.fields.iter().map(|f| f.method_meta_tokens(None));

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::GraphQLType<#scalar>
                for #ty#ty_generics
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
                    #( let _ = registry.get_type::<#impler_tys>(info); )*

                    let fields = [
                        #( #fields_meta, )*
                    ];
                    registry.build_interface_type::<#ty#ty_generics>(info, &fields)
                        #description
                        .into_meta()
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValue`] trait for this
    /// [GraphQL interface][1].
    ///
    /// [`GraphQLValue`]: juniper::GraphQLValue
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    #[must_use]
    fn impl_graphql_value_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let trait_name = &self.name;
        let scalar = &self.scalar;
        let context = &self.context;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.trait_generics.split_for_impl();

        let fields_resolvers = self.fields.iter().filter_map(|f| {
            (!f.is_async).then(|| {
                let name = &f.name;
                quote! {
                    #name => {
                        ::juniper::macros::reflection::Field::<
                            #scalar,
                            { ::juniper::macros::reflection::fnv1a128(#name) }
                        >::call(self, info, args, executor)
                    }
                }
            })
        });
        let async_fields_err = {
            let names = self
                .fields
                .iter()
                .filter_map(|f| f.is_async.then(|| f.name.as_str()))
                .collect::<Vec<_>>();
            (!names.is_empty()).then(|| {
                field::Definition::method_resolve_field_err_async_field_tokens(
                    &names, scalar, trait_name,
                )
            })
        };
        let no_field_err =
            field::Definition::method_resolve_field_err_no_field_tokens(scalar, trait_name);

        let downcast_check = self.method_concrete_type_name_tokens();

        let downcast = self.method_resolve_into_type_tokens();

        quote! {
            #[allow(deprecated)]
            #[automatically_derived]
            impl#impl_generics ::juniper::GraphQLValue<#scalar> for #ty#ty_generics
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
                        #( #fields_resolvers )*
                        #async_fields_err
                        _ => #no_field_err,
                    }
                }

                fn concrete_type_name(
                    &self,
                    context: &Self::Context,
                    info: &Self::TypeInfo,
                ) -> String {
                    #downcast_check
                }

                fn resolve_into_type(
                    &self,
                    info: &Self::TypeInfo,
                    type_name: &str,
                    _: Option<&[::juniper::Selection<#scalar>]>,
                    executor: &::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    #downcast
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValueAsync`] trait for this
    /// [GraphQL interface][1].
    ///
    /// [`GraphQLValueAsync`]: juniper::GraphQLValueAsync
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    #[must_use]
    fn impl_graphql_value_async_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let trait_name = &self.name;
        let scalar = &self.scalar;

        let generics = self.impl_generics(true);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.trait_generics.split_for_impl();

        let fields_resolvers = self.fields.iter().map(|f| {
            let name = &f.name;
            quote! {
                #name => {
                    ::juniper::macros::reflection::AsyncField::<
                        #scalar,
                        { ::juniper::macros::reflection::fnv1a128(#name) }
                    >::call(self, info, args, executor)
                }
            }
        });
        let no_field_err =
            field::Definition::method_resolve_field_err_no_field_tokens(scalar, trait_name);

        let downcast = self.method_resolve_into_type_async_tokens();

        quote! {
            #[allow(deprecated, non_snake_case)]
            #[automatically_derived]
            impl#impl_generics ::juniper::GraphQLValueAsync<#scalar> for #ty#ty_generics
                #where_clause
            {
                fn resolve_field_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    field: &'b str,
                    args: &'b ::juniper::Arguments<#scalar>,
                    executor: &'b ::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    match field {
                        #( #fields_resolvers )*
                        _ => Box::pin(async move { #no_field_err }),
                    }
                }

                fn resolve_into_type_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    type_name: &str,
                    _: Option<&'b [::juniper::Selection<'b, #scalar>]>,
                    executor: &'b ::juniper::Executor<'b, 'b, Self::Context, #scalar>
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    #downcast
                }
            }
        }
    }

    /// Returns generated code implementing [`BaseType`], [`BaseSubTypes`] and
    /// [`WrappedType`] traits for this [GraphQL interface][1].
    ///
    /// [`BaseSubTypes`]: juniper::macros::reflection::BaseSubTypes
    /// [`BaseType`]: juniper::macros::reflection::BaseType
    /// [`WrappedType`]: juniper::macros::reflection::WrappedType
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    #[must_use]
    pub(crate) fn impl_traits_for_reflection(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let implementers = &self.implementers;
        let scalar = &self.scalar;
        let name = &self.name;
        let fields = self.fields.iter().map(|f| &f.name);

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.trait_generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::macros::reflection::BaseType<#scalar>
                for #ty#ty_generics
                #where_clause
            {
                const NAME: ::juniper::macros::reflection::Type = #name;
            }

            #[automatically_derived]
            impl#impl_generics ::juniper::macros::reflection::BaseSubTypes<#scalar>
                for #ty#ty_generics
                #where_clause
            {
                const NAMES: ::juniper::macros::reflection::Types = &[
                    <Self as ::juniper::macros::reflection::BaseType<#scalar>>::NAME,
                    #(<#implementers as ::juniper::macros::reflection::BaseType<#scalar>>::NAME),*
                ];
            }

            #[automatically_derived]
            impl#impl_generics ::juniper::macros::reflection::WrappedType<#scalar>
                for #ty#ty_generics
                #where_clause
            {
                const VALUE: ::juniper::macros::reflection::WrappedValue = 1;
            }

            #[automatically_derived]
            impl#impl_generics ::juniper::macros::reflection::Fields<#scalar>
                for #ty#ty_generics
                #where_clause
            {
                const NAMES: ::juniper::macros::reflection::Names = &[#(#fields),*];
            }
        }
    }

    /// Returns generated code implementing [`FieldMeta`] for this
    /// [GraphQL interface][1].
    ///
    /// [`FieldMeta`]: juniper::macros::reflection::FieldMeta
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    fn impl_field_meta(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let context = &self.context;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.trait_generics.split_for_impl();

        self.fields
            .iter()
            .map(|field| {
                let field_name = &field.name;
                let mut return_ty = field.ty.clone();
                generics.replace_type_with_defaults(&mut return_ty);

                let (args_tys, args_names): (Vec<_>, Vec<_>) = field
                    .arguments
                    .iter()
                    .flat_map(|vec| vec.iter())
                    .filter_map(|arg| match arg {
                        field::MethodArgument::Regular(arg) => Some((&arg.ty, &arg.name)),
                        _ => None,
                    })
                    .unzip();

                quote! {
                    #[allow(non_snake_case)]
                    impl#impl_generics ::juniper::macros::reflection::FieldMeta<
                        #scalar,
                        { ::juniper::macros::reflection::fnv1a128(#field_name) }
                    > for #ty#ty_generics #where_clause {
                        type Context = #context;
                        type TypeInfo = ();
                        const TYPE: ::juniper::macros::reflection::Type =
                            <#return_ty as ::juniper::macros::reflection::BaseType<#scalar>>::NAME;
                        const SUB_TYPES: ::juniper::macros::reflection::Types =
                            <#return_ty as ::juniper::macros::reflection::BaseSubTypes<#scalar>>::NAMES;
                        const WRAPPED_VALUE: ::juniper::macros::reflection::WrappedValue =
                            <#return_ty as ::juniper::macros::reflection::WrappedType<#scalar>>::VALUE;
                        const ARGUMENTS: &'static [(
                            ::juniper::macros::reflection::Name,
                            ::juniper::macros::reflection::Type,
                            ::juniper::macros::reflection::WrappedValue,
                        )] = &[#((
                            #args_names,
                            <#args_tys as ::juniper::macros::reflection::BaseType<#scalar>>::NAME,
                            <#args_tys as ::juniper::macros::reflection::WrappedType<#scalar>>::VALUE,
                        )),*];
                    }
                }
            })
            .collect()
    }

    /// Returns generated code implementing [`Field`] or [`AsyncField`] trait
    /// for this [GraphQL interface][1].
    ///
    /// [`AsyncField`]: juniper::macros::reflection::AsyncField
    /// [`Field`]: juniper::macros::reflection::Field
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    fn impl_field(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;
        let const_scalar = self.const_scalar();

        let impl_tys = self.implementers.iter().collect::<Vec<_>>();
        let impl_idents = self
            .implementers
            .iter()
            .filter_map(|ty| ty.path.segments.last().map(|seg| &seg.ident))
            .collect::<Vec<_>>();

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.trait_generics.split_for_impl();

        self.fields
            .iter()
            .filter_map(|field| {
                if field.is_async {
                    return None;
                }

                let field_name = &field.name;
                let mut return_ty = field.ty.clone();
                generics.replace_type_with_defaults(&mut return_ty);

                let const_ty_generics = self.const_trait_generics();

                let unreachable_arm = (self.implementers.is_empty()
                    || !self.trait_generics.params.is_empty())
                .then(|| {
                    quote! { _ => unreachable!() }
                });

                Some(quote! {
                    #[allow(non_snake_case)]
                    impl#impl_generics ::juniper::macros::reflection::Field<
                        #scalar,
                        { ::juniper::macros::reflection::fnv1a128(#field_name) }
                    > for #ty#ty_generics #where_clause {
                        fn call(
                            &self,
                            info: &Self::TypeInfo,
                            args: &::juniper::Arguments<#scalar>,
                            executor: &::juniper::Executor<Self::Context, #scalar>,
                        ) -> ::juniper::ExecutionResult<#scalar> {
                            match self {
                                #(#ty::#impl_idents(v) => {
                                    ::juniper::assert_field!(
                                        #ty#const_ty_generics,
                                        #impl_tys,
                                        #const_scalar,
                                        #field_name,
                                    );

                                    <_ as ::juniper::macros::reflection::Field<
                                        #scalar,
                                        { ::juniper::macros::reflection::fnv1a128(#field_name) },
                                    >>::call(v, info, args, executor)
                                })*
                                #unreachable_arm
                            }
                        }
                    }
                })
            })
            .collect()
    }

    /// Returns generated code implementing [`AsyncField`] trait for this
    /// [GraphQL interface][1].
    ///
    /// [`AsyncField`]: juniper::macros::reflection::AsyncField
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    fn impl_async_field(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;
        let const_scalar = self.const_scalar();

        let impl_tys = self.implementers.iter().collect::<Vec<_>>();
        let impl_idents = self
            .implementers
            .iter()
            .filter_map(|ty| ty.path.segments.last().map(|seg| &seg.ident))
            .collect::<Vec<_>>();

        let generics = self.impl_generics(true);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.trait_generics.split_for_impl();

        self.fields
            .iter()
            .map(|field| {
                let field_name = &field.name;
                let mut return_ty = field.ty.clone();
                generics.replace_type_with_defaults(&mut return_ty);

                let const_ty_generics = self.const_trait_generics();

                let unreachable_arm = (self.implementers.is_empty()
                    || !self.trait_generics.params.is_empty())
                .then(|| {
                    quote! { _ => unreachable!() }
                });

                quote! {
                    #[allow(non_snake_case)]
                    impl#impl_generics ::juniper::macros::reflection::AsyncField<
                        #scalar,
                        { ::juniper::macros::reflection::fnv1a128(#field_name) }
                    > for #ty#ty_generics #where_clause {
                        fn call<'b>(
                            &'b self,
                            info: &'b Self::TypeInfo,
                            args: &'b ::juniper::Arguments<#scalar>,
                            executor: &'b ::juniper::Executor<Self::Context, #scalar>,
                        ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                            match self {
                                #(#ty::#impl_idents(v) => {
                                    ::juniper::assert_field!(
                                        #ty#const_ty_generics,
                                        #impl_tys,
                                        #const_scalar,
                                        #field_name,
                                    );

                                    <_ as ::juniper::macros::reflection::AsyncField<
                                        #scalar,
                                        { ::juniper::macros::reflection::fnv1a128(#field_name) },
                                    >>::call(v, info, args, executor)
                                })*
                                #unreachable_arm
                            }
                        }
                    }
                }
            })
            .collect()
    }

    /// Returns generated code for the [`GraphQLValue::concrete_type_name`][0]
    /// method, which returns name of the underlying [`Implementer`] GraphQL
    /// type contained in this [`EnumType`].
    ///
    /// [0]: juniper::GraphQLValue::concrete_type_name
    #[must_use]
    fn method_concrete_type_name_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let match_arms = self
            .implementers
            .iter()
            .filter_map(|ty| ty.path.segments.last().map(|seg| (&seg.ident, ty)))
            .map(|(ident, ty)| {
                quote! {
                    Self::#ident(v) => <
                        #ty as ::juniper::GraphQLValue<#scalar>
                    >::concrete_type_name(v, context, info),
                }
            });

        let non_exhaustive_match_arm =
            (!self.trait_generics.params.is_empty() || self.implementers.is_empty()).then(|| {
                quote! { _ => unreachable!(), }
            });

        quote! {
            match self {
                #( #match_arms )*
                #non_exhaustive_match_arm
            }
        }
    }

    /// Returns generated code for the
    /// [`GraphQLValueAsync::resolve_into_type_async`][0] method, which
    /// downcasts this [`EnumType`] into its underlying [`Implementer`] type
    /// asynchronously.
    ///
    /// [0]: juniper::GraphQLValueAsync::resolve_into_type_async
    #[must_use]
    fn method_resolve_into_type_async_tokens(&self) -> TokenStream {
        let resolving_code = gen::async_resolving_code(None);

        let match_arms = self.implementers.iter().filter_map(|ty| {
            ty.path.segments.last().map(|ident| {
                quote! {
                    Self::#ident(v) => {
                        let fut = ::juniper::futures::future::ready(v);
                        #resolving_code
                    }
                }
            })
        });
        let non_exhaustive_match_arm =
            (!self.trait_generics.params.is_empty() || self.implementers.is_empty()).then(|| {
                quote! { _ => unreachable!(), }
            });

        quote! {
            match self {
                #( #match_arms )*
                #non_exhaustive_match_arm
            }
        }
    }

    /// Returns generated code for the [`GraphQLValue::resolve_into_type`][0]
    /// method, which downcasts this [`EnumType`] into its underlying
    /// [`Implementer`] type synchronously.
    ///
    /// [0]: juniper::GraphQLValue::resolve_into_type
    #[must_use]
    fn method_resolve_into_type_tokens(&self) -> TokenStream {
        let resolving_code = gen::sync_resolving_code();

        let match_arms = self.implementers.iter().filter_map(|ty| {
            ty.path.segments.last().map(|ident| {
                quote! {
                    Self::#ident(res) => #resolving_code,
                }
            })
        });

        let non_exhaustive_match_arm =
            (!self.trait_generics.params.is_empty() || self.implementers.is_empty()).then(|| {
                quote! { _ => unreachable!(), }
            });

        quote! {
            match self {
                #( #match_arms )*
                #non_exhaustive_match_arm
            }
        }
    }

    /// Returns trait generics replaced with default values for usage in `const`
    /// context.
    #[must_use]
    fn const_trait_generics(&self) -> syn::PathArguments {
        struct GenericsForConst(syn::AngleBracketedGenericArguments);

        impl Visit<'_> for GenericsForConst {
            fn visit_generic_param(&mut self, param: &syn::GenericParam) {
                let arg = match param {
                    syn::GenericParam::Lifetime(_) => parse_quote!( 'static ),
                    syn::GenericParam::Type(ty) => {
                        if ty.default.is_none() {
                            parse_quote!(::juniper::DefaultScalarValue)
                        } else {
                            return;
                        }
                    }
                    syn::GenericParam::Const(c) => {
                        if c.default.is_none() {
                            // This hack works because only `min_const_generics`
                            // are enabled for now.
                            // TODO: replace this once full `const_generics` are
                            //       available.
                            //       Maybe with `<_ as Default>::default()`?
                            parse_quote!({ 0_u8 as _ })
                        } else {
                            return;
                        }
                    }
                };
                self.0.args.push(arg)
            }
        }

        let mut visitor = GenericsForConst(parse_quote!( <> ));
        visitor.visit_generics(&self.trait_generics);
        syn::PathArguments::AngleBracketed(visitor.0)
    }

    /// Returns [`scalar`] replaced with [`DefaultScalarValue`] in case it's
    /// [`ExplicitGeneric`] or [`ImplicitGeneric`] for using [`scalar`] in
    /// `const` context.
    ///
    /// [`scalar`]: Self::scalar
    /// [`DefaultScalarValue`]: juniper::DefaultScalarValue
    /// [`ExplicitGeneric`]: scalar::Type::ExplicitGeneric
    /// [`ImplicitGeneric`]: scalar::Type::ImplicitGeneric
    #[must_use]
    fn const_scalar(&self) -> syn::Type {
        match &self.scalar {
            scalar::Type::Concrete(ty) => ty.clone(),
            scalar::Type::ExplicitGeneric(_) | scalar::Type::ImplicitGeneric(_) => {
                parse_quote! { ::juniper::DefaultScalarValue }
            }
        }
    }

    /// Returns prepared [`syn::Generics`] for [`GraphQLType`] trait (and
    /// similar) implementation of this [`EnumType`].
    ///
    /// If `for_async` is `true`, then additional predicates are added to suit
    /// the [`GraphQLAsyncValue`] trait (and similar) requirements.
    ///
    /// [`GraphQLAsyncValue`]: juniper::GraphQLAsyncValue
    /// [`GraphQLType`]: juniper::GraphQLType
    #[must_use]
    fn impl_generics(&self, for_async: bool) -> syn::Generics {
        let mut generics = self.trait_generics.clone();

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
            let self_ty = if self.trait_generics.lifetimes().next().is_some() {
                // Modify lifetime names to omit "lifetime name `'a` shadows a
                // lifetime name that is already in scope" error.
                let mut generics = self.trait_generics.clone();
                for lt in generics.lifetimes_mut() {
                    let ident = lt.lifetime.ident.unraw();
                    lt.lifetime.ident = format_ident!("__fa__{}", ident);
                }

                let lifetimes = generics.lifetimes().map(|lt| &lt.lifetime);
                let ty = &self.enum_alias_ident;
                let (_, ty_generics, _) = generics.split_for_impl();

                quote! { for<#( #lifetimes ),*> #ty#ty_generics }
            } else {
                quote! { Self }
            };
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { #self_ty: Sync });

            if scalar.is_generic() {
                generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote! { #scalar: Send + Sync });
            }
        }

        generics
    }

    /// Indicates whether this [`EnumType`] has non-exhaustive phantom variant
    /// to hold type parameters.
    #[must_use]
    fn has_phantom_variant(&self) -> bool {
        !self.trait_generics.params.is_empty()
    }
}
