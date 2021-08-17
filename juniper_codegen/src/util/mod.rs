#![allow(clippy::single_match)]

pub mod duplicate;
pub mod span_container;

use std::{collections::HashMap, convert::TryFrom, str::FromStr};

use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::quote;
use span_container::SpanContainer;
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Ident, Lit, Meta, MetaList, MetaNameValue, NestedMeta,
};

use crate::common::parse::ParseBufferExt as _;

/// Compares a path to a one-segment string value,
/// return true if equal.
pub fn path_eq_single(path: &syn::Path, value: &str) -> bool {
    path.segments.len() == 1 && path.segments[0].ident == value
}

#[derive(Debug)]
pub struct DeprecationAttr {
    pub reason: Option<String>,
}

pub fn find_graphql_attr(attrs: &[Attribute]) -> Option<&Attribute> {
    attrs
        .iter()
        .find(|attr| path_eq_single(&attr.path, "graphql"))
}

/// Filters given `attrs` to contain attributes only with the given `name`.
pub fn filter_attrs<'a>(
    name: &'a str,
    attrs: &'a [Attribute],
) -> impl Iterator<Item = &'a Attribute> + 'a {
    attrs
        .iter()
        .filter(move |attr| path_eq_single(&attr.path, name))
}

pub fn get_deprecated(attrs: &[Attribute]) -> Option<SpanContainer<DeprecationAttr>> {
    attrs
        .iter()
        .filter_map(|attr| match attr.parse_meta() {
            Ok(Meta::List(ref list)) if list.path.is_ident("deprecated") => {
                let val = get_deprecated_meta_list(list);
                Some(SpanContainer::new(list.path.span(), None, val))
            }
            Ok(Meta::Path(ref path)) if path.is_ident("deprecated") => Some(SpanContainer::new(
                path.span(),
                None,
                DeprecationAttr { reason: None },
            )),
            _ => None,
        })
        .next()
}

fn get_deprecated_meta_list(list: &MetaList) -> DeprecationAttr {
    for meta in &list.nested {
        if let NestedMeta::Meta(Meta::NameValue(ref nv)) = *meta {
            if nv.path.is_ident("note") {
                match nv.lit {
                    Lit::Str(ref strlit) => {
                        return DeprecationAttr {
                            reason: Some(strlit.value()),
                        };
                    }
                    _ => abort!(syn::Error::new(
                        nv.lit.span(),
                        "only strings are allowed for deprecation",
                    )),
                }
            } else {
                abort!(syn::Error::new(
                    nv.path.span(),
                    "unrecognized setting on #[deprecated(..)] attribute",
                ));
            }
        }
    }
    DeprecationAttr { reason: None }
}

// Gets doc comment.
pub fn get_doc_comment(attrs: &[Attribute]) -> Option<SpanContainer<String>> {
    if let Some(items) = get_doc_attr(attrs) {
        if let Some(doc_strings) = get_doc_strings(&items) {
            return Some(doc_strings.map(|strings| join_doc_strings(&strings)));
        }
    }
    None
}

// Concatenates doc strings into one string.
fn join_doc_strings(docs: &[String]) -> String {
    // Note: this is guaranteed since this function is only called
    // from get_doc_strings().
    debug_assert!(!docs.is_empty());

    let last_index = docs.len() - 1;
    docs.iter()
        .map(|s| s.as_str().trim_end())
        // Trim leading space.
        .map(|s| s.strip_prefix(' ').unwrap_or(s))
        // Add newline, exept when string ends in a continuation backslash or is the last line.
        .enumerate()
        .fold(String::new(), |mut buffer, (index, s)| {
            if index == last_index {
                buffer.push_str(s);
            } else if s.ends_with('\\') {
                buffer.push_str(s.trim_end_matches('\\'));
                buffer.push(' ');
            } else {
                buffer.push_str(s);
                buffer.push('\n');
            }
            buffer
        })
}

