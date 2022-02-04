//! Code generation for `#[derive(GraphQLScalar)]` macro.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse_quote, spanned::Spanned};

use crate::{common::scalar, result::GraphQLScope};

use super::{Attr, Definition, Field, GraphQLScalarMethods, ParseToken, TypeOrIdent};

/// [`GraphQLScope`] of errors for `#[derive(GraphQLScalar)]` macro.
const ERR: GraphQLScope = GraphQLScope::DeriveScalar;

/// Expands `#[derive(GraphQLScalar)]` macro into generated code.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;

    let attr = Attr::from_attrs("graphql", &ast.attrs)?;

    let field = match (
        attr.to_output.as_deref().cloned(),
        attr.from_input.as_deref().cloned(),
        attr.from_input_err.as_deref().cloned(),
        attr.parse_token.as_deref().cloned(),
        attr.with.as_deref().cloned(),
    ) {
        (Some(to_output), Some(from_input), Some(from_input_err), Some(parse_token), None) => {
            GraphQLScalarMethods::Custom {
                to_output,
                from_input: (from_input, from_input_err),
                parse_token,
            }
        }
        (to_output, from_input, from_input_err, parse_token, Some(module)) => {
            GraphQLScalarMethods::Custom {
                to_output: to_output.unwrap_or_else(|| parse_quote! { #module::to_output }),
                from_input: (
                    from_input.unwrap_or_else(|| parse_quote! { #module::from_input }),
                    from_input_err.unwrap_or_else(|| parse_quote! { #module::Error }),
                ),
                parse_token: parse_token
                    .unwrap_or_else(|| ParseToken::Custom(parse_quote! { #module::parse_token })),
            }
        }
        (to_output, from_input, from_input_err, parse_token, None) => {
            let from_input = match (from_input, from_input_err) {
                (Some(from_input), Some(err)) => Some((from_input, err)),
                (None, None) => None,
                _ => {
                    return Err(ERR.custom_error(
                        ast.span(),
                        "`from_input_with` attribute should be provided in \
                         tandem with `from_input_err`",
                    ))
                }
            };

            let data = if let syn::Data::Struct(data) = &ast.data {
                data
            } else {
                return Err(ERR.custom_error(
                    ast.span(),
                    "expected all custom resolvers or single-field struct",
                ));
            };
            let field = match &data.fields {
                syn::Fields::Unit => Err(ERR.custom_error(
                    ast.span(),
                    "expected exactly 1 field, e.g.: `Test(i32)`, `Test { test: i32 }` \
                     or all custom resolvers",
                )),
                syn::Fields::Unnamed(fields) => fields
                    .unnamed
                    .first()
                    .and_then(|f| (fields.unnamed.len() == 1).then(|| Field::Unnamed(f.clone())))
                    .ok_or_else(|| {
                        ERR.custom_error(
                            ast.span(),
                            "expected exactly 1 field, e.g., Test(i32) \
                             or all custom resolvers",
                        )
                    }),
                syn::Fields::Named(fields) => fields
                    .named
                    .first()
                    .and_then(|f| (fields.named.len() == 1).then(|| Field::Named(f.clone())))
                    .ok_or_else(|| {
                        ERR.custom_error(
                            ast.span(),
                            "expected exactly 1 field, e.g., Test { test: i32 } \
                             or all custom resolvers",
                        )
                    }),
            }?;
            GraphQLScalarMethods::Delegated {
                to_output,
                from_input,
                parse_token,
                field,
            }
        }
    };

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    Ok(Definition {
        ty: TypeOrIdent::Ident(ast.ident.clone()),
        where_clause: attr
            .where_clause
            .map_or_else(|| Vec::new(), |cl| cl.into_inner()),
        generics: ast.generics.clone(),
        methods: field,
        name: attr
            .name
            .as_deref()
            .cloned()
            .unwrap_or_else(|| ast.ident.to_string()),
        description: attr.description.as_deref().cloned(),
        specified_by_url: attr.specified_by_url.as_deref().cloned(),
        scalar,
    }
    .to_token_stream())
}

// /// Definition of [GraphQL scalar][1] for code generation.
// ///
// /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
// struct Definition {
//     /// Name of this [GraphQL scalar][1] in GraphQL schema.
//     ///
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     name: String,
//
//     /// Rust type [`Ident`] that this [GraphQL scalar][1] is represented with.
//     ///
//     /// [`Ident`]: syn::Ident
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     ident: syn::Ident,
//
//     /// Generics of the Rust type that this [GraphQL scalar][1] is implemented
//     /// for.
//     ///
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     generics: syn::Generics,
//
//     /// [`GraphQLScalarDefinition`] representing [GraphQL scalar][1].
//     ///
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     methods: GraphQLScalarMethods,
//
//     /// Description of this [GraphQL scalar][1] to put into GraphQL schema.
//     ///
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     description: Option<String>,
//
//     /// Spec [`Url`] of this [GraphQL scalar][1] to put into GraphQL schema.
//     ///
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     specified_by_url: Option<Url>,
//
//     /// [`ScalarValue`] parametrization to generate [`GraphQLType`]
//     /// implementation with for this [GraphQL scalar][1].
//     ///
//     /// [`GraphQLType`]: juniper::GraphQLType
//     /// [`ScalarValue`]: juniper::ScalarValue
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     scalar: scalar::Type,
// }
//
// impl ToTokens for Definition {
//     fn to_tokens(&self, into: &mut TokenStream) {
//         self.impl_output_and_input_type_tokens().to_tokens(into);
//         self.impl_type_tokens().to_tokens(into);
//         self.impl_value_tokens().to_tokens(into);
//         self.impl_value_async_tokens().to_tokens(into);
//         self.impl_to_input_value_tokens().to_tokens(into);
//         self.impl_from_input_value_tokens().to_tokens(into);
//         self.impl_parse_scalar_value_tokens().to_tokens(into);
//         self.impl_graphql_scalar_tokens().to_tokens(into);
//         self.impl_reflection_traits_tokens().to_tokens(into);
//     }
// }
//
// impl Definition {
//     /// Returns generated code implementing [`marker::IsInputType`] and
//     /// [`marker::IsOutputType`] trait for this [GraphQL scalar][1].
//     ///
//     /// [`marker::IsInputType`]: juniper::marker::IsInputType
//     /// [`marker::IsOutputType`]: juniper::marker::IsOutputType
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     #[must_use]
//     fn impl_output_and_input_type_tokens(&self) -> TokenStream {
//         let ident = &self.ident;
//         let scalar = &self.scalar;
//
//         let generics = self.impl_generics(false);
//         let (impl_gens, _, where_clause) = generics.split_for_impl();
//         let (_, ty_gens, _) = self.generics.split_for_impl();
//
//         quote! {
//             #[automatically_derived]
//             impl#impl_gens ::juniper::marker::IsInputType<#scalar> for #ident#ty_gens
//                 #where_clause { }
//
//             #[automatically_derived]
//             impl#impl_gens ::juniper::marker::IsOutputType<#scalar> for #ident#ty_gens
//                 #where_clause { }
//         }
//     }
//
//     /// Returns generated code implementing [`GraphQLType`] trait for this
//     /// [GraphQL scalar][1].
//     ///
//     /// [`GraphQLType`]: juniper::GraphQLType
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     fn impl_type_tokens(&self) -> TokenStream {
//         let ident = &self.ident;
//         let scalar = &self.scalar;
//         let name = &self.name;
//
//         let description = self
//             .description
//             .as_ref()
//             .map(|val| quote! { .description(#val) });
//         let specified_by_url = self.specified_by_url.as_ref().map(|url| {
//             let url_lit = url.as_str();
//             quote! { .specified_by_url(#url_lit) }
//         });
//
//         let generics = self.impl_generics(false);
//         let (impl_gens, _, where_clause) = generics.split_for_impl();
//         let (_, ty_gens, _) = self.generics.split_for_impl();
//
//         quote! {
//             #[automatically_derived]
//             impl#impl_gens ::juniper::GraphQLType<#scalar> for #ident#ty_gens
//                 #where_clause
//             {
//                 fn name(_: &Self::TypeInfo) -> Option<&'static str> {
//                     Some(#name)
//                 }
//
//                 fn meta<'r>(
//                     info: &Self::TypeInfo,
//                     registry: &mut ::juniper::Registry<'r, #scalar>,
//                 ) -> ::juniper::meta::MetaType<'r, #scalar>
//                 where
//                     #scalar: 'r,
//                 {
//                     registry.build_scalar_type::<Self>(info)
//                         #description
//                         #specified_by_url
//                         .into_meta()
//                 }
//             }
//         }
//     }
//
//     /// Returns generated code implementing [`GraphQLValue`] trait for this
//     /// [GraphQL scalar][1].
//     ///
//     /// [`GraphQLValue`]: juniper::GraphQLValue
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     fn impl_value_tokens(&self) -> TokenStream {
//         let ident = &self.ident;
//         let scalar = &self.scalar;
//
//         let resolve = self.methods.expand_resolve(scalar);
//
//         let generics = self.impl_generics(false);
//         let (impl_gens, _, where_clause) = generics.split_for_impl();
//         let (_, ty_gens, _) = self.generics.split_for_impl();
//
//         quote! {
//             #[automatically_derived]
//             impl#impl_gens ::juniper::GraphQLValue<#scalar> for #ident#ty_gens
//                 #where_clause
//             {
//                 type Context = ();
//                 type TypeInfo = ();
//
//                 fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
//                     <Self as ::juniper::GraphQLType<#scalar>>::name(info)
//                 }
//
//                 fn resolve(
//                     &self,
//                     info: &(),
//                     selection: Option<&[::juniper::Selection<#scalar>]>,
//                     executor: &::juniper::Executor<Self::Context, #scalar>,
//                 ) -> ::juniper::ExecutionResult<#scalar> {
//                     #resolve
//                 }
//             }
//         }
//     }
//
//     /// Returns generated code implementing [`GraphQLValueAsync`] trait for this
//     /// [GraphQL scalar][1].
//     ///
//     /// [`GraphQLValueAsync`]: juniper::GraphQLValueAsync
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     fn impl_value_async_tokens(&self) -> TokenStream {
//         let ident = &self.ident;
//         let scalar = &self.scalar;
//
//         let generics = self.impl_generics(true);
//         let (impl_gens, _, where_clause) = generics.split_for_impl();
//         let (_, ty_gens, _) = self.generics.split_for_impl();
//
//         quote! {
//             #[automatically_derived]
//             impl#impl_gens ::juniper::GraphQLValueAsync<#scalar> for #ident#ty_gens
//                 #where_clause
//             {
//                 fn resolve_async<'b>(
//                     &'b self,
//                     info: &'b Self::TypeInfo,
//                     selection_set: Option<&'b [::juniper::Selection<#scalar>]>,
//                     executor: &'b ::juniper::Executor<Self::Context, #scalar>,
//                 ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
//                     use ::juniper::futures::future;
//                     let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
//                     Box::pin(future::ready(v))
//                 }
//             }
//         }
//     }
//
//     /// Returns generated code implementing [`InputValue`] trait for this
//     /// [GraphQL scalar][1].
//     ///
//     /// [`InputValue`]: juniper::InputValue
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     fn impl_to_input_value_tokens(&self) -> TokenStream {
//         let ident = &self.ident;
//         let scalar = &self.scalar;
//
//         let to_input_value = self.methods.expand_to_input_value(scalar);
//
//         let generics = self.impl_generics(false);
//         let (impl_gens, _, where_clause) = generics.split_for_impl();
//         let (_, ty_gens, _) = self.generics.split_for_impl();
//
//         quote! {
//             #[automatically_derived]
//             impl#impl_gens ::juniper::ToInputValue<#scalar> for #ident#ty_gens
//                 #where_clause
//             {
//                 fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
//                     #to_input_value
//                 }
//             }
//         }
//     }
//
//     /// Returns generated code implementing [`FromInputValue`] trait for this
//     /// [GraphQL scalar][1].
//     ///
//     /// [`FromInputValue`]: juniper::FromInputValue
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     fn impl_from_input_value_tokens(&self) -> TokenStream {
//         let ident = &self.ident;
//         let scalar = &self.scalar;
//
//         let error_ty = self.methods.expand_from_input_err(scalar);
//         let from_input_value = self.methods.expand_from_input(scalar);
//
//         let generics = self.impl_generics(false);
//         let (impl_gens, _, where_clause) = generics.split_for_impl();
//         let (_, ty_gens, _) = self.generics.split_for_impl();
//
//         quote! {
//             #[automatically_derived]
//             impl#impl_gens ::juniper::FromInputValue<#scalar> for #ident#ty_gens
//                 #where_clause
//             {
//                 type Error = #error_ty;
//
//                 fn from_input_value(input: &::juniper::InputValue<#scalar>) -> Result<Self, Self::Error> {
//                    #from_input_value
//                 }
//             }
//         }
//     }
//
//     /// Returns generated code implementing [`ParseScalarValue`] trait for this
//     /// [GraphQL scalar][1].
//     ///
//     /// [`ParseScalarValue`]: juniper::ParseScalarValue
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     fn impl_parse_scalar_value_tokens(&self) -> TokenStream {
//         let ident = &self.ident;
//         let scalar = &self.scalar;
//
//         let from_str = self.methods.expand_parse_token(scalar);
//
//         let generics = self.impl_generics(false);
//         let (impl_gens, _, where_clause) = generics.split_for_impl();
//         let (_, ty_gens, _) = self.generics.split_for_impl();
//
//         quote! {
//             #[automatically_derived]
//             impl#impl_gens ::juniper::ParseScalarValue<#scalar> for #ident#ty_gens
//                 #where_clause
//            {
//                fn from_str(
//                     token: ::juniper::parser::ScalarToken<'_>,
//                ) -> ::juniper::ParseScalarResult<'_, #scalar> {
//                     #from_str
//                 }
//             }
//         }
//     }
//
//     /// Returns generated code implementing [`GraphQLScalar`] trait for this
//     /// [GraphQL scalar][1].
//     ///
//     /// [`GraphQLScalar`]: juniper::GraphQLScalar
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     fn impl_graphql_scalar_tokens(&self) -> TokenStream {
//         let ident = &self.ident;
//         let scalar = &self.scalar;
//
//         let generics = self.impl_generics(false);
//         let (impl_gens, _, where_clause) = generics.split_for_impl();
//         let (_, ty_gens, _) = self.generics.split_for_impl();
//
//         let to_output = self.methods.expand_to_output(scalar);
//         let from_input_err = self.methods.expand_from_input_err(scalar);
//         let from_input = self.methods.expand_from_input(scalar);
//         let parse_token = self.methods.expand_parse_token(scalar);
//
//         quote! {
//             #[automatically_derived]
//             impl#impl_gens ::juniper::GraphQLScalar<#scalar> for #ident#ty_gens
//                 #where_clause
//             {
//                 type Error = #from_input_err;
//
//                 fn to_output(&self) -> ::juniper::Value<#scalar> {
//                     #to_output
//                 }
//
//                 fn from_input(
//                     input: &::juniper::InputValue<#scalar>
//                 ) -> Result<Self, Self::Error> {
//                     #from_input
//                 }
//
//                 fn parse_token(
//                     token: ::juniper::ScalarToken<'_>
//                 ) -> ::juniper::ParseScalarResult<'_, #scalar> {
//                     #parse_token
//                 }
//             }
//         }
//     }
//
//     /// Returns generated code implementing [`BaseType`], [`BaseSubTypes`] and
//     /// [`WrappedType`] traits for this [GraphQL scalar][1].
//     ///
//     /// [`BaseSubTypes`]: juniper::macros::reflection::BaseSubTypes
//     /// [`BaseType`]: juniper::macros::reflection::BaseType
//     /// [`WrappedType`]: juniper::macros::reflection::WrappedType
//     /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
//     fn impl_reflection_traits_tokens(&self) -> TokenStream {
//         let ident = &self.ident;
//         let scalar = &self.scalar;
//         let name = &self.name;
//
//         let generics = self.impl_generics(false);
//         let (impl_gens, _, where_clause) = generics.split_for_impl();
//         let (_, ty_gens, _) = self.generics.split_for_impl();
//
//         quote! {
//             #[automatically_derived]
//             impl#impl_gens ::juniper::macros::reflect::BaseType<#scalar> for #ident#ty_gens
//                 #where_clause
//             {
//                 const NAME: ::juniper::macros::reflect::Type = #name;
//             }
//
//             #[automatically_derived]
//             impl#impl_gens ::juniper::macros::reflect::BaseSubTypes<#scalar> for #ident#ty_gens
//                 #where_clause
//             {
//                 const NAMES: ::juniper::macros::reflect::Types =
//                     &[<Self as ::juniper::macros::reflect::BaseType<#scalar>>::NAME];
//             }
//
//             #[automatically_derived]
//             impl#impl_gens ::juniper::macros::reflect::WrappedType<#scalar> for #ident#ty_gens
//                 #where_clause
//             {
//                 const VALUE: ::juniper::macros::reflect::WrappedValue = 1;
//             }
//         }
//     }
//
//     /// Returns prepared [`syn::Generics`] for [`GraphQLType`] trait (and
//     /// similar) implementation of this enum.
//     ///
//     /// If `for_async` is `true`, then additional predicates are added to suit
//     /// the [`GraphQLAsyncValue`] trait (and similar) requirements.
//     ///
//     /// [`GraphQLAsyncValue`]: juniper::GraphQLAsyncValue
//     /// [`GraphQLType`]: juniper::GraphQLType
//     #[must_use]
//     fn impl_generics(&self, for_async: bool) -> syn::Generics {
//         let mut generics = self.generics.clone();
//
//         let scalar = &self.scalar;
//         if scalar.is_implicit_generic() {
//             generics.params.push(parse_quote! { #scalar });
//         }
//         if scalar.is_generic() {
//             generics
//                 .make_where_clause()
//                 .predicates
//                 .push(parse_quote! { #scalar: ::juniper::ScalarValue });
//         }
//         if let Some(bound) = scalar.bounds() {
//             generics.make_where_clause().predicates.push(bound);
//         }
//
//         if for_async {
//             let self_ty = if self.generics.lifetimes().next().is_some() {
//                 // Modify lifetime names to omit "lifetime name `'a` shadows a
//                 // lifetime name that is already in scope" error.
//                 let mut generics = self.generics.clone();
//                 for lt in generics.lifetimes_mut() {
//                     let ident = lt.lifetime.ident.unraw();
//                     lt.lifetime.ident = format_ident!("__fa__{}", ident);
//                 }
//
//                 let lifetimes = generics.lifetimes().map(|lt| &lt.lifetime);
//                 let ty = &self.ident;
//                 let (_, ty_generics, _) = generics.split_for_impl();
//
//                 quote! { for<#( #lifetimes ),*> #ty#ty_generics }
//             } else {
//                 quote! { Self }
//             };
//             generics
//                 .make_where_clause()
//                 .predicates
//                 .push(parse_quote! { #self_ty: Sync });
//
//             if scalar.is_generic() {
//                 generics
//                     .make_where_clause()
//                     .predicates
//                     .push(parse_quote! { #scalar: Send + Sync });
//             }
//         }
//
//         generics
//     }
// }
