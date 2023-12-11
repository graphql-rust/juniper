//! Code generation for [GraphQL interface][1].
//!
//! [1]: https://spec.graphql.org/October2021#sec-Interfaces

pub mod attr;
pub mod derive;

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token,
    visit::Visit,
};

use crate::common::{
    field, filter_attrs, gen,
    parse::{
        attr::{err, OptionExt as _},
        GenericsExt as _, ParseBufferExt as _,
    },
    rename, scalar, AttrNames, Description, SpanContainer,
};

/// Returns [`syn::Ident`]s for a generic enum deriving [`Clone`] and [`Copy`]
/// on it and enum alias which generic arguments are filled with
/// [GraphQL interface][1] implementers.
///
/// [1]: https://spec.graphql.org/October2021#sec-Interfaces
fn enum_idents(
    trait_ident: &syn::Ident,
    alias_ident: Option<&syn::Ident>,
) -> (syn::Ident, syn::Ident) {
    let enum_alias_ident = alias_ident
        .cloned()
        .unwrap_or_else(|| format_ident!("{trait_ident}Value"));
    let enum_ident = alias_ident.map_or_else(
        || format_ident!("{trait_ident}ValueEnum"),
        |c| format_ident!("{c}Enum"),
    );
    (enum_ident, enum_alias_ident)
}

/// Available arguments behind `#[graphql_interface]` attribute placed on a
/// trait or struct definition, when generating code for [GraphQL interface][1]
/// type.
///
/// [1]: https://spec.graphql.org/October2021#sec-Interfaces
#[derive(Debug, Default)]
struct Attr {
    /// Explicitly specified name of [GraphQL interface][1] type.
    ///
    /// If [`None`], then Rust trait name is used by default.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of [GraphQL interface][1] type.
    ///
    /// If [`None`], then Rust doc comment will be used as the [description][2],
    /// if any.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<SpanContainer<Description>>,

    /// Explicitly specified identifier of the type alias of Rust enum type
    /// behind the trait or struct, being an actual implementation of a
    /// [GraphQL interface][1] type.
    ///
    /// If [`None`], then `{trait_name}Value` identifier will be used.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    r#enum: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified Rust types of [GraphQL objects][2] or
    /// [interfaces][1] implementing this [GraphQL interface][1] type.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    /// [2]: https://spec.graphql.org/October2021#sec-Objects
    implemented_for: HashSet<SpanContainer<syn::TypePath>>,

    /// Explicitly specified [GraphQL interfaces, implemented][1] by this
    /// [GraphQL interface][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Interfaces
    /// [1]: https://spec.graphql.org/October2021#sel-GAHbhBDABAB_E-0b
    implements: HashSet<SpanContainer<syn::TypePath>>,

    /// Explicitly specified type of [`Context`] to use for resolving this
    /// [GraphQL interface][1] type with.
    ///
    /// If [`None`], then unit type `()` is assumed as a type of [`Context`].
    ///
    /// [`Context`]: juniper::Context
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    context: Option<SpanContainer<syn::Type>>,

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
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    scalar: Option<SpanContainer<scalar::AttrValue>>,

    /// Explicitly specified marker indicating that the Rust trait should be
    /// transformed into [`async_trait`].
    ///
    /// If [`None`], then trait will be transformed into [`async_trait`] only if
    /// it contains async methods.
    asyncness: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified [`rename::Policy`] for all fields of this
    /// [GraphQL interface][1] type.
    ///
    /// If [`None`], then the [`rename::Policy::CamelCase`] will be applied by
    /// default.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    rename_fields: Option<SpanContainer<rename::Policy>>,

    /// Indicator whether the generated code is intended to be used only inside
    /// the [`juniper`] library.
    is_internal: bool,
}