// Gets doc strings from doc comment attributes.
fn get_doc_strings(items: &[MetaNameValue]) -> Option<SpanContainer<Vec<String>>> {
    let mut span = None;
    let comments = items
        .iter()
        .filter_map(|item| {
            if item.path.is_ident("doc") {
                match item.lit {
                    Lit::Str(ref strlit) => {
                        if span.is_none() {
                            span = Some(strlit.span());
                        }
                        Some(strlit.value())
                    }
                    _ => abort!(syn::Error::new(
                        item.lit.span(),
                        "doc attributes only have string literal"
                    )),
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    span.map(|span| SpanContainer::new(span, None, comments))
}

// Gets doc comment attributes.
fn get_doc_attr(attrs: &[Attribute]) -> Option<Vec<MetaNameValue>> {
    let mut docs = Vec::new();
    for attr in attrs {
        match attr.parse_meta() {
            Ok(Meta::NameValue(ref nv)) if nv.path.is_ident("doc") => docs.push(nv.clone()),
            _ => {}
        }
    }
    if !docs.is_empty() {
        return Some(docs);
    }
    None
}

// Note: duplicated from juniper crate!
#[doc(hidden)]
pub fn to_camel_case(s: &str) -> String {
    let mut dest = String::new();

    // Handle `_` and `__` to be more friendly with the `_var` convention for unused variables, and
    // GraphQL introspection identifiers.
    let s_iter = if let Some(s) = s.strip_prefix("__") {
        dest.push_str("__");
        s
    } else {
        s.strip_prefix('_').unwrap_or(s)
    }
    .split('_')
    .enumerate();

    for (i, part) in s_iter {
        if i > 0 && part.len() == 1 {
            dest.push_str(&part.to_uppercase());
        } else if i > 0 && part.len() > 1 {
            let first = part
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .collect::<String>();
            let second = &part[1..];

            dest.push_str(&first);
            dest.push_str(second);
        } else if i == 0 {
            dest.push_str(part);
        }
    }

    dest
}

pub(crate) fn to_upper_snake_case(s: &str) -> String {
    let mut last_lower = false;
    let mut upper = String::new();
    for c in s.chars() {
        if c == '_' {
            last_lower = false;
        } else if c.is_lowercase() {
            last_lower = true;
        } else if c.is_uppercase() {
            if last_lower {
                upper.push('_');
            }
            last_lower = false;
        }

        for u in c.to_uppercase() {
            upper.push(u);
        }
    }
    upper
}

#[doc(hidden)]
pub fn is_valid_name(field_name: &str) -> bool {
    let mut chars = field_name.chars();

    match chars.next() {
        // first char can't be a digit
        Some(c) if c.is_ascii_alphabetic() || c == '_' => (),
        // can't be an empty string or any other character
        _ => return false,
    };

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The different possible ways to change case of fields in a struct, or variants in an enum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenameRule {
    /// Don't apply a default rename rule.
    None,
    /// Rename to "camelCase" style.
    CamelCase,
    /// Rename to "SCREAMING_SNAKE_CASE" style
    ScreamingSnakeCase,
}

impl RenameRule {
    pub fn apply(&self, field: &str) -> String {
        match self {
            Self::None => field.to_owned(),
            Self::CamelCase => to_camel_case(field),
            Self::ScreamingSnakeCase => to_upper_snake_case(field),
        }
    }
}

impl FromStr for RenameRule {
    type Err = ();

    fn from_str(rule: &str) -> Result<Self, Self::Err> {
        match rule {
            "none" => Ok(Self::None),
            "camelCase" => Ok(Self::CamelCase),
            "SCREAMING_SNAKE_CASE" => Ok(Self::ScreamingSnakeCase),
            _ => Err(()),
        }
    }
}

impl TryFrom<syn::LitStr> for RenameRule {
    type Error = syn::Error;

    fn try_from(lit: syn::LitStr) -> syn::Result<Self> {
        Self::from_str(&lit.value()).map_err(|_| syn::Error::new(lit.span(), "unknown rename rule"))
    }
}

impl Parse for RenameRule {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Self::try_from(input.parse::<syn::LitStr>()?)
    }
}

#[derive(Default, Debug)]
pub struct ObjectAttributes {
    pub name: Option<SpanContainer<String>>,
    pub description: Option<SpanContainer<String>>,
    pub context: Option<SpanContainer<syn::Type>>,
    pub scalar: Option<SpanContainer<syn::Type>>,
    pub interfaces: Vec<SpanContainer<syn::Type>>,
    pub no_async: Option<SpanContainer<()>>,
    pub is_internal: bool,
    pub rename: Option<RenameRule>,
}

impl Parse for ObjectAttributes {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut output = Self::default();

        while !input.is_empty() {
            let ident = input.parse_any_ident()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    output.name = Some(SpanContainer::new(
                        ident.span(),
                        Some(val.span()),
                        val.value(),
                    ));
                }
                "description" => {
                    input.parse::<token::Eq>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    output.description = Some(SpanContainer::new(
                        ident.span(),
                        Some(val.span()),
                        val.value(),
                    ));
                }
                "context" | "Context" => {
                    input.parse::<token::Eq>()?;
                    // TODO: remove legacy support for string based Context.
                    let ctx = if let Ok(val) = input.parse::<syn::LitStr>() {
                        eprintln!("DEPRECATION WARNING: using a string literal for the Context is deprecated");
                        eprintln!("Use a normal type instead - example: 'Context = MyContextType'");
                        syn::parse_str::<syn::Type>(&val.value())?
                    } else {
                        input.parse::<syn::Type>()?
                    };
                    output.context = Some(SpanContainer::new(ident.span(), Some(ctx.span()), ctx));
                }
                "scalar" | "Scalar" => {
                    input.parse::<token::Eq>()?;
                    let val = input.parse::<syn::Type>()?;
                    output.scalar = Some(SpanContainer::new(ident.span(), Some(val.span()), val));
                }
                "impl" | "implements" | "interfaces" => {
                    input.parse::<token::Eq>()?;
                    output.interfaces = input.parse_maybe_wrapped_and_punctuated::<
                        syn::Type, token::Bracket, token::Comma,
                    >()?.into_iter()
                        .map(|interface| {
                            SpanContainer::new(ident.span(), Some(interface.span()), interface)
                        })
                        .collect();
                }
                // FIXME: make this unneccessary.
                "noasync" => {
                    output.no_async = Some(SpanContainer::new(ident.span(), None, ()));
                }
                "internal" => {
                    output.is_internal = true;
                }
                "rename" => {
                    input.parse::<token::Eq>()?;
                    output.rename = Some(input.parse::<RenameRule>()?);
                }
                _ => {
                    return Err(syn::Error::new(ident.span(), "unknown attribute"));
                }
            }
            input.try_parse::<token::Comma>()?;
        }

        Ok(output)
    }
}

impl ObjectAttributes {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let attr_opt = find_graphql_attr(attrs);
        if let Some(attr) = attr_opt {
            // Need to unwrap  outer (), which are not present for proc macro attributes,
            // but are present for regular ones.

            let mut a: Self = attr.parse_args()?;
            if a.description.is_none() {
                a.description = get_doc_comment(attrs);
            }
            Ok(a)
        } else {
            Ok(Self {
                description: get_doc_comment(attrs),
                ..Self::default()
            })
        }
    }
}

