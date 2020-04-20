#![allow(clippy::single_match)]

pub mod parse_impl;

use quote::quote;
use std::collections::HashMap;
use syn::{
    parse, parse_quote, punctuated::Punctuated, Attribute, Lit, Meta, MetaList, MetaNameValue,
    NestedMeta, Token,
};

pub fn juniper_path(is_internal: bool) -> syn::Path {
    let name = if is_internal { "crate" } else { "juniper" };
    syn::parse_str::<syn::Path>(name).unwrap()
}

/// Returns the name of a type.
/// If the type does not end in a simple ident, `None` is returned.
pub fn name_of_type(ty: &syn::Type) -> Option<syn::Ident> {
    let path_opt = match ty {
        syn::Type::Path(ref type_path) => Some(&type_path.path),
        syn::Type::Reference(ref reference) => match &*reference.elem {
            syn::Type::Path(ref type_path) => Some(&type_path.path),
            syn::Type::TraitObject(ref trait_obj) => {
                match trait_obj.bounds.iter().next().unwrap() {
                    syn::TypeParamBound::Trait(ref trait_bound) => Some(&trait_bound.path),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    };
    let path = path_opt?;

    path.segments
        .iter()
        .last()
        .map(|segment| segment.ident.clone())
}

/// Compares a path to a one-segment string value,
/// return true if equal.
pub fn path_eq_single(path: &syn::Path, value: &str) -> bool {
    path.segments.len() == 1 && path.segments[0].ident == value
}

/// Check if a type is a reference to another type.
pub fn type_is_ref_of(ty: &syn::Type, target: &syn::Type) -> bool {
    match ty {
        syn::Type::Reference(_ref) => &*_ref.elem == target,
        _ => false,
    }
}

/// Check if a Type is a simple identifier.
pub fn type_is_identifier(ty: &syn::Type, name: &str) -> bool {
    match ty {
        syn::Type::Path(ref type_path) => path_eq_single(&type_path.path, name),
        _ => false,
    }
}

/// Check if a Type is a reference to a given identifier.
pub fn type_is_identifier_ref(ty: &syn::Type, name: &str) -> bool {
    match ty {
        syn::Type::Reference(_ref) => type_is_identifier(&*_ref.elem, name),
        _ => false,
    }
}

pub enum AttributeValidation {
    Any,
    // Bare,
    String,
}

pub enum AttributeValue {
    Bare,
    String(String),
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

pub fn get_deprecated(attrs: &[Attribute]) -> Option<DeprecationAttr> {
    for attr in attrs {
        match attr.parse_meta() {
            Ok(Meta::List(ref list)) if list.path.is_ident("deprecated") => {
                return Some(get_deprecated_meta_list(list));
            }
            Ok(Meta::Path(ref path)) if path.is_ident("deprecated") => {
                return Some(DeprecationAttr { reason: None });
            }
            _ => {}
        }
    }
    None
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
                    _ => panic!("deprecated attribute note value only has string literal"),
                }
            } else {
                panic!(
                    "Unrecognized setting on #[deprecated(..)] attribute: {:?}",
                    nv.path,
                );
            }
        }
    }
    DeprecationAttr { reason: None }
}

