#![allow(clippy::collapsible_if)]

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    token,
};
use url::Url;

use crate::{
    common::{
        parse::{
            self,
            attr::{err, OptionExt as _},
            ParseBufferExt as _,
        },
        scalar,
    },
    result::GraphQLScope,
    util::{filter_attrs, get_doc_comment, span_container::SpanContainer, DeprecationAttr},
};

/// [`GraphQLScope`] of errors for `#[graphql_interface]` macro.
const ERR: GraphQLScope = GraphQLScope::ImplScalar;

/// Expands `#[graphql_interface]` macro into generated code.
pub(crate) fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        let attrs = parse::attr::unite(("graphql_scalar", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip("graphql_scalar", ast.attrs);
        return expand_on_impl_block(attrs, ast);
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_scalar] attribute is applicable to impl trait only",
    ))
}

fn expand_on_impl_block(
    attrs: Vec<syn::Attribute>,
    ast: syn::ItemImpl,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs("graphql_scalar", &attrs)?;

    let mut self_ty = ast.self_ty.clone();
    if let syn::Type::Group(group) = self_ty.as_ref() {
        self_ty = group.elem.clone();
    }

    let name = attr
        .name
        .map(SpanContainer::into_inner)
        .or_else(|| {
            if let syn::Type::Path(path) = self_ty.as_ref() {
                path.path
                    .segments
                    .last()
                    .map(|last| last.ident.unraw().to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            ERR.custom_error(
                self_ty.span(),
                "unable to find target for implementation target for `GraphQLScalar`",
            )
        })?;

    let (_, trait_ty, _) = ast.trait_.as_ref().ok_or_else(|| {
        ERR.custom_error(
            ast.impl_token.span(),
            "expected GraphQLScalar trait implementation",
        )
    })?;

    let get_scalar = || {
        if let Some(last_seg) = trait_ty.segments.last() {
            match &last_seg.arguments {
                syn::PathArguments::AngleBracketed(gens) => {
                    if let Some(syn::GenericArgument::Type(ty)) = gens.args.last() {
                        let is_generic = ast
                            .generics
                            .params
                            .iter()
                            .filter_map(|par| match par {
                                syn::GenericParam::Type(ty) => Some(&ty.ident),
                                _ => None,
                            })
                            .find(|gen_par| {
                                gen_par.to_string() == ty.to_token_stream().to_string()
                            });

                        return is_generic.map_or_else(
                            || scalar::Type::Concrete(ty.clone()),
                            |scalar| scalar::Type::ExplicitGeneric(scalar.clone()),
                        );
                    }
                }
                syn::PathArguments::None | syn::PathArguments::Parenthesized(_) => {}
            }
        }
        scalar::Type::Concrete(parse_quote! { ::juniper::DefaultScalarValue })
    };
    let scalar = get_scalar();

    let mut out = ast.to_token_stream();
    Definition {
        impl_for_type: *ast.self_ty.clone(),
        generics: ast.generics.clone(),
        name,
        description: attr.description.as_deref().cloned(),
        scalar,
        specified_by_url: attr.specified_by_url.as_deref().cloned(),
    }
    .to_tokens(&mut out);

    Ok(out)
}

#[derive(Default)]
struct Attr {
    pub name: Option<SpanContainer<String>>,
    pub description: Option<SpanContainer<String>>,
    pub deprecation: Option<SpanContainer<DeprecationAttr>>,
    pub specified_by_url: Option<SpanContainer<Url>>,
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
                    let desc = input.parse::<syn::LitStr>()?;
                    out.description
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(desc.span()),
                            desc.value(),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "specified_by_url" => {
                    input.parse::<token::Eq>()?;
                    let lit = input.parse::<syn::LitStr>()?;
                    let url = lit.value().parse::<Url>().map_err(|err| {
                        syn::Error::new(lit.span(), format!("Invalid URL: {}", err))
                    })?;
                    out.specified_by_url
                        .replace(SpanContainer::new(ident.span(), Some(lit.span()), url))
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

impl Attr {
    /// Tries to merge two [`Attr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            deprecation: try_merge_opt!(deprecation: self, another),
            specified_by_url: try_merge_opt!(specified_by_url: self, another),
        })
    }

    /// Parses [`Attr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a trait definition.
    fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut attr = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if attr.description.is_none() {
            attr.description = get_doc_comment(attrs);
        }

        Ok(attr)
    }
}