#[derive(Debug)]
pub struct FieldAttributeArgument {
    pub name: syn::Ident,
    pub rename: Option<SpanContainer<syn::LitStr>>,
    pub default: Option<syn::Expr>,
    pub description: Option<syn::LitStr>,
}

impl Parse for FieldAttributeArgument {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse::<Ident>()?.unraw();

        let mut arg = Self {
            name,
            rename: None,
            default: None,
            description: None,
        };

        let content;
        syn::parenthesized!(content in input);
        while !content.is_empty() {
            let name = content.parse::<syn::Ident>()?;
            content.parse::<token::Eq>()?;

            match name.to_string().as_str() {
                "name" => {
                    let val: syn::LitStr = content.parse()?;
                    arg.rename = Some(SpanContainer::new(name.span(), Some(val.span()), val));
                }
                "description" => {
                    arg.description = Some(content.parse()?);
                }
                "default" => {
                    arg.default = Some(content.parse()?);
                }
                _ => return Err(syn::Error::new(name.span(), "unknown attribute")),
            }

            // Discard trailing comma.
            content.parse::<token::Comma>().ok();
        }

        Ok(arg)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FieldAttributeParseMode {
    Object,
}

enum FieldAttribute {
    Name(SpanContainer<syn::LitStr>),
    Description(SpanContainer<syn::LitStr>),
    Deprecation(SpanContainer<DeprecationAttr>),
    Skip(SpanContainer<syn::Ident>),
    Arguments(HashMap<String, FieldAttributeArgument>),
    Default(Box<SpanContainer<Option<syn::Expr>>>),
}

impl Parse for FieldAttribute {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;

