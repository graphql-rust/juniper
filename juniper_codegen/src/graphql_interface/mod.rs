//! Code generation for [GraphQL interface][1].
//!
//! [1]: https://spec.graphql.org/June2018/#sec-Interfaces

pub mod attr;

use std::collections::HashMap;

use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
};

use crate::util::{
    dup_attr_err, filter_attrs, get_doc_comment, span_container::SpanContainer, OptionExt as _,
};

/*
/// Helper alias for the type of [`InterfaceMeta::external_downcasters`] field.
type InterfaceMetaDowncasters = HashMap<syn::Type, SpanContainer<syn::ExprPath>>;*/

/// Available metadata (arguments) behind `#[graphql]` (or `#[graphql_interface]`) attribute when
/// generating code for [GraphQL interface][1] type.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
#[derive(Debug, Default)]
struct InterfaceMeta {
    /// Explicitly specified name of [GraphQL interface][1] type.
    ///
    /// If absent, then Rust type name is used by default.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of [GraphQL interface][1] type.
    ///
    /// If absent, then Rust doc comment is used as [description][2], if any.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    pub description: Option<SpanContainer<String>>,

    /// Explicitly specified type of `juniper::Context` to use for resolving this
    /// [GraphQL interface][1] type with.
    ///
    /// If absent, then unit type `()` is assumed as type of `juniper::Context`.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type of `juniper::ScalarValue` to use for resolving this
    /// [GraphQL interface][1] type with.
    ///
    /// If absent, then generated code will be generic over any `juniper::ScalarValue` type, which,
    /// in turn, requires all [interface][1] implementors to be generic over any
    /// `juniper::ScalarValue` type too. That's why this type should be specified only if one of the
    /// implementors implements `juniper::GraphQLType` in a non-generic way over
    /// `juniper::ScalarValue` type.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub scalar: Option<SpanContainer<syn::Type>>,

    /*
    /// Explicitly specified external downcasting functions for [GraphQL interface][1] implementors.
    ///
    /// If absent, then macro will try to auto-infer all the possible variants from the type
    /// declaration, if possible. That's why specifying an external resolver function has sense,
    /// when some custom [union][1] variant resolving logic is involved, or variants cannot be
    /// inferred.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    pub external_downcasters: InterfaceMetaDowncasters,*/
    /// Indicator whether the generated code is intended to be used only inside the `juniper`
    /// library.
    pub is_internal: bool,
}

impl Parse for InterfaceMeta {
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
                "internal" => {
                    output.is_internal = true;
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

impl InterfaceMeta {
    /// Tries to merge two [`InterfaceMeta`]s into single one, reporting about duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            context: try_merge_opt!(context: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            is_internal: self.is_internal || another.is_internal,
        })
    }

    /// Parses [`InterfaceMeta`] from the given multiple `name`d [`syn::Attribute`]s placed on a
    /// trait definition.
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