// Gets doc comment.
pub fn get_doc_comment(attrs: &[Attribute]) -> Option<String> {
    if let Some(items) = get_doc_attr(attrs) {
        if let Some(doc_strings) = get_doc_strings(&items) {
            return Some(join_doc_strings(&doc_strings));
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
        .map(|s| if s.starts_with(' ') { &s[1..] } else { s })
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
fn get_doc_strings(items: &[MetaNameValue]) -> Option<Vec<String>> {
    let comments = items
        .iter()
        .filter_map(|item| {
            if item.path.is_ident("doc") {
                match item.lit {
                    Lit::Str(ref strlit) => Some(strlit.value()),
                    _ => panic!("doc attributes only have string literal"),
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if comments.is_empty() {
        None
    } else {
        Some(comments)
    }
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

// Get the nested items of a a #[graphql(...)] attribute.
pub fn get_graphql_attr(attrs: &[Attribute]) -> Option<Vec<NestedMeta>> {
    for attr in attrs {
        match attr.parse_meta() {
            Ok(Meta::List(ref list)) if list.path.is_ident("graphql") => {
                return Some(list.nested.iter().cloned().collect());
            }
            _ => {}
        }
    }
    None
}

pub fn keyed_item_value(
    item: &NestedMeta,
    name: &str,
    validation: AttributeValidation,
) -> Option<AttributeValue> {
    match *item {
        // Attributes in the form of `#[graphql(name = "value")]`.
        NestedMeta::Meta(Meta::NameValue(ref nameval)) if nameval.path.is_ident(name) => {
            match nameval.lit {
                // We have a string attribute value.
                Lit::Str(ref strlit) => Some(AttributeValue::String(strlit.value())),
                _ => None,
            }
        }
        // Attributes in the form of `#[graphql(name)]`.
        NestedMeta::Meta(Meta::Path(ref path)) if path.is_ident(name) => match validation {
            AttributeValidation::String => {
                panic!(format!(
                    "Invalid format for attribute \"{:?}\": expected a string value",
                    item
                ));
            }
            _ => Some(AttributeValue::Bare),
        },
        _ => None,
    }
}

// Note: duplicated from juniper crate!
#[doc(hidden)]
pub fn to_camel_case(s: &str) -> String {
    let mut dest = String::new();

    for (i, part) in s.split('_').enumerate() {
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

#[derive(Default, Debug)]
pub struct ObjectAttributes {
    pub name: Option<String>,
    pub description: Option<String>,
    pub context: Option<syn::Type>,
    pub scalar: Option<syn::Type>,
    pub interfaces: Vec<syn::Type>,
    pub no_async: bool,
}

impl syn::parse::Parse for ObjectAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let mut output = Self {
            name: None,
            description: None,
            context: None,
            scalar: None,
            interfaces: Vec::new(),
            no_async: false,
        };

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<syn::Token![=]>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    output.name = Some(val.value());
                }
                "description" => {
                    input.parse::<syn::Token![=]>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    output.description = Some(val.value());
                }
                "context" | "Context" => {
                    input.parse::<syn::Token![=]>()?;
                    // TODO: remove legacy support for string based Context.
                    let ctx = if let Ok(val) = input.parse::<syn::LitStr>() {
                        eprintln!("DEPRECATION WARNING: using a string literal for the Context is deprecated");
                        eprintln!("Use a normal type instead - example: 'Context = MyContextType'");
                        syn::parse_str::<syn::Type>(&val.value())?
                    } else {
                        input.parse::<syn::Type>()?
                    };
                    output.context = Some(ctx);
                }
                "scalar" | "Scalar" => {
                    input.parse::<syn::Token![=]>()?;
                    let val = input.parse::<syn::Type>()?;
                    output.scalar = Some(val);
                }
                "interfaces" => {
                    input.parse::<syn::Token![=]>()?;
                    let content;
                    syn::bracketed!(content in input);
                    output.interfaces =
                        syn::punctuated::Punctuated::<syn::Type, syn::Token![,]>::parse_terminated(
                            &content,
                        )?
                        .into_iter()
                        .collect();
                }
                // FIXME: make this unneccessary.
                "noasync" => {
                    output.no_async = true;
                }
                other => {
                    return Err(input.error(format!("Unknown attribute: {}", other)));
                }
            }
            if input.lookahead1().peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(output)
    }
}

impl ObjectAttributes {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::parse::Result<Self> {
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
            let mut a = Self::default();
            a.description = get_doc_comment(attrs);
            Ok(a)
        }
    }
}

#[derive(Debug)]
pub struct FieldAttributeArgument {
    pub name: syn::Ident,
    pub default: Option<syn::Expr>,
    pub description: Option<syn::LitStr>,
}

impl parse::Parse for FieldAttributeArgument {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let name = input.parse()?;

        let mut arg = Self {
            name,
            default: None,
            description: None,
        };

        let content;
        syn::parenthesized!(content in input);
        while !content.is_empty() {
            let name = content.parse::<syn::Ident>()?;
            content.parse::<Token![=]>()?;

            match name.to_string().as_str() {
                "description" => {
                    arg.description = Some(content.parse()?);
                }
                "default" => {
                    arg.default = Some(content.parse()?);
                }
                other => {
                    return Err(content.error(format!("Invalid attribute argument key {}", other)));
                }
            }

            // Discard trailing comma.
            content.parse::<Token![,]>().ok();
        }

        Ok(arg)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FieldAttributeParseMode {
    Object,
    Impl,
}

enum FieldAttribute {
    Name(syn::LitStr),
    Description(syn::LitStr),
    Deprecation(DeprecationAttr),
    Skip(syn::Ident),
    Arguments(HashMap<String, FieldAttributeArgument>),
}

impl parse::Parse for FieldAttribute {
    fn parse(input: parse::ParseStream) -> parse::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;

        match ident.to_string().as_str() {
            "name" => {
                input.parse::<Token![=]>()?;
                let lit = input.parse::<syn::LitStr>()?;
                let raw = lit.value();
                if !is_valid_name(&raw) {
                    Err(input.error(format!(
                        "Invalid #[graphql(name = ...)] attribute: \n\
                         '{}' is not a valid field name\nNames must \
                         match /^[_a-zA-Z][_a-zA-Z0-9]*$/",
                        raw,
                    )))
                } else {
                    Ok(FieldAttribute::Name(lit))
                }
            }
            "description" => {
                input.parse::<Token![=]>()?;
                Ok(FieldAttribute::Description(input.parse()?))
            }
            "deprecated" | "deprecation" => {
                let reason = if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    Some(input.parse::<syn::LitStr>()?.value())
                } else {
                    None
                };
                Ok(FieldAttribute::Deprecation(DeprecationAttr { reason }))
            }
            "skip" => Ok(FieldAttribute::Skip(ident)),
            "arguments" => {
                let arg_content;
                syn::parenthesized!(arg_content in input);
                let args = Punctuated::<FieldAttributeArgument, Token![,]>::parse_terminated(
                    &arg_content,
                )?;
                let map = args
                    .into_iter()
                    .map(|arg| (arg.name.to_string(), arg))
                    .collect();
                Ok(FieldAttribute::Arguments(map))
            }
            other => Err(input.error(format!("Unknown attribute: {}", other))),
        }
    }
}

#[derive(Default)]
pub struct FieldAttributes {
    pub name: Option<String>,
    pub description: Option<String>,
    pub deprecation: Option<DeprecationAttr>,
    // Only relevant for GraphQLObject derive.
    pub skip: bool,
    /// Only relevant for object macro.
    pub arguments: HashMap<String, FieldAttributeArgument>,
}

impl parse::Parse for FieldAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let items = Punctuated::<FieldAttribute, Token![,]>::parse_terminated(&input)?;

        let mut output = Self {
            name: None,
            description: None,
            deprecation: None,
            skip: false,
            arguments: Default::default(),
        };

        for item in items {
            match item {
                FieldAttribute::Name(name) => {
                    output.name = Some(name.value());
                }
                FieldAttribute::Description(name) => {
                    output.description = Some(name.value());
                }
                FieldAttribute::Deprecation(attr) => {
                    output.deprecation = Some(attr);
                }
                FieldAttribute::Skip(_) => {
                    output.skip = true;
                }
                FieldAttribute::Arguments(args) => {
                    output.arguments = args;
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
        attrs: Vec<syn::Attribute>,
        _mode: FieldAttributeParseMode,
    ) -> syn::parse::Result<Self> {
        let doc_comment = get_doc_comment(&attrs);
        let deprecation = get_deprecated(&attrs);

        let attr_opt = attrs.into_iter().find(|attr| attr.path.is_ident("graphql"));

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

    pub fn argument(&self, name: &str) -> Option<&FieldAttributeArgument> {
        self.arguments.get(name)
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
    pub resolver_code: proc_macro2::TokenStream,
    pub is_type_inferred: bool,
    pub is_async: bool,
}

pub fn unraw(s: &str) -> String {
    use syn::ext::IdentExt;
    quote::format_ident!("{}", s).unraw().to_string()
}

/// Definition of a graphql type based on information extracted
/// by various macros.
/// The definition can be rendered to Rust code.
#[derive(Debug)]
pub struct GraphQLTypeDefiniton {
    pub name: String,
    pub _type: syn::Type,
    pub context: Option<syn::Type>,
    pub scalar: Option<syn::Type>,
    pub description: Option<String>,
    pub fields: Vec<GraphQLTypeDefinitionField>,
    pub generics: syn::Generics,
    pub interfaces: Option<Vec<syn::Type>>,
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

impl GraphQLTypeDefiniton {
    #[allow(unused)]
    fn has_async_field(&self) -> bool {
        self.fields.iter().any(|field| field.is_async)
    }

    pub fn into_tokens(self, juniper_crate_name: &str) -> proc_macro2::TokenStream {
        let juniper_crate_name = syn::parse_str::<syn::Path>(juniper_crate_name).unwrap();

        let name = &self.name;
        let ty = &self._type;
        let context = self
            .context
            .as_ref()
            .map(|ctx| quote!( #ctx ))
            .unwrap_or_else(|| quote!(()));

        let field_definitions = self.fields.iter().map(|field| {
            let args = field.args.iter().map(|arg| {
                let arg_type = &arg._type;
                let arg_name = unraw(&arg.name);

                let description = match arg.description.as_ref() {
                    Some(value) => quote!( .description( #value ) ),
                    None => quote!(),
                };

                // Code.
                match arg.default.as_ref() {
                    Some(value) => quote!(
                        .argument(
                            registry.arg_with_default::<#arg_type>(#arg_name, &#value, info)
                                #description
                        )
                    ),
                    None => quote!(
                        .argument(
                            registry.arg::<#arg_type>(#arg_name, info)
                                #description
                        )
                    ),
                }
            });

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

            let field_name = unraw(&field.name);

            let _type = &field._type;
            quote! {
                registry
                    .field_convert::<#_type, _, Self::Context>(#field_name, info)
                    #(#args)*
                    #description
                    #deprecation
            }
        });

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
                    quote!(#juniper_crate_name::DefaultScalarValue)
                }
            });

        let resolve_matches = self.fields.iter().map(|field| {
            let name = &field.name;
            let code = &field.resolver_code;

            if field.is_async {
                quote!(
                    #name => {
                        panic!("Tried to resolve async field {} on type {:?} with a sync resolver",
                            #name,
                            <Self as #juniper_crate_name::GraphQLType<#scalar>>::name(_info)
                        );
                    },
                )
            } else {
                let _type = if field.is_type_inferred {
                    quote!()
                } else {
                    let _type = &field._type;
                    quote!(: #_type)
                };
                quote!(
                    #name => {
                        let res #_type = (|| { #code })();
                        #juniper_crate_name::IntoResolvable::into(
                            res,
                            executor.context()
                        )
                            .and_then(|res| {
                                match res {
                                    Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(&(), &r),
                                    None => Ok(#juniper_crate_name::Value::null()),
                                }
                            })
                    },
                )
            }
        });

        let description = self
            .description
            .as_ref()
            .map(|description| quote!( .description(#description) ));

        let interfaces = self.interfaces.as_ref().map(|items| {
            quote!(
                .interfaces(&[
                    #( registry.get_type::< #items >(&()) ,)*
                ])
            )
        });

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
                .push(parse_quote!(__S: #juniper_crate_name::ScalarValue));
        }

        let type_generics_tokens = if self.include_type_generics {
            Some(type_generics)
        } else {
            None
        };
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let resolve_field_async = {
            let resolve_matches_async = self.fields.iter().map(|field| {
                let name = &field.name;
                let code = &field.resolver_code;
                let _type = if field.is_type_inferred {
                    quote!()
                } else {
                    let _type = &field._type;
                    quote!(: #_type)
                };

                if field.is_async {
                    quote!(
                        #name => {
                            let f = async move {
                                let res #_type = async move { #code }.await;

                                let inner_res = #juniper_crate_name::IntoResolvable::into(
                                    res,
                                    executor.context()
                                );
                                match inner_res {
                                    Ok(Some((ctx, r))) => {
                                        let subexec = executor
                                            .replaced_context(ctx);
                                        subexec.resolve_with_ctx_async(&(), &r)
                                            .await
                                    },
                                    Ok(None) => Ok(#juniper_crate_name::Value::null()),
                                    Err(e) => Err(e),
                                }
                            };
                            use futures::future;
                            future::FutureExt::boxed(f)
                        },
                    )
                } else {
                    let inner = if !self.no_async {
                        quote!(
                            let f = async move {
                                match res2 {
                                    Ok(Some((ctx, r))) => {
                                        let sub = executor.replaced_context(ctx);
                                        sub.resolve_with_ctx_async(&(), &r).await
                                    },
                                    Ok(None) => Ok(#juniper_crate_name::Value::null()),
                                    Err(e) => Err(e),
                                }
                            };
                            use futures::future;
                            future::FutureExt::boxed(f)
                        )
                    } else {
                        quote!(
                            let v = match res2 {
                                Ok(Some((ctx, r))) => executor.replaced_context(ctx).resolve_with_ctx(&(), &r),
                                Ok(None) => Ok(#juniper_crate_name::Value::null()),
                                Err(e) => Err(e),
                            };
                            use futures::future;
                            future::FutureExt::boxed(future::ready(v))
                        )
                    };

                    quote!(
                        #name => {
                            let res #_type = (||{ #code })();
                            let res2 = #juniper_crate_name::IntoResolvable::into(
                                res,
                                executor.context()
                            );
                            #inner
                        },
                    )
                }
            });

            let mut where_async = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));

            where_async
                .predicates
                .push(parse_quote!( #scalar: Send + Sync ));
            where_async.predicates.push(parse_quote!(Self: Send + Sync));

            // FIXME: add where clause for interfaces.

            quote!(
                impl#impl_generics #juniper_crate_name::GraphQLTypeAsync<#scalar> for #ty #type_generics_tokens
                    #where_async
                {
                    fn resolve_field_async<'b>(
                        &'b self,
                        info: &'b Self::TypeInfo,
                        field: &'b str,
                        args: &'b #juniper_crate_name::Arguments<#scalar>,
                        executor: &'b #juniper_crate_name::Executor<Self::Context, #scalar>,
                    ) -> #juniper_crate_name::BoxFuture<'b, #juniper_crate_name::ExecutionResult<#scalar>>
                        where #scalar: Send + Sync,
                    {
                        use futures::future;
                        use #juniper_crate_name::GraphQLType;
                        match field {
                            #( #resolve_matches_async )*
                            _ => {
                                panic!("Field {} not found on type {:?}",
                                    field,
                                    <Self as #juniper_crate_name::GraphQLType<#scalar>>::name(info)
                                );
                            }
                        }
                    }
                }
            )
        };

        let output = quote!(
        impl#impl_generics #juniper_crate_name::GraphQLType<#scalar> for #ty #type_generics_tokens
            #where_clause
        {
                type Context = #context;
                type TypeInfo = ();

                fn name(_: &Self::TypeInfo) -> Option<&str> {
                    Some(#name)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut #juniper_crate_name::Registry<'r, #scalar>
                ) -> #juniper_crate_name::meta::MetaType<'r, #scalar>
                    where #scalar : 'r,
                {
                    let fields = vec![
                        #( #field_definitions ),*
                    ];
                    let meta = registry.build_object_type::<#ty>( info, &fields )
                        #description
                        #interfaces;
                    meta.into_meta()
                }

                #[allow(unused_variables)]
                #[allow(unused_mut)]
                fn resolve_field(
                    &self,
                    _info: &(),
                    field: &str,
                    args: &#juniper_crate_name::Arguments<#scalar>,
                    executor: &#juniper_crate_name::Executor<Self::Context, #scalar>,
                ) -> #juniper_crate_name::ExecutionResult<#scalar> {
                    match field {
                        #( #resolve_matches )*
                        _ => {
                            panic!("Field {} not found on type {:?}",
                                field,
                                <Self as #juniper_crate_name::GraphQLType<#scalar>>::name(_info)
                            );
                        }
                    }
                }


                fn concrete_type_name(&self, _: &Self::Context, _: &Self::TypeInfo) -> String {
                    #name.to_string()
                }

        }

        #resolve_field_async
        );
        output
    }

    pub fn into_subscription_tokens(self, juniper_crate_name: &str) -> proc_macro2::TokenStream {
        let juniper_crate_name = syn::parse_str::<syn::Path>(juniper_crate_name).unwrap();

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
                    quote!(#juniper_crate_name::DefaultScalarValue)
                }
            });

        let field_definitions = self.fields.iter().map(|field| {
            let args = field.args.iter().map(|arg| {
                let arg_type = &arg._type;
                let arg_name = &arg.name;

                let description = match arg.description.as_ref() {
                    Some(value) => quote!( .description( #value ) ),
                    None => quote!(),
                };

                match arg.default.as_ref() {
                    Some(value) => quote!(
                        .argument(
                            registry.arg_with_default::<#arg_type>(#arg_name, &#value, info)
                                #description
                        )
                    ),
                    None => quote!(
                        .argument(
                            registry.arg::<#arg_type>(#arg_name, info)
                                #description
                        )
                    ),
                }
            });

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

            let field_name = &field.name;

            let type_name = &field._type;

            let _type;

            if field.is_async {
                _type = quote!(<#type_name as #juniper_crate_name::ExtractTypeFromStream<_, #scalar>>::Item);
            } else {
                panic!("Synchronous resolvers are not supported. Specify that this function is async: 'async fn foo()'")
            }

            quote! {
                registry
                    .field_convert::<#_type, _, Self::Context>(#field_name, info)
                    #(#args)*
                    #description
                    #deprecation
            }
        });

        let description = self
            .description
            .as_ref()
            .map(|description| quote!( .description(#description) ));

        let interfaces = self.interfaces.as_ref().map(|items| {
            quote!(
                .interfaces(&[
                    #( registry.get_type::< #items >(&()) ,)*
                ])
            )
        });

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
                .push(parse_quote!(__S: #juniper_crate_name::ScalarValue));
        }

        let type_generics_tokens = if self.include_type_generics {
            Some(type_generics)
        } else {
            None
        };
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let resolve_matches_async = self.fields
            .iter()
            .filter(|field| field.is_async)
            .map(|field| {
                let name = &field.name;
                let code = &field.resolver_code;

                let _type;
                if field.is_type_inferred {
                    _type = quote!();
                } else {
                    let _type_name = &field._type;
                    _type = quote!(: #_type_name);
                };
                quote!(
                    #name => {
                        futures::FutureExt::boxed(async move {
                            let res #_type = { #code };
                            let res = #juniper_crate_name::IntoFieldResult::<_, #scalar>::into_result(res)?;
                            let executor= executor.as_owned_executor();
                            let f = res.then(move |res| {
                                let executor = executor.clone();
                                let res2: #juniper_crate_name::FieldResult<_, #scalar> =
                                    #juniper_crate_name::IntoResolvable::into(res, executor.context());
                                async move {
                                    let ex = executor.as_executor();
                                    match res2 {
                                        Ok(Some((ctx, r))) => {
                                            let sub = ex.replaced_context(ctx);
                                            sub.resolve_with_ctx_async(&(), &r)
                                                .await
                                                .map_err(|e| ex.new_error(e))
                                        }
                                        Ok(None) => Ok(Value::null()),
                                        Err(e) => Err(ex.new_error(e)),
                                    }
                                }
                            });
                            Ok(
                                #juniper_crate_name::Value::Scalar::<
                                    #juniper_crate_name::ValuesStream
                                >(Box::pin(f))
                            )
                        })
                    }
                )

            });

        let graphql_implementation = quote!(
            impl#impl_generics #juniper_crate_name::GraphQLType<#scalar> for #ty #type_generics_tokens
                #where_clause
            {
                    type Context = #context;
                    type TypeInfo = ();

                    fn name(_: &Self::TypeInfo) -> Option<&str> {
                        Some(#name)
                    }

                    fn meta<'r>(
                        info: &Self::TypeInfo,
                        registry: &mut #juniper_crate_name::Registry<'r, #scalar>
                    ) -> #juniper_crate_name::meta::MetaType<'r, #scalar>
                        where #scalar : 'r,
                    {
                        let fields = vec![
                            #( #field_definitions ),*
                        ];
                        let meta = registry.build_object_type::<#ty>( info, &fields )
                            #description
                            #interfaces;
                        meta.into_meta()
                    }

                    fn resolve_field(
                        &self,
                        _: &(),
                        _: &str,
                        _: &#juniper_crate_name::Arguments<#scalar>,
                        _: &#juniper_crate_name::Executor<Self::Context, #scalar>,
                    ) -> #juniper_crate_name::ExecutionResult<#scalar> {
                        panic!("Called `resolve_field` on subscription object");
                    }


                    fn concrete_type_name(&self, _: &Self::Context, _: &Self::TypeInfo) -> String {
                        #name.to_string()
                    }
            }
        );

        let subscription_implementation = quote!(
            impl#impl_generics #juniper_crate_name::GraphQLSubscriptionType<#scalar> for #ty #type_generics_tokens
            #where_clause
            {
                #[allow(unused_variables)]
                fn resolve_field_into_stream<
                    's, 'i, 'fi, 'args, 'e, 'ref_e, 'res, 'f,
                >(
                    &'s self,
                    info: &'i Self::TypeInfo,
                    field_name: &'fi str,
                    args: #juniper_crate_name::Arguments<'args, #scalar>,
                    executor: &'ref_e #juniper_crate_name::Executor<'ref_e, 'e, Self::Context, #scalar>,
                ) -> std::pin::Pin<Box<
                        dyn futures::future::Future<
                            Output = Result<
                                #juniper_crate_name::Value<#juniper_crate_name::ValuesStream<'res, #scalar>>,
                                #juniper_crate_name::FieldError<#scalar>
                            >
                        > + Send + 'f
                    >>
                    where
                        's: 'f,
                        'i: 'res,
                        'fi: 'f,
                        'e: 'res,
                        'args: 'f,
                        'ref_e: 'f,
                        'res: 'f,
                {
                    use #juniper_crate_name::Value;
                    use futures::stream::StreamExt as _;

                    match field_name {
                            #( #resolve_matches_async )*
                            _ => {
                                panic!("Field {} not found on type {}", field_name, "GraphQLSubscriptionType");
                            }
                        }
                }
            }
        );

        quote!(
            #graphql_implementation
            #subscription_implementation
        )
    }

    pub fn into_union_tokens(self, juniper_crate_name: &str) -> proc_macro2::TokenStream {
        let juniper_crate_name = syn::parse_str::<syn::Path>(juniper_crate_name).unwrap();

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
                    quote!(#juniper_crate_name::DefaultScalarValue)
                }
            });

        let description = self
            .description
            .as_ref()
            .map(|description| quote!( .description(#description) ));

        let meta_types = self.fields.iter().map(|field| {
            let var_ty = &field._type;

            quote! {
                registry.get_type::<&#var_ty>(&(())),
            }
        });

        let matcher_variants = self
            .fields
            .iter()
            .map(|field| {
                let var_ty = &field._type;
                let resolver_code = &field.resolver_code;

                quote!(
                    #resolver_code(ref x) => <#var_ty as #juniper_crate_name::GraphQLType<#scalar>>::name(&()).unwrap().to_string(),
                )
            });

        let concrete_type_resolver = quote!(
            match self {
                #( #matcher_variants )*
            }
        );

        let matcher_expr: Vec<_> = self
            .fields
            .iter()
            .map(|field| {
                let resolver_code = &field.resolver_code;

                quote!(
                    match self { #resolver_code(ref val) => Some(val), _ => None, }
                )
            })
            .collect();

        let resolve_into_type = self.fields.iter().zip(matcher_expr.iter()).map(|(field, expr)| {
            let var_ty = &field._type;

            quote! {
                if type_name == (<#var_ty as #juniper_crate_name::GraphQLType<#scalar>>::name(&())).unwrap() {
                    return #juniper_crate_name::IntoResolvable::into(
                        { #expr },
                        executor.context()
                    )
                    .and_then(|res| {
                        match res {
                            Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(&(), &r),
                            None => Ok(#juniper_crate_name::Value::null()),
                        }
                    });
                }
            }
        });

        let resolve_into_type_async = self.fields.iter().zip(matcher_expr.iter()).map(|(field, expr)| {
            let var_ty = &field._type;

            quote! {
                if type_name == (<#var_ty as #juniper_crate_name::GraphQLType<#scalar>>::name(&())).unwrap() {
                    let inner_res = #juniper_crate_name::IntoResolvable::into(
                        { #expr },
                        executor.context()
                    );

                    let f = async move {
                        match inner_res {
                            Ok(Some((ctx, r))) => {
                                let subexec = executor.replaced_context(ctx);
                                subexec.resolve_with_ctx_async(&(), &r).await
                            },
                            Ok(None) => Ok(#juniper_crate_name::Value::null()),
                            Err(e) => Err(e),
                        }
                    };
                    use futures::future;
                    return future::FutureExt::boxed(f);
                }
            }
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
                .push(parse_quote!(__S: #juniper_crate_name::ScalarValue));
        }

        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let mut where_async = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
        where_async
            .predicates
            .push(parse_quote!( #scalar: Send + Sync ));
        where_async.predicates.push(parse_quote!(Self: Send + Sync));

        let async_type_impl = quote!(
            impl#impl_generics #juniper_crate_name::GraphQLTypeAsync<#scalar> for #ty
                #where_async
            {
                fn resolve_into_type_async<'b>(
                    &'b self,
                    _info: &'b Self::TypeInfo,
                    type_name: &str,
                    _: Option<&'b [#juniper_crate_name::Selection<'b, #scalar>]>,
                    executor: &'b #juniper_crate_name::Executor<'b, 'b, Self::Context, #scalar>
                ) -> #juniper_crate_name::BoxFuture<'b, #juniper_crate_name::ExecutionResult<#scalar>> {
                    let context = &executor.context();

                    #( #resolve_into_type_async )*

                    panic!("Concrete type not handled by instance resolvers on {}", #name);
                }
            }
        );

        let convesion_impls = self.fields.iter().map(|field| {
            let variant_ty = &field._type;
            let resolver_code = &field.resolver_code;

            quote!(
                impl std::convert::From<#variant_ty> for #ty {
                    fn from(val: #variant_ty) -> Self {
                        #resolver_code(val)
                    }
                }
            )
        });

        let mut type_impl = quote! {
            #( #convesion_impls )*

            impl #impl_generics #juniper_crate_name::GraphQLType<#scalar> for #ty #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn name(_ : &Self::TypeInfo) -> Option<&str> {
                    Some(#name)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut #juniper_crate_name::Registry<'r, #scalar>
                ) -> #juniper_crate_name::meta::MetaType<'r, #scalar>
                    where
                        #scalar: 'r,
                {
                    let types = &[
                        #( #meta_types )*
                    ];
                    registry.build_union_type::<#ty>(
                        info, types
                    )
                    #description
                    .into_meta()
                }

                #[allow(unused_variables)]
                fn concrete_type_name(&self, context: &Self::Context, _info: &Self::TypeInfo) -> String {
                    #concrete_type_resolver
                }

                fn resolve_into_type(
                    &self,
                    _info: &Self::TypeInfo,
                    type_name: &str,
                    _: Option<&[#juniper_crate_name::Selection<#scalar>]>,
                    executor: &#juniper_crate_name::Executor<Self::Context, #scalar>,
                ) -> #juniper_crate_name::ExecutionResult<#scalar> {
                    let context = &executor.context();

                    #( #resolve_into_type )*

                    panic!("Concrete type not handled by instance resolvers on {}", #name);
                }
            }
        };

        if !self.no_async {
            type_impl.extend(async_type_impl)
        }

        type_impl
    }

    pub fn into_enum_tokens(self, juniper_crate_name: &str) -> proc_macro2::TokenStream {
        let juniper_crate_name = syn::parse_str::<syn::Path>(juniper_crate_name).unwrap();

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
                    quote!(#juniper_crate_name::DefaultScalarValue)
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
                    Some(reason) => quote!( #juniper_crate_name::meta::DeprecationStatus::Deprecated(Some(#reason.to_string())) ),
                    None => quote!( #juniper_crate_name::meta::DeprecationStatus::Deprecated(None) ),
                })
                .unwrap_or_else(|| quote!(#juniper_crate_name::meta::DeprecationStatus::Current));

            quote!(
                #juniper_crate_name::meta::EnumValue {
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
                &#resolver_code => #juniper_crate_name::Value::scalar(String::from(#variant_name)),
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
                    #juniper_crate_name::InputValue::scalar(#variant_name.to_string()),
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
                .push(parse_quote!(__S: #juniper_crate_name::ScalarValue));
        }

        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let mut where_async = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
        where_async
            .predicates
            .push(parse_quote!( #scalar: Send + Sync ));
        where_async.predicates.push(parse_quote!(Self: Send + Sync));

        let _async = quote!(
            impl#impl_generics #juniper_crate_name::GraphQLTypeAsync<#scalar> for #ty
                #where_async
            {
                fn resolve_async<'a>(
                    &'a self,
                    info: &'a Self::TypeInfo,
                    selection_set: Option<&'a [#juniper_crate_name::Selection<#scalar>]>,
                    executor: &'a #juniper_crate_name::Executor<Self::Context, #scalar>,
                ) -> #juniper_crate_name::BoxFuture<'a, #juniper_crate_name::ExecutionResult<#scalar>> {
                    use #juniper_crate_name::GraphQLType;
                    use futures::future;
                    let v = self.resolve(info, selection_set, executor);
                    future::FutureExt::boxed(future::ready(v))
                }
            }
        );

        let mut body = quote!(
            impl#impl_generics #juniper_crate_name::GraphQLType<#scalar> for #ty
                #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn name(_: &()) -> Option<&'static str> {
                    Some(#name)
                }

                fn meta<'r>(
                    _: &(),
                    registry: &mut #juniper_crate_name::Registry<'r, #scalar>
                ) -> #juniper_crate_name::meta::MetaType<'r, #scalar>
                    where #scalar: 'r,
                {
                    registry.build_enum_type::<#ty>(&(), &[
                        #( #values )*
                    ])
                    #description
                    .into_meta()
                }

                fn resolve(
                    &self,
                    _: &(),
                    _: Option<&[#juniper_crate_name::Selection<#scalar>]>,
                    _: &#juniper_crate_name::Executor<Self::Context, #scalar>
                ) -> #juniper_crate_name::ExecutionResult<#scalar> {
                    let v = match self {
                        #( #resolves )*
                    };
                    Ok(v)
                }
            }

            impl#impl_generics #juniper_crate_name::FromInputValue<#scalar> for #ty
                #where_clause
            {
                fn from_input_value(v: &#juniper_crate_name::InputValue<#scalar>) -> Option<#ty>
                {
                    match v.as_enum_value().or_else(|| {
                        v.as_string_value()
                    }) {
                        #( #from_inputs )*
                        _ => None,
                    }
                }
            }

            impl#impl_generics #juniper_crate_name::ToInputValue<#scalar> for #ty
                #where_clause
            {
                fn to_input_value(&self) -> #juniper_crate_name::InputValue<#scalar> {
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
}

#[cfg(test)]
mod test {
    use super::*;
    use quote::__private::*;
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
        assert_eq!(&to_camel_case("_test")[..], "Test");
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