        match ident.to_string().as_str() {
            "name" => {
                input.parse::<token::Eq>()?;
                let lit = input.parse::<syn::LitStr>()?;
                let raw = lit.value();
                if !is_valid_name(&raw) {
                    Err(syn::Error::new(lit.span(), "name consists of not allowed characters. (must match /^[_a-zA-Z][_a-zA-Z0-9]*$/)"))
                } else {
                    Ok(FieldAttribute::Name(SpanContainer::new(
                        ident.span(),
                        Some(lit.span()),
                        lit,
                    )))
                }
            }
            "description" => {
                input.parse::<token::Eq>()?;
                let lit = input.parse::<syn::LitStr>()?;
                Ok(FieldAttribute::Description(SpanContainer::new(
                    ident.span(),
                    Some(lit.span()),
                    lit,
                )))
            }
            "deprecated" | "deprecation" => {
                let reason = if input.peek(token::Eq) {
                    input.parse::<token::Eq>()?;
                    Some(input.parse::<syn::LitStr>()?)
                } else {
                    None
                };
                Ok(FieldAttribute::Deprecation(SpanContainer::new(
                    ident.span(),
                    reason.as_ref().map(|val| val.span()),
                    DeprecationAttr {
                        reason: reason.map(|val| val.value()),
                    },
                )))
            }
            "skip" => Ok(FieldAttribute::Skip(SpanContainer::new(
                ident.span(),
                None,
                ident,
            ))),
            "arguments" => {
                let arg_content;
                syn::parenthesized!(arg_content in input);
                let args = Punctuated::<FieldAttributeArgument, token::Comma>::parse_terminated(
                    &arg_content,
                )?;
                let map = args
                    .into_iter()
                    .map(|arg| (arg.name.to_string(), arg))
                    .collect();
                Ok(FieldAttribute::Arguments(map))
            }
            "default" => {
                let default_expr = if input.peek(token::Eq) {
                    input.parse::<token::Eq>()?;
                    let lit = input.parse::<syn::LitStr>()?;
                    let default_expr = lit.parse::<syn::Expr>()?;
                    SpanContainer::new(ident.span(), Some(lit.span()), Some(default_expr))
                } else {
                    SpanContainer::new(ident.span(), None, None)
                };

                Ok(FieldAttribute::Default(Box::new(default_expr)))
            }
            _ => Err(syn::Error::new(ident.span(), "unknown attribute")),
        }
    }
}

#[derive(Default)]
pub struct FieldAttributes {
    pub name: Option<SpanContainer<String>>,
    pub description: Option<SpanContainer<String>>,
    pub deprecation: Option<SpanContainer<DeprecationAttr>>,
    // Only relevant for GraphQLObject derive.
    pub skip: Option<SpanContainer<syn::Ident>>,
    /// Only relevant for object macro.
    pub arguments: HashMap<String, FieldAttributeArgument>,
    /// Only relevant for object input objects.
    pub default: Option<SpanContainer<Option<syn::Expr>>>,
}

impl Parse for FieldAttributes {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let items = Punctuated::<FieldAttribute, token::Comma>::parse_terminated(&input)?;

        let mut output = Self::default();

        for item in items {
            match item {
                FieldAttribute::Name(name) => {
                    output.name = Some(name.map(|val| val.value()));
                }
                FieldAttribute::Description(name) => {
                    output.description = Some(name.map(|val| val.value()));
                }
                FieldAttribute::Deprecation(attr) => {
                    output.deprecation = Some(attr);
                }
                FieldAttribute::Skip(ident) => {
                    output.skip = Some(ident);
                }
                FieldAttribute::Arguments(args) => {
                    output.arguments = args;
                }
                FieldAttribute::Default(expr) => {
                    output.default = Some(*expr);
                }
            }
        }

        if !input.is_empty() {
            Err(input.error("Unexpected input"))
        } else {
            Ok(output)
        }
    }
}

impl FieldAttributes {
    pub fn from_attrs(
        attrs: &[syn::Attribute],
        _mode: FieldAttributeParseMode,
    ) -> syn::Result<Self> {
        let doc_comment = get_doc_comment(&attrs);
        let deprecation = get_deprecated(&attrs);

        let attr_opt = attrs.iter().find(|attr| attr.path.is_ident("graphql"));

        let mut output = match attr_opt {
            Some(attr) => attr.parse_args()?,
            None => Self::default(),
        };

        // Check for regular doc comment.
        if output.description.is_none() {
            output.description = doc_comment;
        }
        if output.deprecation.is_none() {
            output.deprecation = deprecation;
        }

        Ok(output)
    }
}

#[derive(Debug)]
pub struct GraphQLTypeDefinitionFieldArg {
    pub name: String,
    pub description: Option<String>,
    pub default: Option<syn::Expr>,
    pub _type: Box<syn::Type>,
}

