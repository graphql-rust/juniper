extern crate proc_macro;
extern crate proc_macro2;

use crate::util;
// use quote::quote;
use quote::quote;
use std::iter::{FromIterator, IntoIterator};
use syn::{
    parse::{Parse, ParseStream, Result},
    Token,
};

#[derive(Debug, Clone)]
struct NamedField {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    ident: syn::Ident,
    colon_token: syn::Token![:],
    ty: syn::Type,
}

impl NamedField {
    fn into_field(self) -> syn::Field {
        syn::Field {
            attrs: self
                .attrs
                .into_iter()
                .filter(|attr| !util::path_eq_single(&attr.path, "graphql"))
                .collect(),
            vis: self.vis,
            ident: Some(self.ident),
            colon_token: Some(self.colon_token),
            ty: self.ty,
        }
    }
}

impl Parse for NamedField {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            attrs: input.call(syn::Attribute::parse_outer)?,
            vis: input.parse()?,
            ident: input.parse()?,
            colon_token: input.parse()?,
            ty: input.parse()?,
        })
    }
}

#[derive(Debug)]
struct GqlObject {
    pub attrs: Vec<syn::Attribute>,
    pub vis: syn::Visibility,
    pub ident: syn::Ident,
    pub data_fields: Option<syn::punctuated::Punctuated<NamedField, Token![,]>>,
    pub fields: Vec<NamedField>,
    pub methods: Vec<syn::ImplItemMethod>,
}

impl GqlObject {
    pub fn to_struct(self) -> syn::ItemStruct {
        syn::ItemStruct {
            attrs: self
                .attrs
                .into_iter()
                .filter(|attr| !util::path_eq_single(&attr.path, "graphql"))
                .collect(),
            vis: self.vis,
            struct_token: syn::token::Struct::default(),
            ident: self.ident,
            generics: syn::Generics::default(),
            fields: syn::Fields::Named(syn::FieldsNamed {
                brace_token: syn::token::Brace::default(),
                named: syn::punctuated::Punctuated::from_iter(
                    self.fields
                        .into_iter()
                        .chain(self.data_fields.unwrap_or_default())
                        .map(NamedField::into_field),
                ),
            }),
            semi_token: None,
        }
    }

