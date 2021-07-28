//! Code generation for [GraphQL object][1].
//!
//! [1]: https://spec.graphql.org/June2018/#sec-Objects

pub mod attr;
pub mod derive;

use std::{collections::HashSet, convert::TryInto as _};

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
    token,
};

use crate::{
    common::{
        field,
        parse::{
            attr::{err, OptionExt as _},
            ParseBufferExt as _,
        },
        ScalarValueType,
    },
    util::{filter_attrs, get_doc_comment, span_container::SpanContainer, RenameRule},
};

/// Available arguments behind `#[graphql]` (or `#[graphql_object]`) attribute
/// when generating code for [GraphQL object][1] type.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Objects
#[derive(Debug, Default)]
struct Attr {
    /// Explicitly specified name of this [GraphQL object][1] type.
    ///
    /// If [`None`], then Rust type name is used by default.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of this [GraphQL object][1] type.
    ///
    /// If [`None`], then Rust doc comment is used as [description][2], if any.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    description: Option<SpanContainer<String>>,

    /// Explicitly specified type of `juniper::Context` to use for resolving
    /// this [GraphQL object][1] type with.
    ///
    /// If [`None`], then unit type `()` is assumed as a type of
    /// `juniper::Context`.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type of `juniper::ScalarValue` to use for resolving
    /// this [GraphQL object][1] type with.
    ///
    /// If [`None`], then generated code will be generic over any
    /// `juniper::ScalarValue` type, which, in turn, requires all [object][1]
    /// fields to be generic over any `juniper::ScalarValue` type too. That's
    /// why this type should be specified only if one of the variants implements
    /// `juniper::GraphQLType` in a non-generic way over `juniper::ScalarValue`
    /// type.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    scalar: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified [GraphQL interfaces][2] this [GraphQL object][1]
    /// type implements.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    /// [2]: https://spec.graphql.org/June2018/#sec-Interfaces
    interfaces: HashSet<SpanContainer<syn::Type>>,

    /// Explicitly specified [`RenameRule`] for all fields of this
    /// [GraphQL object][1] type.
    ///
    /// If [`None`] then the default rule will be [`RenameRule::CamelCase`].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    rename_fields: Option<SpanContainer<RenameRule>>,

    /// Indicator whether the generated code is intended to be used only inside
    /// the [`juniper`] library.
    is_internal: bool,
}

impl Parse for Attr {
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
                "impl" | "implements" | "interfaces" => {
                    input.parse::<token::Eq>()?;
                    for iface in input.parse_maybe_wrapped_and_punctuated::<
                        syn::Type, token::Bracket, token::Comma,
                    >()? {
                        let iface_span = iface.span();
                        output
                            .interfaces
                            .replace(SpanContainer::new(ident.span(), Some(iface_span), iface))
                            .none_or_else(|_| err::dup_arg(iface_span))?;
                    }
                }
                "rename_all" => {
                    input.parse::<token::Eq>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    output
                        .rename_fields
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(val.span()),
                            val.try_into()?,
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?;
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

impl Attr {
    /// Tries to merge two [`Attr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            context: try_merge_opt!(context: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            interfaces: try_merge_hashset!(interfaces: self, another => span_joined),
            rename_fields: try_merge_opt!(rename_fields: self, another),
            is_internal: self.is_internal || another.is_internal,
        })
    }

    /// Parses [`Attr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a struct or impl block definition.
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

#[derive(Debug)]
struct Definition {
    /// Name of this [GraphQL object][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    name: String,

    /// Rust type that this [GraphQL object][1] is represented with.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    ty: syn::Type,

    /// Generics of the Rust type that this [GraphQL object][1] is implemented
    /// for.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    generics: syn::Generics,

    /// Description of this [GraphQL object][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    description: Option<String>,

    /// Rust type of `juniper::Context` to generate `juniper::GraphQLType`
    /// implementation with for this [GraphQL object][1].
    ///
    /// If [`None`] then generated code will use unit type `()` as
    /// `juniper::Context`.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    context: Option<syn::Type>,