#[derive(Debug)]
pub struct GraphQLTypeDefinitionField {
    pub name: String,
    pub _type: syn::Type,
    pub description: Option<String>,
    pub deprecation: Option<DeprecationAttr>,
    pub args: Vec<GraphQLTypeDefinitionFieldArg>,
    pub resolver_code: TokenStream,
    pub is_type_inferred: bool,
    pub is_async: bool,
    pub default: Option<TokenStream>,
    pub span: Span,
}

impl syn::spanned::Spanned for GraphQLTypeDefinitionField {
    fn span(&self) -> Span {
        self.span
    }
}

impl<'a> syn::spanned::Spanned for &'a GraphQLTypeDefinitionField {
    fn span(&self) -> Span {
        self.span
    }
}

/// Definition of a graphql type based on information extracted
/// by various macros.
/// The definition can be rendered to Rust code.
#[derive(Debug)]
pub struct GraphQLTypeDefinition {
    pub name: String,
    pub _type: syn::Type,
    pub context: Option<syn::Type>,
    pub scalar: Option<syn::Type>,
    pub description: Option<String>,
    pub fields: Vec<GraphQLTypeDefinitionField>,
    pub generics: syn::Generics,
    pub interfaces: Vec<syn::Type>,
    // Due to syn parsing differences,
    // when parsing an impl the type generics are included in the type
    // directly, but in syn::DeriveInput, the type generics are
    // in the generics field.
    // This flag signifies if the type generics need to be
    // included manually.
    pub include_type_generics: bool,
    // This flag indicates if the generated code should always be
    // generic over the ScalarValue.
    // If false, the scalar is only generic if a generic parameter
    // is specified manually.
    pub generic_scalar: bool,
    // FIXME: make this redundant.
    pub no_async: bool,
}

impl GraphQLTypeDefinition {
    #[allow(unused)]
    fn has_async_field(&self) -> bool {
        self.fields.iter().any(|field| field.is_async)
    }