    fn to_graphql_definition(&self) -> util::GraphQLTypeDefiniton {
        let struct_fields = syn::punctuated::Punctuated::<_, Token![,]>::from_iter(
            self.fields.iter().cloned().map(NamedField::into_field),
        );

        // Parse attributes.
        let attrs = match util::ObjectAttributes::from_attrs(&self.attrs) {
            Ok(a) => a,
            Err(e) => {
                panic!("Invalid #[graphql(...)] attribute: {}", e);
            }
        };
        if attrs.interfaces.len() > 0 {
            panic!("Invalid #[graphql(...)] attribute 'interfaces': gql! does not support 'interfaces'");
        }
        let ident = &self.ident;
        let name = attrs.name.unwrap_or_else(|| ident.to_string());
        let fields = struct_fields.into_iter().filter_map(|field| {
            let field_attrs = match util::FieldAttributes::from_attrs(
                field.attrs,
                util::FieldAttributeParseMode::Object,
            ) {
                Ok(attrs) => attrs,
                Err(e) => panic!("Invalid #[graphql] attribute: \n{}", e),
            };
            if field_attrs.skip {
                None
            } else {
                let field_name = field.ident.unwrap();
                let name = field_attrs
                    .name
                    .clone()
                    .unwrap_or_else(|| util::to_camel_case(&field_name.to_string()));
                let resolver_code = quote!(
                    &self . #field_name
                );
                Some(util::GraphQLTypeDefinitionField {
                    name,
                    _type: field.ty,
                    args: Vec::new(),
                    description: field_attrs.description,
                    deprecation: field_attrs.deprecation,
                    resolver_code,
                    is_type_inferred: true,
                    is_async: false,
                })
            }
        });
        let mut definition = util::GraphQLTypeDefiniton {
            name,
            _type: syn::parse_str(&self.ident.to_string()).unwrap(),
            context: attrs.context,
            scalar: attrs.scalar,
            description: attrs.description,
            fields: fields.collect(),
            generics: syn::Generics::default(),
            interfaces: None,
            include_type_generics: true,
            generic_scalar: true,
            no_async: attrs.no_async,
        };

        for method in self.methods.iter().cloned() {
            let _type = match &method.sig.output {
                syn::ReturnType::Type(_, ref t) => (**t).clone(),
                syn::ReturnType::Default => {
                    panic!(
                        "Invalid field method {}: must return a value",
                        method.sig.ident
                    );
                }
            };
            let is_async = method.sig.asyncness.is_some();
            let attrs = match util::FieldAttributes::from_attrs(
                method.attrs,
                util::FieldAttributeParseMode::Impl,
            ) {
                Ok(attrs) => attrs,
                Err(err) => panic!(
                    "Invalid #[graphql(...)] attribute on field {}:\n{}",
                    method.sig.ident, err
                ),
            };
            let mut args = Vec::new();
            let mut resolve_parts = Vec::new();
            for arg in method.sig.inputs {
                match arg {
                    syn::FnArg::Receiver(rec) => {
                        if rec.reference.is_none() || rec.mutability.is_some() {
                            panic!(
                                "Invalid method receiver {}(self, ...): did you mean '&self'?",
                                method.sig.ident
                            );
                        }
                    }
                    syn::FnArg::Typed(ref captured) => {
                        let (arg_ident, is_mut) = match &*captured.pat {
                            syn::Pat::Ident(ref pat_ident) => {
                                (&pat_ident.ident, pat_ident.mutability.is_some())
                            }
                            _ => {
                                panic!("Invalid token for function argument");
                            }
                        };
                        let arg_name = arg_ident.to_string();
                        let context_type = definition.context.as_ref();
                        // Check for executor arguments.
                        if util::type_is_identifier_ref(&captured.ty, "Executor") {
                            resolve_parts.push(quote!(let #arg_ident = executor;));
                        }
                        // Make sure executor is specified as a reference.
                        else if util::type_is_identifier(&captured.ty, "Executor") {
                            panic!("Invalid executor argument: to access the Executor, you need to specify the type as a reference.\nDid you mean &Executor?");
                        }
                        // Check for context arg.
                        else if context_type
                            .clone()
                            .map(|ctx| util::type_is_ref_of(&captured.ty, ctx))
                            .unwrap_or(false)
                        {
                            resolve_parts.push(quote!( let #arg_ident = executor.context(); ));
                        }
                        // Make sure the user does not specify the Context
                        //  without a reference. (&Context)
                        else if context_type
                            .clone()
                            .map(|ctx| ctx == &*captured.ty)
                            .unwrap_or(false)
                        {
                            panic!(
                                "Invalid context argument: to access the context, you need to specify the type as a reference.\nDid you mean &{}?",
                                quote!(captured.ty),
                            );
                        } else {
                            // Regular argument.
                            let ty = &captured.ty;
                            // TODO: respect graphql attribute overwrite.
                            let final_name = util::to_camel_case(&arg_name);
                            let expect_text = format!(
                                "Internal error: missing argument {} - validation must have failed",
                                &final_name
                            );
                            let mut_modifier = if is_mut { quote!(mut) } else { quote!() };
                            resolve_parts.push(quote!(
                                let #mut_modifier #arg_ident = args
                                    .get::<#ty>(#final_name)
                                    .expect(#expect_text);
                            ));
                            args.push(util::GraphQLTypeDefinitionFieldArg {
                                description: attrs
                                    .argument(&arg_name)
                                    .and_then(|arg| arg.description.as_ref().map(|d| d.value())),
                                default: attrs
                                    .argument(&arg_name)
                                    .and_then(|arg| arg.default.clone()),
                                _type: ty.clone(),
                                name: final_name,
                            })
                        }
                    }
                }
            }
            let body = &method.block;
            let resolver_code = quote!(
                #( #resolve_parts )*
                #body
            );
            let ident = &method.sig.ident;
            let name = attrs
                .name
                .unwrap_or_else(|| util::to_camel_case(&ident.to_string()));
            definition.fields.push(util::GraphQLTypeDefinitionField {
                name,
                _type,
                args,
                description: attrs.description,
                deprecation: attrs.deprecation,
                resolver_code,
                is_type_inferred: false,
                is_async,
            });
        }

        definition
    }
}

impl Parse for GqlObject {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let vis = input.parse()?;
        let ident = input.parse()?;
        let data_fields = if input.lookahead1().peek(syn::token::Paren) {
            Some(parse_data_fields(input)?)
        } else {
            None
        };
        let (fields, methods) = data_object(input)?;

        Ok(Self {
            attrs,
            vis,
            ident,
            data_fields,
            fields,
            methods,
        })
    }
}

enum ContentElement {
    Field(NamedField),
    Method(syn::ImplItemMethod),
}

fn parse_fn_args(input: ParseStream) -> Result<syn::punctuated::Punctuated<syn::FnArg, Token![,]>> {
    let mut args = syn::punctuated::Punctuated::new();
    let mut has_receiver = false;

    while !input.is_empty() {
        let attrs = input.call(syn::Attribute::parse_outer)?;

        let arg = {
            let mut arg: syn::FnArg = input.parse()?;
            match &mut arg {
                syn::FnArg::Receiver(receiver) if has_receiver => {
                    return Err(syn::Error::new(
                        receiver.self_token.span,
                        "unexpected second method receiver",
                    ));
                }
                syn::FnArg::Receiver(receiver) if !args.is_empty() => {
                    return Err(syn::Error::new(
                        receiver.self_token.span,
                        "unexpected method receiver",
                    ));
                }
                syn::FnArg::Receiver(receiver) => {
                    has_receiver = true;
                    receiver.attrs = attrs;
                }
                syn::FnArg::Typed(arg) => arg.attrs = attrs,
            }
            arg
        };
        args.push_value(arg);

        if input.is_empty() {
            break;
        }

        let comma: Token![,] = input.parse()?;
        args.push_punct(comma);
    }

    Ok(args)
}

impl Parse for ContentElement {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let vis = input.parse()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![fn])
            || lookahead.peek(Token![async])
            || lookahead.peek(Token![const])
        {
            let constness: Option<Token![const]> = input.parse()?;
            let asyncness: Option<Token![async]> = input.parse()?;
            let fn_token: Token![fn] = input.parse()?;
            let ident: syn::Ident = input.parse()?;

            let content;
            let paren_token = syn::parenthesized!(content in input);
            let inputs = parse_fn_args(&content)?;
            let output: syn::ReturnType = input.parse()?;
            let content;
            let brace_token = syn::braced!(content in input);
            let mut attrs = attrs;
            attrs.extend(content.call(syn::Attribute::parse_inner)?);
            let block = syn::Block {
                brace_token,
                stmts: content.call(syn::Block::parse_within)?,
            };
            Ok(ContentElement::Method(syn::ImplItemMethod {
                attrs,
                vis,
                defaultness: None,
                sig: syn::Signature {
                    constness,
                    asyncness,
                    unsafety: None,
                    abi: None,
                    fn_token,
                    ident,
                    paren_token,
                    inputs,
                    output,
                    variadic: None,
                    generics: syn::Generics::default(),
                },
                block,
            }))
        } else {
            Ok(ContentElement::Field(NamedField {
                attrs,
                vis,
                ident: input.parse()?,
                colon_token: input.parse()?,
                ty: input.parse()?,
            }))
        }
    }
}

fn data_object(input: ParseStream) -> Result<(Vec<NamedField>, Vec<syn::ImplItemMethod>)> {
    let content;
    syn::braced!(content in input);
    let mut fields = vec![];
    let mut methods = vec![];

    while !content.is_empty() {
        match content.parse()? {
            ContentElement::Field(field) => fields.push(field),
            ContentElement::Method(method) => methods.push(method),
        }
    }
    Ok((fields, methods))
}

fn parse_data_fields(
    input: ParseStream,
) -> Result<syn::punctuated::Punctuated<NamedField, Token![,]>> {
    let content;
    syn::parenthesized!(content in input);
    Ok(content.parse_terminated(NamedField::parse)?)
}

pub(crate) struct GqlBlock(Vec<GqlObject>);

impl GqlBlock {
    pub(crate) fn into_tokens(self) -> proc_macro2::TokenStream {
        let impls = self.0.into_iter().map(|obj| {
            let impls = obj.to_graphql_definition().into_tokens("juniper");
            let struc = obj.to_struct();
            quote!(
                #struc
                #impls
            )
        });
        proc_macro2::TokenStream::from_iter(impls)
    }
}

impl Parse for GqlBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut objs = vec![];
        while !input.is_empty() {
            objs.push(input.parse()?);
        }
        Ok(Self(objs))
    }
}