pub struct Definition {
    impl_for_type: syn::Type,
    generics: syn::Generics,
    name: String,
    scalar: scalar::Type,
    description: Option<String>,
    specified_by_url: Option<Url>,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.impl_output_and_input_type_tokens().to_tokens(into);
        self.impl_type_tokens().to_tokens(into);
        self.impl_value_tokens().to_tokens(into);
        self.impl_value_async().to_tokens(into);
        self.impl_to_input_value_tokens().to_tokens(into);
        self.impl_from_input_value_tokens().to_tokens(into);
        self.impl_parse_scalar_value_tokens().to_tokens(into);
        self.impl_traits_for_reflection_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Returns generated code implementing [`marker::IsOutputType`] trait for
    /// this [GraphQL interface][1].
    ///
    /// [`marker::IsOutputType`]: juniper::marker::IsOutputType
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    #[must_use]
    fn impl_output_and_input_type_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::marker::IsInputType<#scalar> for #ty
                #where_clause { }

            impl#impl_gens ::juniper::marker::IsOutputType<#scalar> for #ty
                #where_clause { }
        }
    }

    fn impl_type_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let name = &self.name;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        let description = self
            .description
            .as_ref()
            .map(|val| quote! { .description(#val) });
        let specified_by_url = self.specified_by_url.as_ref().map(|url| {
            let url_lit = url.as_str();
            quote! { .specified_by_url(#url_lit) }
        });

        quote! {
            impl#impl_gens ::juniper::GraphQLType<#scalar> for #ty
                #where_clause
            {
                fn name(_: &Self::TypeInfo) -> Option<&'static str> {
                    Some(#name)
                }

                fn meta<'__registry>(
                    info: &Self::TypeInfo,
                    registry: &mut ::juniper::Registry<'__registry, #scalar>,
                ) -> ::juniper::meta::MetaType<'__registry, #scalar>
                where
                    #scalar: '__registry,
                {
                    registry.build_scalar_type::<Self>(info)
                        #description
                        #specified_by_url
                        .into_meta()
                }
            }
        }
    }

    fn impl_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::GraphQLValue<#scalar> for #ty
                #where_clause
            {
                type Context = ();
                type TypeInfo = ();

                fn type_name<'__i>(&self, info: &'__i Self::TypeInfo) -> Option<&'__i str> {
                    <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                }

                fn resolve(
                    &self,
                    info: &(),
                    selection: Option<&[::juniper::Selection<#scalar>]>,
                    executor: &::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    Ok(::juniper::GraphQLScalar::resolve(self))
                }
            }
        }
    }

    fn impl_value_async(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(true);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::GraphQLValueAsync<#scalar> for #ty
                #where_clause
            {
                fn resolve_async<'__l>(
                    &'__l self,
                    info: &'__l Self::TypeInfo,
                    selection_set: Option<&'__l [::juniper::Selection<#scalar>]>,
                    executor: &'__l ::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'__l, ::juniper::ExecutionResult<#scalar>> {
                    use ::juniper::futures::future;
                    let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
                    Box::pin(future::ready(v))
                }
            }
        }
    }

    fn impl_to_input_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::ToInputValue<#scalar> for #ty
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    let v = ::juniper::GraphQLScalar::resolve(self);
                    ::juniper::ToInputValue::to_input_value(&v)
                }
            }
        }
    }

    fn impl_from_input_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::FromInputValue<#scalar> for #ty
                #where_clause
            {
                type Error = <Self as ::juniper::GraphQLScalar<#scalar>>::Error;

                fn from_input_value(input: &::juniper::InputValue<#scalar>) -> Result<Self, Self::Error> {
                    ::juniper::GraphQLScalar::from_input_value(input)
                }
            }
        }
    }

    fn impl_parse_scalar_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::ParseScalarValue<#scalar> for #ty
                #where_clause
           {
               fn from_str(
                    token: ::juniper::parser::ScalarToken,
               ) -> ::juniper::ParseScalarResult<#scalar> {
                    <Self as ::juniper::GraphQLScalar<#scalar>>::from_str(token)
                }
            }
        }
    }

    fn impl_traits_for_reflection_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;
        let name = &self.name;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::macros::reflection::BaseType<#scalar> for #ty
                #where_clause
            {
                const NAME: ::juniper::macros::reflection::Type = #name;
            }

            impl#impl_gens ::juniper::macros::reflection::BaseSubTypes<#scalar> for #ty
                #where_clause
            {
                const NAMES: ::juniper::macros::reflection::Types =
                    &[<Self as ::juniper::macros::reflection::BaseType<#scalar>>::NAME];
            }

            impl#impl_gens ::juniper::macros::reflection::WrappedType<#scalar> for #ty
                #where_clause
            {
                const VALUE: ::juniper::macros::reflection::WrappedValue = 1;
            }
        }
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
                    lt.lifetime.ident = format_ident!("__fa__{}", ident);
                }

                let lifetimes = generics.lifetimes().map(|lt| &lt.lifetime);
                let ty = &self.impl_for_type;
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
}