    pub fn into_enum_tokens(self) -> TokenStream {
        let name = &self.name;
        let ty = &self._type;
        let context = self
            .context
            .as_ref()
            .map(|ctx| quote!( #ctx ))
            .unwrap_or_else(|| quote!(()));

        let scalar = self
            .scalar
            .as_ref()
            .map(|s| quote!( #s ))
            .unwrap_or_else(|| {
                if self.generic_scalar {
                    // If generic_scalar is true, we always insert a generic scalar.
                    // See more comments below.
                    quote!(__S)
                } else {
                    quote!(::juniper::DefaultScalarValue)
                }
            });

        let description = self
            .description
            .as_ref()
            .map(|description| quote!( .description(#description) ));

        let values = self.fields.iter().map(|variant| {
            let variant_name = &variant.name;

            let descr = variant
                .description
                .as_ref()
                .map(|description| quote!(Some(#description.to_string())))
                .unwrap_or_else(|| quote!(None));

            let depr = variant
                .deprecation
                .as_ref()
                .map(|deprecation| match deprecation.reason.as_ref() {
                    Some(reason) => quote!( ::juniper::meta::DeprecationStatus::Deprecated(Some(#reason.to_string())) ),
                    None => quote!( ::juniper::meta::DeprecationStatus::Deprecated(None) ),
                })
                .unwrap_or_else(|| quote!(::juniper::meta::DeprecationStatus::Current));

            quote!(
                ::juniper::meta::EnumValue {
                    name: #variant_name.to_string(),
                    description: #descr,
                    deprecation_status: #depr,
                },
            )
        });

        let resolves = self.fields.iter().map(|variant| {
            let variant_name = &variant.name;
            let resolver_code = &variant.resolver_code;

            quote!(
                &#resolver_code => ::juniper::Value::scalar(String::from(#variant_name)),
            )
        });

        let from_inputs = self.fields.iter().map(|variant| {
            let variant_name = &variant.name;
            let resolver_code = &variant.resolver_code;

            quote!(
                Some(#variant_name) => Some(#resolver_code),
            )
        });

        let to_inputs = self.fields.iter().map(|variant| {
            let variant_name = &variant.name;
            let resolver_code = &variant.resolver_code;

            quote!(
                &#resolver_code =>
                    ::juniper::InputValue::scalar(#variant_name.to_string()),
            )
        });

        let mut generics = self.generics.clone();

        if self.scalar.is_none() && self.generic_scalar {
            // No custom scalar specified, but always generic specified.
            // Therefore we inject the generic scalar.

            generics.params.push(parse_quote!(__S));

            let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
            // Insert ScalarValue constraint.
            where_clause
                .predicates
                .push(parse_quote!(__S: ::juniper::ScalarValue));
        }

        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let mut where_async = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
        where_async
            .predicates
            .push(parse_quote!( #scalar: Send + Sync ));
        where_async.predicates.push(parse_quote!(Self: Sync));

        let _async = quote!(
            impl#impl_generics ::juniper::GraphQLValueAsync<#scalar> for #ty
                #where_async
            {
                fn resolve_async<'a>(
                    &'a self,
                    info: &'a Self::TypeInfo,
                    selection_set: Option<&'a [::juniper::Selection<#scalar>]>,
                    executor: &'a ::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'a, ::juniper::ExecutionResult<#scalar>> {
                    use ::juniper::futures::future;
                    let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
                    Box::pin(future::ready(v))
                }
            }
        );

        let mut body = quote!(
            impl#impl_generics ::juniper::marker::IsInputType<#scalar> for #ty
                #where_clause { }

            impl#impl_generics ::juniper::marker::IsOutputType<#scalar> for #ty
                #where_clause { }

            impl#impl_generics ::juniper::GraphQLType<#scalar> for #ty
                #where_clause
            {
                fn name(_: &()) -> Option<&'static str> {
                    Some(#name)
                }

                fn meta<'r>(
                    _: &(),
                    registry: &mut ::juniper::Registry<'r, #scalar>
                ) -> ::juniper::meta::MetaType<'r, #scalar>
                    where #scalar: 'r,
                {
                    registry.build_enum_type::<#ty>(&(), &[
                        #( #values )*
                    ])
                    #description
                    .into_meta()
                }
            }

            impl#impl_generics ::juniper::GraphQLValue<#scalar> for #ty
                #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn type_name<'__i>(&self, info: &'__i Self::TypeInfo) -> Option<&'__i str> {
                    <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                }

                fn resolve(
                    &self,
                    _: &(),
                    _: Option<&[::juniper::Selection<#scalar>]>,
                    _: &::juniper::Executor<Self::Context, #scalar>
                ) -> ::juniper::ExecutionResult<#scalar> {
                    let v = match self {
                        #( #resolves )*
                    };
                    Ok(v)
                }
            }

            impl#impl_generics ::juniper::FromInputValue<#scalar> for #ty
                #where_clause
            {
                fn from_input_value(v: &::juniper::InputValue<#scalar>) -> Option<#ty>
                {
                    match v.as_enum_value().or_else(|| {
                        v.as_string_value()
                    }) {
                        #( #from_inputs )*
                        _ => None,
                    }
                }
            }

            impl#impl_generics ::juniper::ToInputValue<#scalar> for #ty
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    match self {
                        #( #to_inputs )*
                    }
                }
            }
        );

        if !self.no_async {
            body.extend(_async)
        }

        body
    }

    pub fn into_input_object_tokens(self) -> TokenStream {
        let name = &self.name;
        let ty = &self._type;
        let context = self
            .context
            .as_ref()
            .map(|ctx| quote!( #ctx ))
            .unwrap_or_else(|| quote!(()));

        let scalar = self
            .scalar
            .as_ref()
            .map(|s| quote!( #s ))
            .unwrap_or_else(|| {
                if self.generic_scalar {
                    // If generic_scalar is true, we always insert a generic scalar.
                    // See more comments below.
                    quote!(__S)
                } else {
                    quote!(::juniper::DefaultScalarValue)
                }
            });

        let meta_fields = self
            .fields
            .iter()
            .map(|field| {
                // HACK: use a different interface for the GraphQLField?
                let field_ty = &field._type;
                let field_name = &field.name;

                let description = match field.description.as_ref() {
                    Some(description) => quote!( .description(#description) ),
                    None => quote!(),
                };

                let deprecation = match field.deprecation.as_ref() {
                    Some(deprecation) => {
                        if let Some(reason) = deprecation.reason.as_ref() {
                            quote!( .deprecated(Some(#reason)) )
                        } else {
                            quote!( .deprecated(None) )
                        }
                    }
                    None => quote!(),
                };

                let create_meta_field = match field.default {
                    Some(ref def) => {
                        quote! {
                            registry.arg_with_default::<#field_ty>( #field_name, &#def, &())
                        }
                    }
                    None => {
                        quote! {
                            registry.arg::<#field_ty>(#field_name, &())
                        }
                    }
                };

                quote!(
                    {
                        #create_meta_field
                        #description
                        #deprecation
                    },
                )
            })
            .collect::<Vec<_>>();

        let from_inputs = self
            .fields
            .iter()
            .map(|field| {
                let field_ident = &field.resolver_code;
                let field_name = &field.name;

                // Build from_input clause.
                let from_input_default = match field.default {
                    Some(ref def) => {
                        quote! {
                            Some(&&::juniper::InputValue::Null) | None if true => #def,
                        }
                    }
                    None => quote! {},
                };

                quote!(
                    #field_ident: {
                        match obj.get(#field_name) {
                            #from_input_default
                            Some(ref v) => ::juniper::FromInputValue::from_input_value(v)?,
                            None => ::juniper::FromInputValue::<#scalar>::from_implicit_null()?,
                        }
                    },
                )
            })
            .collect::<Vec<_>>();

        let to_inputs = self
            .fields
            .iter()
            .map(|field| {
                let field_name = &field.name;
                let field_ident = &field.resolver_code;
                // Build to_input clause.
                quote!(
                    (#field_name, self.#field_ident.to_input_value()),
                )
            })
            .collect::<Vec<_>>();

        let description = self
            .description
            .as_ref()
            .map(|description| quote!( .description(#description) ));

        // Preserve the original type_generics before modification,
        // since alteration makes them invalid if self.generic_scalar
        // is specified.
        let (_, type_generics, _) = self.generics.split_for_impl();

        let mut generics = self.generics.clone();

        if self.scalar.is_none() && self.generic_scalar {
            // No custom scalar specified, but always generic specified.
            // Therefore we inject the generic scalar.

            generics.params.push(parse_quote!(__S));

            let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
            // Insert ScalarValue constraint.
            where_clause
                .predicates
                .push(parse_quote!(__S: ::juniper::ScalarValue));
        }

        let type_generics_tokens = if self.include_type_generics {
            Some(type_generics)
        } else {
            None
        };

        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let mut where_async = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));

        where_async
            .predicates
            .push(parse_quote!( #scalar: Send + Sync ));
        where_async.predicates.push(parse_quote!(Self: Sync));

        let async_type = quote!(
            impl#impl_generics ::juniper::GraphQLValueAsync<#scalar> for #ty #type_generics_tokens
                #where_async
            {}
        );

        let marks = self.fields.iter().map(|field| {
            let field_ty = &field._type;
            quote! { <#field_ty as ::juniper::marker::IsInputType<#scalar>>::mark(); }
        });

        let mut body = quote!(
            impl#impl_generics ::juniper::marker::IsInputType<#scalar> for #ty #type_generics_tokens
                #where_clause {
                    fn mark() {
                        #( #marks )*
                    }
                }

            impl#impl_generics ::juniper::GraphQLType<#scalar> for #ty #type_generics_tokens
                #where_clause
            {
                fn name(_: &()) -> Option<&'static str> {
                    Some(#name)
                }

                fn meta<'r>(
                    _: &(),
                    registry: &mut ::juniper::Registry<'r, #scalar>
                ) -> ::juniper::meta::MetaType<'r, #scalar>
                where #scalar: 'r
                {
                    let fields = &[
                        #( #meta_fields )*
                    ];
                    registry.build_input_object_type::<#ty>(&(), fields)
                    #description
                    .into_meta()
                }
            }

            impl#impl_generics ::juniper::GraphQLValue<#scalar> for #ty #type_generics_tokens
                #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn type_name<'__i>(&self, info: &'__i Self::TypeInfo) -> Option<&'__i str> {
                    <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                }
            }

            impl#impl_generics ::juniper::FromInputValue<#scalar> for #ty #type_generics_tokens
                #where_clause
            {
                fn from_input_value(value: &::juniper::InputValue<#scalar>) -> Option<Self>
                {
                    let obj = value.to_object_value()?;
                    Some(#ty {
                        #( #from_inputs )*
                    })
                }
            }

            impl#impl_generics ::juniper::ToInputValue<#scalar> for #ty #type_generics_tokens
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    ::juniper::InputValue::object(vec![
                        #( #to_inputs )*
                    ].into_iter().collect())
                }
            }
        );

        if !self.no_async {
            body.extend(async_type);
        }

        body
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use syn::{Ident, LitStr};

    fn strs_to_strings(source: Vec<&str>) -> Vec<String> {
        source
            .iter()
            .map(|x| (*x).to_string())
            .collect::<Vec<String>>()
    }

    fn litstr(s: &str) -> Lit {
        Lit::Str(LitStr::new(s, Span::call_site()))
    }

    fn ident(s: &str) -> Ident {
        quote::format_ident!("{}", s)
    }

    mod test_get_doc_strings {
        use super::*;

        #[test]
        fn test_single() {
            let result = get_doc_strings(&[MetaNameValue {
                path: ident("doc").into(),
                eq_token: Default::default(),
                lit: litstr("foo"),
            }]);
            assert_eq!(
                &result.unwrap(),
                Some(&strs_to_strings(vec!["foo"])).unwrap()
            );
        }

        #[test]
        fn test_many() {
            let result = get_doc_strings(&[
                MetaNameValue {
                    path: ident("doc").into(),
                    eq_token: Default::default(),
                    lit: litstr("foo"),
                },
                MetaNameValue {
                    path: ident("doc").into(),
                    eq_token: Default::default(),
                    lit: litstr("\n"),
                },
                MetaNameValue {
                    path: ident("doc").into(),
                    eq_token: Default::default(),
                    lit: litstr("bar"),
                },
            ]);
            assert_eq!(
                &result.unwrap(),
                Some(&strs_to_strings(vec!["foo", "\n", "bar"])).unwrap()
            );
        }

        #[test]
        fn test_not_doc() {
            let result = get_doc_strings(&[MetaNameValue {
                path: ident("blah").into(),
                eq_token: Default::default(),
                lit: litstr("foo"),
            }]);
            assert_eq!(&result, &None);
        }
    }

    mod test_join_doc_strings {
        use super::*;

        #[test]
        fn test_single() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo"]));
            assert_eq!(&result, "foo");
        }
        #[test]
        fn test_multiple() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo", "bar"]));
            assert_eq!(&result, "foo\nbar");
        }

        #[test]
        fn test_trims_spaces() {
            let result = join_doc_strings(&strs_to_strings(vec![" foo ", "bar ", " baz"]));
            assert_eq!(&result, "foo\nbar\nbaz");
        }

        #[test]
        fn test_empty() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo", "", "bar"]));
            assert_eq!(&result, "foo\n\nbar");
        }

        #[test]
        fn test_newline_spaces() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo ", "", " bar"]));
            assert_eq!(&result, "foo\n\nbar");
        }

        #[test]
        fn test_continuation_backslash() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo\\", "x\\", "y", "bar"]));
            assert_eq!(&result, "foo x y\nbar");
        }
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(&to_camel_case("test")[..], "test");
        assert_eq!(&to_camel_case("_test")[..], "test");
        assert_eq!(&to_camel_case("__test")[..], "__test");
        assert_eq!(&to_camel_case("first_second")[..], "firstSecond");
        assert_eq!(&to_camel_case("first_")[..], "first");
        assert_eq!(&to_camel_case("a_b_c")[..], "aBC");
        assert_eq!(&to_camel_case("a_bc")[..], "aBc");
        assert_eq!(&to_camel_case("a_b")[..], "aB");
        assert_eq!(&to_camel_case("a")[..], "a");
        assert_eq!(&to_camel_case("")[..], "");
    }

    #[test]
    fn test_to_upper_snake_case() {
        assert_eq!(to_upper_snake_case("abc"), "ABC");
        assert_eq!(to_upper_snake_case("a_bc"), "A_BC");
        assert_eq!(to_upper_snake_case("ABC"), "ABC");
        assert_eq!(to_upper_snake_case("A_BC"), "A_BC");
        assert_eq!(to_upper_snake_case("SomeInput"), "SOME_INPUT");
        assert_eq!(to_upper_snake_case("someInput"), "SOME_INPUT");
        assert_eq!(to_upper_snake_case("someINpuT"), "SOME_INPU_T");
        assert_eq!(to_upper_snake_case("some_INpuT"), "SOME_INPU_T");
    }

    #[test]
    fn test_is_valid_name() {
        assert_eq!(is_valid_name("yesItIs"), true);
        assert_eq!(is_valid_name("NoitIsnt"), true);
        assert_eq!(is_valid_name("iso6301"), true);
        assert_eq!(is_valid_name("thisIsATest"), true);
        assert_eq!(is_valid_name("i6Op"), true);
        assert_eq!(is_valid_name("i!"), false);
        assert_eq!(is_valid_name(""), false);
        assert_eq!(is_valid_name("aTest"), true);
        assert_eq!(is_valid_name("__Atest90"), true);
    }
}
