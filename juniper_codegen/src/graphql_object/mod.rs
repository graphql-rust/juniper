//! Code generation for [GraphQL object][1].
//!
//! [1]: https://spec.graphql.org/June2018/#sec-Objects

pub mod attr;
pub mod derive;

use std::{collections::HashSet, convert::TryInto as _};

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens, TokenStreamExt as _};
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
    util::{
        filter_attrs, get_deprecated, get_doc_comment, span_container::SpanContainer, RenameRule,
    },
};

/// Available metadata (arguments) behind `#[graphql]` (or `#[graphql_object]`)
/// attribute when generating code for [GraphQL object][1] type.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Objects
#[derive(Debug, Default)]
struct ObjectMeta {
    /// Explicitly specified name of [GraphQL object][1] type.
    ///
    /// If absent, then Rust type name is used by default.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of [GraphQL object][1] type.
    ///
    /// If absent, then Rust doc comment is used as [description][2], if any.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    description: Option<SpanContainer<String>>,

    /// Explicitly specified type of `juniper::Context` to use for resolving
    /// this [GraphQL object][1] type with.
    ///
    /// If absent, then unit type `()` is assumed as type of `juniper::Context`.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type of `juniper::ScalarValue` to use for resolving
    /// this [GraphQL object][1] type with.
    ///
    /// If absent, then generated code will be generic over any
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
    /// the `juniper` library.
    is_internal: bool,
}

impl Parse for ObjectMeta {
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

impl ObjectMeta {
    /// Tries to merge two [`ObjectMeta`]s into a single one, reporting about
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

    /// Parses [`ObjectMeta`] from the given multiple `name`d
    /// [`syn::Attribute`]s placed on a struct or impl block definition.
    fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut meta = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if meta.description.is_none() {
            meta.description = get_doc_comment(attrs);
        }

        Ok(meta)
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
    interfaces: Vec<syn::Type>,
}

impl Definition {
    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL object][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    #[must_use]
    fn impl_graphql_type_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
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
        let mut interface_tys: Vec<_> = self.interfaces.iter().map(|iface| &iface.ty).collect();
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

    /// Returns generated code implementing [`GraphQLObject`] trait for this
    /// [GraphQL object][1].
    ///
    /// [`GraphQLObject`]: juniper::GraphQLObject
    /// [1]: https://spec.graphql.org/June2018/#sec-Objects
    #[must_use]
    fn impl_graphql_object_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (impl_generics, ty_generics, where_clause) = self.generics();
        let ty = &self.ty;

        let interface_tys: Vec<_> = self.interfaces.iter().map(|iface| &iface.ty).collect();
        let all_interfaces_unique = (interface_tys.len() > 1).then(|| {
            quote! {
                ::juniper::sa::assert_type_ne_all!(#( #interface_tys ),*);
            }
        });

        quote! {
            #[automatically_derived]
            impl#impl_generics ::juniper::marker::GraphQLObject<#scalar> for #ty#ty_generics #where_clause
            {
                fn mark() {
                    #all_interfaces_unique
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

        let (impl_generics, ty_generics, where_clause) = self.generics();
        let ty = &self.ty;

        let fields_marks = self.fields.iter().map(|f| f.method_mark_tokens(scalar));

        let interface_tys = self.interfaces.iter().map(|iface| &iface.ty);

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
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        into.append_all(&[
            self.impl_graphql_object_tokens(),
            self.impl_output_type_tokens(),
            self.impl_graphql_type_tokens(),
        ]);
    }
}
