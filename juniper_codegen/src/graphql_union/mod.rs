pub mod attribute;
pub mod derive;

use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
};

use crate::util::{
    filter_graphql_attrs, get_doc_comment, span_container::SpanContainer, OptionExt as _,
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
}

impl Parse for UnionMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut output = Self::default();

        // TODO: check for duplicates?
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
        })
    }

    /// Parses [`UnionMeta`] from the given attributes placed on type definition.
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut meta = filter_graphql_attrs(attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if meta.description.is_none() {
            meta.description = get_doc_comment(attrs);
        }

        Ok(meta)
    }
}

/// Available metadata behind `#[graphql]` attribute when generating code for [GraphQL union][1]'s
/// variant.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
#[derive(Debug, Default)]
struct UnionVariantMeta {
    /// Explicitly specified marker for the variant/field being ignored and not included into
    /// [GraphQL union][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Unions
    pub ignore: Option<SpanContainer<syn::Ident>>,
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
        })
    }

    /// Parses [`UnionVariantMeta`] from the given attributes placed on variant/field definition.
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_graphql_attrs(attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}