impl Parse for Attr {
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
                "for" | "implementers" => {
                    input.parse::<token::Eq>()?;
                    for impler in input.parse_maybe_wrapped_and_punctuated::<
                        syn::TypePath, token::Bracket, token::Comma,
                    >()? {
                        let impler_span = impler.span();
                        out
                            .implemented_for
                            .replace(SpanContainer::new(ident.span(), Some(impler_span), impler))
                            .none_or_else(|_| err::dup_arg(impler_span))?;
                    }
                }
                "impl" | "implements" => {
                    input.parse::<token::Eq>()?;
                    for iface in input.parse_maybe_wrapped_and_punctuated::<
                        syn::TypePath, token::Bracket, token::Comma,
                    >()? {
                        let iface_span = iface.span();
                        out
                            .implements
                            .replace(SpanContainer::new(ident.span(), Some(iface_span), iface))
                            .none_or_else(|_| err::dup_arg(iface_span))?;
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

impl Attr {
    /// Tries to merge two [`TraitAttr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            context: try_merge_opt!(context: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            implemented_for: try_merge_hashset!(implemented_for: self, another => span_joined),
            implements: try_merge_hashset!(implements: self, another => span_joined),
            r#enum: try_merge_opt!(r#enum: self, another),
            asyncness: try_merge_opt!(asyncness: self, another),
            rename_fields: try_merge_opt!(rename_fields: self, another),
            is_internal: self.is_internal || another.is_internal,
        })
    }

    /// Parses a [`TraitAttr`] from the provided multiple [`syn::Attribute`]s with
    /// the specified `names`, placed on a trait or struct definition.
    fn from_attrs(names: impl AttrNames, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut attr = filter_attrs(names, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if attr.description.is_none() {
            attr.description = Description::parse_from_doc_attrs(attrs)?;
        }

        Ok(attr)
    }
}

/// Definition of [GraphQL interface][1] for code generation.
///
/// [1]: https://spec.graphql.org/October2021#sec-Interfaces
struct Definition {
    /// [`syn::Generics`] of the trait or struct describing the
    /// [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    generics: syn::Generics,

    /// [`syn::Visibility`] of the trait or struct describing the
    /// [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    vis: syn::Visibility,

    /// Name of the generic enum describing all [`implementers`]. It's generic
    /// to derive [`Clone`], [`Copy`] and [`Debug`] on it.
    ///
    /// [`implementers`]: Self::implementers
    /// [`Debug`]: std::fmt::Debug
    enum_ident: syn::Ident,

    /// Name of the type alias for [`enum_ident`] with [`implementers`].
    ///
    /// [`enum_ident`]: Self::enum_ident
    /// [`implementers`]: Self::implementers
    enum_alias_ident: syn::Ident,

    /// Name of this [GraphQL interface][0] in GraphQL schema.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Interfaces
    name: Box<str>,

    /// Description of this [GraphQL interface][0] to put into GraphQL schema.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Interfaces
    description: Option<Description>,

    /// Rust type of [`Context`] to generate [`GraphQLType`] implementation with
    /// for this [GraphQL interface][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`Context`]: juniper::Context
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    context: syn::Type,

    /// [`ScalarValue`] parametrization to generate [`GraphQLType`]
    /// implementation with for this [GraphQL interface][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    scalar: scalar::Type,

    /// Defined [GraphQL fields][2] of this [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    /// [2]: https://spec.graphql.org/October2021#sec-Language.Fields
    fields: Vec<field::Definition>,

    /// Defined [`Implementer`]s of this [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    implemented_for: Vec<syn::TypePath>,

    /// [GraphQL interfaces implemented][1] by this [GraphQL interface][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Interfaces
    /// [1]: https://spec.graphql.org/October2021#sel-GAHbhBDABAB_E-0b
    implements: Vec<syn::TypePath>,

    /// Unlike `#[graphql_interface]` maro, `#[derive(GraphQLInterface)]` can't
    /// append `#[allow(dead_code)]` to the unused struct, representing
    /// [GraphQL interface][1]. We generate hacky `const` which doesn't actually
    /// use it, but suppresses this warning.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    suppress_dead_code: Option<(syn::Ident, syn::Fields)>,

