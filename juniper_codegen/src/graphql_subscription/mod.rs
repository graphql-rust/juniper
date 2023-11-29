//! Code generation for [GraphQL subscription][1].
//!
//! [1]: https://spec.graphql.org/October2021#sec-Subscription

pub mod attr;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_quote;

use crate::{common::field, graphql_object::Definition};

/// [GraphQL subscription operation][2] of the [`Definition`] to generate code
/// for.
///
/// [2]: https://spec.graphql.org/October2021#sec-Subscription
struct Subscription;

impl ToTokens for Definition<Subscription> {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.impl_output_type_tokens().to_tokens(into);
        self.impl_graphql_type_tokens().to_tokens(into);
        self.impl_graphql_value_tokens().to_tokens(into);
        self.impl_graphql_subscription_value_tokens()
            .to_tokens(into);
    }
}

impl Definition<Subscription> {
    /// Returns generated code implementing [`GraphQLValue`] trait for this
    /// [GraphQL subscription][1].
    ///
    /// [`GraphQLValue`]: juniper::GraphQLValue
    /// [1]: https://spec.graphql.org/October2021#sec-Subscription
    #[must_use]
    fn impl_graphql_value_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;
        let context = &self.context;

        let (impl_generics, where_clause) = self.impl_generics(false);
        let ty = &self.ty;

        let name = &self.name;

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLValue<#scalar> for #ty #where_clause
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
                    _: &Self::TypeInfo,
                    _: &::core::primitive::str,
                    _: &::juniper::Arguments<'_, #scalar>,
                    _: &::juniper::Executor<'_, '_, Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    ::core::result::Result::Err(::juniper::FieldError::from(
                        "Called `resolve_field` on subscription object",
                    ))
                }

                fn concrete_type_name(
                    &self,
                    _: &Self::Context,
                    _: &Self::TypeInfo,
                ) -> ::std::string::String {
                    #name.into()
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLSubscriptionValue`] trait
    /// for this [GraphQL subscription][1].
    ///
    /// [`GraphQLSubscriptionValue`]: juniper::GraphQLSubscriptionValue
    /// [1]: https://spec.graphql.org/October2021#sec-Subscription
    #[must_use]
    fn impl_graphql_subscription_value_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        // We use `for_async = false` here as `GraphQLSubscriptionValue` requires
        // simpler and less `Send`/`Sync` bounds than `GraphQLValueAsync`.
        let (impl_generics, mut where_clause) = self.impl_generics(false);
        if scalar.is_generic() {
            where_clause = Some(where_clause.unwrap_or_else(|| parse_quote! { where }));
            where_clause
                .as_mut()
                .unwrap()
                .predicates
                .push(parse_quote! { #scalar: ::core::marker::Send + ::core::marker::Sync });
        }
        let ty = &self.ty;
        let ty_name = ty.to_token_stream().to_string();

        let fields_resolvers = self
            .fields
            .iter()
            .map(|f| f.method_resolve_field_into_stream_tokens(scalar));
        let no_field_err =
            field::Definition::method_resolve_field_err_no_field_tokens(scalar, &ty_name);

        quote! {
            #[allow(deprecated)]
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLSubscriptionValue<#scalar> for #ty #where_clause
            {
                fn resolve_field_into_stream<
                    's, 'i, 'fi, 'args, 'e, 'ref_e, 'res, 'f,
                >(
                    &'s self,
                    info: &'i Self::TypeInfo,
                    field: &'fi ::core::primitive::str,
                    args: ::juniper::Arguments<'args, #scalar>,
                    executor: &'ref_e ::juniper::Executor<'ref_e, 'e, Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'f, std::result::Result<
                    ::juniper::Value<::juniper::ValuesStream<'res, #scalar>>,
                    ::juniper::FieldError<#scalar>,
                >>
                where
                    's: 'f,
                    'fi: 'f,
                    'args: 'f,
                    'ref_e: 'f,
                    'res: 'f,
                    'i: 'res,
                    'e: 'res,
                {
                    match field {
                        #( #fields_resolvers )*
                        _ => ::std::boxed::Box::pin(async move { #no_field_err }),
                    }
                }
            }
        }
    }
}