    /// [`ScalarValue`] parametrization to generate [`GraphQLType`]
    /// implementation with for this [GraphQL object][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    scalar: ScalarValueType,

    /// Defined [GraphQL fields][2] of this [GraphQL object][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    /// [2]: https://spec.graphql.org/June2018/#sec-Language.Fields
    fields: Vec<field::Definition>,

    /// [GraphQL interfaces][2] implemented by this [GraphQL object][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    /// [2]: https://spec.graphql.org/June2018/#sec-Interfaces
    interfaces: HashSet<syn::Type>,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.impl_graphql_object_tokens().to_tokens(into);
        self.impl_output_type_tokens().to_tokens(into);
        self.impl_graphql_type_tokens().to_tokens(into);
        self.impl_graphql_value_tokens().to_tokens(into);
        self.impl_graphql_value_async_tokens().to_tokens(into);
        self.impl_as_dyn_graphql_value_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Returns prepared [`syn::Generics::split_for_impl`] for [`GraphQLType`]
    /// trait (and similar) implementation of this [GraphQL object][1].
    ///
    /// If `for_async` is `true`, then additional predicates are added to suit
    /// the [`GraphQLAsyncValue`] trait (and similar) requirements.
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    #[must_use]
    fn impl_generics(
        &self,
        for_async: bool,
    ) -> (TokenStream, TokenStream, Option<syn::WhereClause>) {
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let mut generics = self.generics.clone();

        let scalar = &self.scalar;
        if self.scalar.is_implicit_generic() {
            generics.params.push(parse_quote! { #scalar });
        }
        if scalar.is_generic() {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { #scalar: ::juniper::ScalarValue });
        }

        if for_async {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { Self: Sync });
            if scalar.is_generic() {
                generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote! { #scalar: Send + Sync });
            }
        }

        let (impl_generics, _, where_clause) = generics.split_for_impl();
        (
            quote! { #impl_generics },
            quote! { #ty_generics },
            where_clause.cloned(),
        )
    }

    /// Returns generated code implementing [`GraphQLObject`] trait for this
    /// [GraphQL object][1].
    ///
    /// [`GraphQLObject`]: juniper::GraphQLObject
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    #[must_use]
    fn impl_graphql_object_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_generics, where_clause) = self.impl_generics(false);
        let ty = &self.ty;

        let interface_tys = self.interfaces.iter();
        // TODO: Make it work by repeating `sa::assert_type_ne_all!` expansion,
        //       but considering generics.
        //let interface_tys: Vec<_> = self.interfaces.iter().collect();
        //let all_interfaces_unique = (interface_tys.len() > 1).then(|| {
        //    quote! { ::juniper::sa::assert_type_ne_all!(#( #interface_tys ),*); }
        //});

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::marker::GraphQLObject<#scalar> for #ty#ty_generics #where_clause
            {
                fn mark() {
                    #( <#interface_tys as ::juniper::marker::GraphQLInterface<#scalar>>::mark(); )*
                }
            }
        }
    }

    /// Returns generated code implementing [`marker::IsOutputType`] trait for
    /// this [GraphQL object][1].
    ///
    /// [`marker::IsOutputType`]: juniper::marker::IsOutputType
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    #[must_use]
    fn impl_output_type_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_generics, where_clause) = self.impl_generics(false);
        let ty = &self.ty;

        let fields_marks = self.fields.iter().map(|f| f.method_mark_tokens(scalar));

        let interface_tys = self.interfaces.iter();

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::marker::IsOutputType<#scalar> for #ty#ty_generics #where_clause
            {
                fn mark() {
                    #( #fields_marks )*
                    #( <#interface_tys as ::juniper::marker::IsOutputType<#scalar>>::mark(); )*
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL object][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    #[must_use]
    fn impl_graphql_type_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_generics, where_clause) = self.impl_generics(false);
        let ty = &self.ty;

        let name = &self.name;
        let description = self
            .description
            .as_ref()
            .map(|desc| quote! { .description(#desc) });

        let fields_meta = self
            .fields
            .iter()
            .map(field::Definition::method_meta_tokens);

        // Sorting is required to preserve/guarantee the order of interfaces registered in schema.
        let mut interface_tys: Vec<_> = self.interfaces.iter().collect();
        interface_tys.sort_unstable_by(|a, b| {
            let (a, b) = (quote!(#a).to_string(), quote!(#b).to_string());
            a.cmp(&b)
        });
        let interfaces = (!interface_tys.is_empty()).then(|| {
            quote! {
                .interfaces(&[
                    #( registry.get_type::<#interface_tys>(info), )*
                ])
            }
        });

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::GraphQLType<#scalar> for #ty#ty_generics #where_clause
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
                    let fields = [
                        #( #fields_meta, )*
                    ];
                    registry.build_object_type::<#ty>(info, &fields)
                        #description
                        #interfaces
                        .into_meta()
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValue`] trait for this
    /// [GraphQL object][1].
    ///
    /// [`GraphQLValue`]: juniper::GraphQLValue
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    #[must_use]
    fn impl_graphql_value_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;
        let context = self.context.clone().unwrap_or_else(|| parse_quote! { () });

        let (impl_generics, ty_generics, where_clause) = self.impl_generics(false);
        let ty = &self.ty;

        let name = &self.name;

        let fields_resolvers = self
            .fields
            .iter()
            .filter_map(|f| f.method_resolve_field_tokens(None));
        let async_fields_panic = {
            let names = self
                .fields
                .iter()
                .filter_map(|f| f.is_async.then(|| f.name.as_str()))
                .collect::<Vec<_>>();
            (!names.is_empty()).then(|| {
                field::Definition::method_resolve_field_panic_async_field_tokens(&names, scalar)
            })
        };
        let no_field_panic = field::Definition::method_resolve_field_panic_no_field_tokens(scalar);

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::GraphQLValue<#scalar> for #ty#ty_generics #where_clause
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
                        #async_fields_panic
                        _ => #no_field_panic,
                    }
                }

                fn concrete_type_name(
                    &self,
                    _: &Self::Context,
                    _: &Self::TypeInfo,
                ) -> String {
                    #name.to_string()
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValueAsync`] trait for this
    /// [GraphQL object][1].
    ///
    /// [`GraphQLValueAsync`]: juniper::GraphQLValueAsync
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    #[must_use]
    fn impl_graphql_value_async_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_generics, where_clause) = self.impl_generics(true);
        let ty = &self.ty;

        let fields_resolvers = self
            .fields
            .iter()
            .map(|f| f.method_resolve_field_async_tokens(None));
        let no_field_panic = field::Definition::method_resolve_field_panic_no_field_tokens(scalar);

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::GraphQLValueAsync<#scalar> for #ty#ty_generics #where_clause
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
                        _ => #no_field_panic,
                    }
                }
            }
        }
    }

    /// Returns generated code implementing [`AsDynGraphQLValue`] trait for this
    /// [GraphQL object][1].
    ///
    /// [`AsDynGraphQLValue`]: juniper::AsDynGraphQLValue
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    #[must_use]
    fn impl_as_dyn_graphql_value_tokens(&self) -> Option<TokenStream> {
        if self.interfaces.is_empty() {
            return None;
        }

        let scalar = &self.scalar;

        let (impl_generics, ty_generics, where_clause) = self.impl_generics(true);
        let ty = &self.ty;

        Some(quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::AsDynGraphQLValue<#scalar> for #ty#ty_generics #where_clause
            {
                type Context = <Self as ::juniper::GraphQLValue<#scalar>>::Context;
                type TypeInfo = <Self as ::juniper::GraphQLValue<#scalar>>::TypeInfo;

                fn as_dyn_graphql_value(
                    &self,
                ) -> &::juniper::DynGraphQLValue<#scalar, Self::Context, Self::TypeInfo> {
                    self
                }

                fn as_dyn_graphql_value_async(
                    &self,
                ) -> &::juniper::DynGraphQLValueAsync<#scalar, Self::Context, Self::TypeInfo> {
                    self
                }
            }
        })
    }
}