    /// Intra-doc link to the [`syn::Item`] defining this
    /// [GraphQL interface][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Interfaces
    src_intra_doc_link: Box<str>,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.generate_enum_tokens().to_tokens(into);
        self.impl_graphql_interface_tokens().to_tokens(into);
        self.impl_output_type_tokens().to_tokens(into);
        self.impl_graphql_type_tokens().to_tokens(into);
        self.impl_graphql_value_tokens().to_tokens(into);
        self.impl_graphql_value_async_tokens().to_tokens(into);
        self.impl_reflection_traits_tokens().to_tokens(into);
        self.impl_field_meta_tokens().to_tokens(into);
        self.impl_field_tokens().to_tokens(into);
        self.impl_async_field_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Generates enum describing all the [`implementers`].
    ///
    /// [`implementers`]: Self::implementers
    #[must_use]
    fn generate_enum_tokens(&self) -> TokenStream {
        let vis = &self.vis;
        let enum_ident = &self.enum_ident;
        let alias_ident = &self.enum_alias_ident;

        let variant_gens_pars = (0..self.implemented_for.len()).map::<syn::GenericParam, _>(|id| {
            let par = format_ident!("__I{id}");
            parse_quote! { #par }
        });
        let variants_idents = self
            .implemented_for
            .iter()
            .filter_map(|ty| ty.path.segments.last().map(|seg| &seg.ident));

        let interface_gens = &self.generics;
        let (interface_impl_gens, interface_ty_gens, interface_where_clause) =
            self.generics.split_for_impl();
        let (interface_gens_lifetimes, interface_gens_tys) = interface_gens
            .params
            .clone()
            .into_iter()
            .partition::<Punctuated<_, _>, _>(|par| matches!(par, syn::GenericParam::Lifetime(_)));

        let enum_gens = {
            let mut enum_gens = interface_gens.clone();
            enum_gens.params = interface_gens_lifetimes.clone();
            enum_gens.params.extend(variant_gens_pars.clone());
            enum_gens.params.extend(interface_gens_tys.clone());
            enum_gens
        };
        let enum_alias_gens = {
            let mut enum_alias_gens = interface_gens.clone();
            enum_alias_gens.move_bounds_to_where_clause();
            enum_alias_gens
        };
        let enum_to_alias_gens = {
            interface_gens_lifetimes
                .into_iter()
                .map(|par| match par {
                    syn::GenericParam::Lifetime(def) => {
                        let lifetime = &def.lifetime;
                        quote! { #lifetime }
                    }
                    rest => quote! { #rest },
                })
                .chain(self.implemented_for.iter().map(ToTokens::to_token_stream))
                .chain(interface_gens_tys.into_iter().map(|par| match par {
                    syn::GenericParam::Type(ty) => {
                        let par_ident = &ty.ident;
                        quote! { #par_ident }
                    }
                    rest => quote! { #rest },
                }))
        };
        let enum_doc = format!(
            "Enum building an opaque value represented by [`{}`]({}) \
             [GraphQL interface][0].\
             \n\n\
             [0]: https://spec.graphql.org/October2021#sec-Interfaces",
            self.name, self.src_intra_doc_link,
        );
        let enum_alias_doc = format!(
            "Opaque value represented by [`{}`]({}) [GraphQL interface][0].\
             \n\n\
             [0]: https://spec.graphql.org/October2021#sec-Interfaces",
            self.name, self.src_intra_doc_link,
        );

        let phantom_variant = self
            .has_phantom_variant()
            .then(|| {
                let phantom_params = interface_gens.params.iter().filter_map(|p| {
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
                        ::core::marker::PhantomData<
                            ::core::sync::atomic::AtomicPtr<std::boxed::Box<#ty>>
                        >
                    })
                });
                quote! { __Phantom(#(#phantom_params),*) }
            })
            .into_iter();

        let from_impls = self
            .implemented_for
            .iter()
            .zip(variants_idents.clone())
            .map(|(ty, ident)| {
                quote! {
                    #[automatically_derived]
                    impl #interface_impl_gens ::core::convert::From<#ty>
                        for #alias_ident #interface_ty_gens
                        #interface_where_clause
                    {
                        fn from(v: #ty) -> Self {
                            Self::#ident(v)
                        }
                    }
                }
            });

        quote! {
            #[automatically_derived]
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug)]
            #[doc = #enum_doc]
            #vis enum #enum_ident #enum_gens {
                #( #[doc(hidden)] #variants_idents(#variant_gens_pars), )*
                #( #[doc(hidden)] #phantom_variant, )*
            }

            #[automatically_derived]
            #[doc = #enum_alias_doc]
            #vis type #alias_ident #enum_alias_gens =
                #enum_ident<#( #enum_to_alias_gens ),*>;

            #( #from_impls )*
        }
    }

    /// Returns generated code implementing [`GraphQLInterface`] trait for this
    /// [GraphQL interface][1].
    ///
    /// [`GraphQLInterface`]: juniper::GraphQLInterface
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    #[must_use]
    fn impl_graphql_interface_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;

        let gens = self.impl_generics(false);
        let (impl_generics, _, where_clause) = gens.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let suppress_dead_code = self.suppress_dead_code.as_ref().map(|(ident, fields)| {
            let const_gens = self.const_trait_generics();
            let fields = fields.iter().map(|f| &f.ident);

            quote! {{
                const SUPPRESS_DEAD_CODE: () = {
                    let none = ::core::option::Option::<#ident #const_gens>::None;
                    match none {
                        ::core::option::Option::Some(unreachable) => {
                            #( let _ = unreachable.#fields; )*
                        }
                        ::core::option::Option::None => {}
                    }
                };
                let _ = SUPPRESS_DEAD_CODE;
            }}
        });

        let implemented_for = &self.implemented_for;
        let all_impled_for_unique = (implemented_for.len() > 1).then(|| {
            quote! { ::juniper::sa::assert_type_ne_all!(#( #implemented_for ),*); }
        });

        let mark_object_or_interface = self.implemented_for.iter().map(|impl_for| {
            quote_spanned! { impl_for.span() =>
                trait GraphQLObjectOrInterface<S: ::juniper::ScalarValue, T> {
                    fn mark();
                }

                {
                    struct Object;

                    impl<S, T> GraphQLObjectOrInterface<S, Object> for T
                    where
                        S: ::juniper::ScalarValue,
                        T: ::juniper::marker::GraphQLObject<S>,
                    {
                        fn mark() {
                            <T as ::juniper::marker::GraphQLObject<S>>::mark()
                        }
                    }
                }

                {
                    struct Interface;

                    impl<S, T> GraphQLObjectOrInterface<S, Interface> for T
                    where
                        S: ::juniper::ScalarValue,
                        T: ::juniper::marker::GraphQLInterface<S>,
                    {
                        fn mark() {
                            <T as ::juniper::marker::GraphQLInterface<S>>::mark()
                        }
                    }
                }

                <#impl_for as GraphQLObjectOrInterface<#scalar, _>>::mark();
            }
        });

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::marker::GraphQLInterface<#scalar>
                for #ty #ty_generics
                #where_clause
            {
                fn mark() {
                    #suppress_dead_code
                    #all_impled_for_unique
                    #( { #mark_object_or_interface } )*
                }
            }
        }
    }

    /// Returns generated code implementing [`marker::IsOutputType`] trait for
    /// this [GraphQL interface][1].
    ///
    /// [`marker::IsOutputType`]: juniper::marker::IsOutputType
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    #[must_use]
    fn impl_output_type_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;
        let const_scalar = &self.scalar.default_ty();

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();
        let ty_const_generics = self.const_trait_generics();

        let fields_marks = self
            .fields
            .iter()
            .map(|f| f.method_mark_tokens(false, scalar));

        let is_output = self.implemented_for.iter().map(|impler| {
            quote_spanned! { impler.span() =>
               <#impler as ::juniper::marker::IsOutputType<#scalar>>::mark();
            }
        });

        let const_impl_for = self.implemented_for.iter().cloned().map(|mut ty| {
            generics.replace_type_path_with_defaults(&mut ty);
            ty
        });
        let const_implements = self
            .implements
            .iter()
            .cloned()
            .map(|mut ty| {
                generics.replace_type_path_with_defaults(&mut ty);
                ty
            })
            .collect::<Vec<_>>();
        let transitive_checks = const_impl_for.clone().map(|const_impl_for| {
            quote_spanned! { const_impl_for.span() =>
                ::juniper::assert_transitive_impls!(
                    #const_scalar,
                    #ty #ty_const_generics,
                    #const_impl_for,
                    #( #const_implements ),*
                );
            }
        });

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::marker::IsOutputType<#scalar>
                for #ty #ty_generics
                #where_clause
            {
                fn mark() {
                    #( #fields_marks )*
                    #( #is_output )*
                    ::juniper::assert_interfaces_impls!(
                        #const_scalar,
                        #ty #ty_const_generics,
                        #( #const_impl_for ),*
                    );
                    ::juniper::assert_implemented_for!(
                        #const_scalar,
                        #ty #ty_const_generics,
                        #( #const_implements ),*
                    );
                    #( #transitive_checks )*
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL interface][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    #[must_use]
    fn impl_graphql_type_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let name = &self.name;
        let description = &self.description;

        // Sorting is required to preserve/guarantee the order of implementers registered in schema.
        let mut implemented_for = self.implemented_for.clone();
        implemented_for.sort_unstable_by(|a, b| {
            let (a, b) = (quote!(#a).to_string(), quote!(#b).to_string());
            a.cmp(&b)
        });

        // Sorting is required to preserve/guarantee the order of interfaces registered in schema.
        let mut implements = self.implements.clone();
        implements.sort_unstable_by(|a, b| {
            let (a, b) = (quote!(#a).to_string(), quote!(#b).to_string());
            a.cmp(&b)
        });
        let impl_interfaces = (!implements.is_empty()).then(|| {
            quote! {
                .interfaces(&[
                    #( registry.get_type::<#implements>(info), )*
                ])
            }
        });

        let fields_meta = self.fields.iter().map(|f| f.method_meta_tokens(None));

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLType<#scalar>
                for #ty #ty_generics
                #where_clause
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
                    // Ensure all implementer types are registered.
                    #( let _ = registry.get_type::<#implemented_for>(info); )*

                    let fields = [
                        #( #fields_meta, )*
                    ];
                    registry.build_interface_type::<#ty #ty_generics>(info, &fields)
                        #description
                        #impl_interfaces
                        .into_meta()
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValue`] trait for this
    /// [GraphQL interface][1].
    ///
    /// [`GraphQLValue`]: juniper::GraphQLValue
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    #[must_use]
    fn impl_graphql_value_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let trait_name = &self.name;
        let scalar = &self.scalar;
        let context = &self.context;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let fields_resolvers = self.fields.iter().map(|f| {
            let name = &f.name;
            Some(quote! {
                #name => {
                    ::juniper::macros::reflect::Field::<
                        #scalar,
                        { ::juniper::macros::reflect::fnv1a128(#name) }
                    >::call(self, info, args, executor)
                }
            })
        });

        let no_field_err =
            field::Definition::method_resolve_field_err_no_field_tokens(scalar, trait_name);

        let downcast_check = self.method_concrete_type_name_tokens();

        let downcast = self.method_resolve_into_type_tokens();

        quote! {
            #[allow(deprecated)]
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLValue<#scalar> for #ty #ty_generics
                #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn type_name<'__i>(
                    &self,
                    info: &'__i Self::TypeInfo,
                ) -> ::core::option::Option<&'__i ::core::primitive::str> {
                    <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                }

                fn resolve_field(
                    &self,
                    info: &Self::TypeInfo,
                    field: &::core::primitive::str,
                    args: &::juniper::Arguments<'_, #scalar>,
                    executor: &::juniper::Executor<'_, '_, Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    match field {
                        #( #fields_resolvers )*
                        _ => #no_field_err,
                    }
                }

                fn concrete_type_name(
                    &self,
                    context: &Self::Context,
                    info: &Self::TypeInfo,
                ) -> ::std::string::String {
                    #downcast_check
                }

                fn resolve_into_type(
                    &self,
                    info: &Self::TypeInfo,
                    type_name: &::core::primitive::str,
                    _: ::core::option::Option<&[::juniper::Selection<'_, #scalar>]>,
                    executor: &::juniper::Executor<'_, '_, Self::Context, #scalar>,
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
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    #[must_use]
    fn impl_graphql_value_async_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let trait_name = &self.name;
        let scalar = &self.scalar;

        let generics = self.impl_generics(true);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let fields_resolvers = self.fields.iter().map(|f| {
            let name = &f.name;
            quote! {
                #name => {
                    ::juniper::macros::reflect::AsyncField::<
                        #scalar,
                        { ::juniper::macros::reflect::fnv1a128(#name) }
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
            impl #impl_generics ::juniper::GraphQLValueAsync<#scalar> for #ty #ty_generics
                #where_clause
            {
                fn resolve_field_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    field: &'b ::core::primitive::str,
                    args: &'b ::juniper::Arguments<'_, #scalar>,
                    executor: &'b ::juniper::Executor<'_, '_, Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    match field {
                        #( #fields_resolvers )*
                        _ => ::std::boxed::Box::pin(async move { #no_field_err }),
                    }
                }

                fn resolve_into_type_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    type_name: &::core::primitive::str,
                    _: ::core::option::Option<&'b [::juniper::Selection<'b, #scalar>]>,
                    executor: &'b ::juniper::Executor<'b, 'b, Self::Context, #scalar>
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    #downcast
                }
            }
        }
    }

    /// Returns generated code implementing [`BaseType`], [`BaseSubTypes`],
    /// [`WrappedType`] and [`Fields`] traits for this [GraphQL interface][1].
    ///
    /// [`BaseSubTypes`]: juniper::macros::reflect::BaseSubTypes
    /// [`BaseType`]: juniper::macros::reflect::BaseType
    /// [`Fields`]: juniper::macros::reflect::Fields
    /// [`WrappedType`]: juniper::macros::reflect::WrappedType
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    #[must_use]
    fn impl_reflection_traits_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let implemented_for = &self.implemented_for;
        let implements = &self.implements;
        let scalar = &self.scalar;
        let name = &self.name;
        let fields = self.fields.iter().map(|f| &f.name);

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::macros::reflect::BaseType<#scalar>
                for #ty #ty_generics
                #where_clause
            {
                const NAME: ::juniper::macros::reflect::Type = #name;
            }

            #[automatically_derived]
            impl #impl_generics ::juniper::macros::reflect::BaseSubTypes<#scalar>
                for #ty #ty_generics
                #where_clause
            {
                const NAMES: ::juniper::macros::reflect::Types = &[
                    <Self as ::juniper::macros::reflect::BaseType<#scalar>>::NAME,
                    #( <#implemented_for as ::juniper::macros::reflect::BaseType<#scalar>>::NAME ),*
                ];
            }

            #[automatically_derived]
            impl #impl_generics ::juniper::macros::reflect::Implements<#scalar>
                for #ty #ty_generics
                #where_clause
            {
                const NAMES: ::juniper::macros::reflect::Types =
                    &[#( <#implements as ::juniper::macros::reflect::BaseType<#scalar>>::NAME ),*];
            }

            #[automatically_derived]
            impl #impl_generics ::juniper::macros::reflect::WrappedType<#scalar>
                for #ty #ty_generics
                #where_clause
            {
                const VALUE: ::juniper::macros::reflect::WrappedValue = 1;
            }

            #[automatically_derived]
            impl #impl_generics ::juniper::macros::reflect::Fields<#scalar>
                for #ty #ty_generics
                #where_clause
            {
                const NAMES: ::juniper::macros::reflect::Names = &[#(#fields),*];
            }
        }
    }

    /// Returns generated code implementing [`FieldMeta`] for each field of this
    /// [GraphQL interface][1].
    ///
    /// [`FieldMeta`]: juniper::macros::reflect::FieldMeta
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    fn impl_field_meta_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let context = &self.context;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

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
                    #[automatically_derived]
                    impl #impl_generics ::juniper::macros::reflect::FieldMeta<
                        #scalar,
                        { ::juniper::macros::reflect::fnv1a128(#field_name) }
                    > for #ty #ty_generics #where_clause {
                        type Context = #context;
                        type TypeInfo = ();
                        const TYPE: ::juniper::macros::reflect::Type =
                            <#return_ty as ::juniper::macros::reflect::BaseType<#scalar>>::NAME;
                        const SUB_TYPES: ::juniper::macros::reflect::Types =
                            <#return_ty as ::juniper::macros::reflect::BaseSubTypes<#scalar>>::NAMES;
                        const WRAPPED_VALUE: ::juniper::macros::reflect::WrappedValue =
                            <#return_ty as ::juniper::macros::reflect::WrappedType<#scalar>>::VALUE;
                        const ARGUMENTS: &'static [(
                            ::juniper::macros::reflect::Name,
                            ::juniper::macros::reflect::Type,
                            ::juniper::macros::reflect::WrappedValue,
                        )] = &[#( (
                            #args_names,
                            <#args_tys as ::juniper::macros::reflect::BaseType<#scalar>>::NAME,
                            <#args_tys as ::juniper::macros::reflect::WrappedType<#scalar>>::VALUE,
                        ) ),*];
                    }
                }
            })
            .collect()
    }

    /// Returns generated code implementing [`Field`] trait for each field of
    /// this [GraphQL interface][1].
    ///
    /// [`Field`]: juniper::macros::reflect::Field
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    fn impl_field_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;
        let const_scalar = self.scalar.default_ty();

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let const_implemented_for = self
            .implemented_for
            .iter()
            .cloned()
            .map(|mut impl_for| {
                generics.replace_type_path_with_defaults(&mut impl_for);
                impl_for
            })
            .collect::<Vec<_>>();
        let implemented_for_idents = self
            .implemented_for
            .iter()
            .filter_map(|ty| ty.path.segments.last().map(|seg| &seg.ident))
            .collect::<Vec<_>>();

        self.fields
            .iter()
            .map(|field| {
                let field_name = &field.name;
                let mut return_ty = field.ty.clone();
                generics.replace_type_with_defaults(&mut return_ty);

                let const_ty_generics = self.const_trait_generics();

                let unreachable_arm = (self.implemented_for.is_empty()
                    || !self.generics.params.is_empty())
                .then(|| {
                    quote! { _ => unreachable!() }
                });

                quote_spanned! { field.ident.span() =>
                    #[allow(non_snake_case)]
                    #[automatically_derived]
                    impl #impl_generics ::juniper::macros::reflect::Field<
                        #scalar,
                        { ::juniper::macros::reflect::fnv1a128(#field_name) }
                    > for #ty #ty_generics #where_clause {
                        fn call(
                            &self,
                            info: &Self::TypeInfo,
                            args: &::juniper::Arguments<'_, #scalar>,
                            executor: &::juniper::Executor<'_, '_, Self::Context, #scalar>,
                        ) -> ::juniper::ExecutionResult<#scalar> {
                            match self {
                                #( #ty::#implemented_for_idents(v) => {
                                    ::juniper::assert_field!(
                                        #ty #const_ty_generics,
                                        #const_implemented_for,
                                        #const_scalar,
                                        #field_name,
                                    );

                                    <_ as ::juniper::macros::reflect::Field::<
                                        #scalar,
                                        { ::juniper::macros::reflect::fnv1a128(#field_name) },
                                    >>::call(v, info, args, executor)
                                } )*
                                #unreachable_arm
                            }
                        }
                    }
                }
            })
            .collect()
    }

    /// Returns generated code implementing [`AsyncField`] trait for each field
    /// of this [GraphQL interface][1].
    ///
    /// [`AsyncField`]: juniper::macros::reflect::AsyncField
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    fn impl_async_field_tokens(&self) -> TokenStream {
        let ty = &self.enum_alias_ident;
        let scalar = &self.scalar;
        let const_scalar = self.scalar.default_ty();

        let generics = self.impl_generics(true);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let const_implemented_for = self
            .implemented_for
            .iter()
            .cloned()
            .map(|mut impl_for| {
                generics.replace_type_path_with_defaults(&mut impl_for);
                impl_for
            })
            .collect::<Vec<_>>();
        let implemented_for_idents = self
            .implemented_for
            .iter()
            .filter_map(|ty| ty.path.segments.last().map(|seg| &seg.ident))
            .collect::<Vec<_>>();

        self.fields
            .iter()
            .map(|field| {
                let field_name = &field.name;
                let mut return_ty = field.ty.clone();
                generics.replace_type_with_defaults(&mut return_ty);

                let const_ty_generics = self.const_trait_generics();

                let unreachable_arm = (self.implemented_for.is_empty()
                    || !self.generics.params.is_empty())
                .then(|| {
                    quote! { _ => unreachable!() }
                });

                quote_spanned! { field.ident.span() =>
                    #[allow(non_snake_case)]
                    #[automatically_derived]
                    impl #impl_generics ::juniper::macros::reflect::AsyncField<
                        #scalar,
                        { ::juniper::macros::reflect::fnv1a128(#field_name) }
                    > for #ty #ty_generics #where_clause {
                        fn call<'b>(
                            &'b self,
                            info: &'b Self::TypeInfo,
                            args: &'b ::juniper::Arguments<'_, #scalar>,
                            executor: &'b ::juniper::Executor<'_, '_, Self::Context, #scalar>,
                        ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                            match self {
                                #( #ty::#implemented_for_idents(v) => {
                                    ::juniper::assert_field!(
                                        #ty #const_ty_generics,
                                        #const_implemented_for,
                                        #const_scalar,
                                        #field_name,
                                    );

                                    <_ as ::juniper::macros::reflect::AsyncField<
                                        #scalar,
                                        { ::juniper::macros::reflect::fnv1a128(#field_name) },
                                    >>::call(v, info, args, executor)
                                } )*
                                #unreachable_arm
                            }
                        }
                    }
                }
            })
            .collect()
    }

    /// Returns generated code for the [`GraphQLValue::concrete_type_name`][0]
    /// method, which returns name of the underlying [`implementers`][1] GraphQL
    /// type contained in this enum.
    ///
    /// [0]: juniper::GraphQLValue::concrete_type_name
    /// [1]: Self::implementers
    #[must_use]
    fn method_concrete_type_name_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let match_arms = self
            .implemented_for
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
            (!self.generics.params.is_empty() || self.implemented_for.is_empty()).then(|| {
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
    /// downcasts this enum into its underlying [`implementers`][1] type
    /// asynchronously.
    ///
    /// [0]: juniper::GraphQLValueAsync::resolve_into_type_async
    /// [1]: Self::implementers
    #[must_use]
    fn method_resolve_into_type_async_tokens(&self) -> TokenStream {
        let resolving_code = gen::async_resolving_code(None);

        let match_arms = self.implemented_for.iter().filter_map(|ty| {
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
            (!self.generics.params.is_empty() || self.implemented_for.is_empty()).then(|| {
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
    /// method, which resolves this enum into its underlying
    /// [`implementers`][1] type synchronously.
    ///
    /// [0]: juniper::GraphQLValue::resolve_into_type
    /// [1]: Self::implementers
    #[must_use]
    fn method_resolve_into_type_tokens(&self) -> TokenStream {
        let resolving_code = gen::sync_resolving_code();

        let match_arms = self.implemented_for.iter().filter_map(|ty| {
            ty.path.segments.last().map(|ident| {
                quote! {
                    Self::#ident(res) => #resolving_code,
                }
            })
        });

        let non_exhaustive_match_arm =
            (!self.generics.params.is_empty() || self.implemented_for.is_empty()).then(|| {
                quote! { _ => unreachable!(), }
            });

        quote! {
            match self {
                #( #match_arms )*
                #non_exhaustive_match_arm
            }
        }
    }

    /// Returns trait generics replaced with the default values for usage in a
    /// `const` context.
    #[must_use]
    fn const_trait_generics(&self) -> syn::PathArguments {
        struct GenericsForConst(syn::AngleBracketedGenericArguments);

        impl Visit<'_> for GenericsForConst {
            fn visit_generic_param(&mut self, param: &syn::GenericParam) {
                let arg = match param {
                    syn::GenericParam::Lifetime(_) => parse_quote! { 'static },
                    syn::GenericParam::Type(ty) => {
                        if ty.default.is_none() {
                            parse_quote! { ::juniper::DefaultScalarValue }
                        } else {
                            return;
                        }
                    }
                    syn::GenericParam::Const(c) => {
                        if c.default.is_none() {
                            // This hack works because only `min_const_generics`
                            // are enabled for now.
                            // TODO: Replace this once full `const_generics` are
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
        visitor.visit_generics(&self.generics);
        syn::PathArguments::AngleBracketed(visitor.0)
    }

    /// Returns prepared [`syn::Generics`] for [`GraphQLType`] trait (and
    /// similar) implementation of this enum.
    ///
    /// If `for_async` is `true`, then additional predicates are added to suit
    /// the [`GraphQLAsyncValue`] trait (and similar) requirements.
    ///
    /// [`GraphQLAsyncValue`]: juniper::GraphQLAsyncValue
    /// [`GraphQLType`]: juniper::GraphQLType
    #[must_use]
    fn impl_generics(&self, for_async: bool) -> syn::Generics {
        let mut generics = self.generics.clone();

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
            let self_ty = if self.generics.lifetimes().next().is_some() {
                // Modify lifetime names to omit "lifetime name `'a` shadows a
                // lifetime name that is already in scope" error.
                let mut generics = self.generics.clone();
                for lt in generics.lifetimes_mut() {
                    let ident = lt.lifetime.ident.unraw();
                    lt.lifetime.ident = format_ident!("__fa__{ident}");
                }

                let lifetimes = generics.lifetimes().map(|lt| &lt.lifetime);
                let ty = &self.enum_alias_ident;
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

        generics
    }

    /// Indicates whether this enum has non-exhaustive phantom variant to hold
    /// type parameters.
    #[must_use]
    fn has_phantom_variant(&self) -> bool {
        !self.generics.params.is_empty()
    }
}
