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
    util::{self, filter_attrs, get_doc_comment, span_container::SpanContainer, DeprecationAttr},
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
    mut ast: syn::ItemImpl,
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

    let mut scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    let mut resolve_body: Option<syn::Block> = None;
    let mut from_input_value_arg: Option<syn::Ident> = None;
    let mut from_input_value_body: Option<syn::Block> = None;
    let mut from_input_value_result: Option<syn::Type> = None;
    let mut from_str_arg: Option<syn::Ident> = None;
    let mut from_str_body: Option<syn::Block> = None;
    let mut from_str_result: Option<syn::Type> = None;
    for impl_item in &ast.items {
        if let syn::ImplItem::Method(method) = impl_item.clone() {
            match method.sig.ident.to_string().as_str() {
                "resolve" => {
                    resolve_body = Some(method.block);
                }
                "from_input_value" => {
                    from_input_value_arg = get_first_method_arg(method.sig.inputs);
                    from_input_value_result = get_method_return_type(method.sig.output);
                    from_input_value_body = Some(method.block);
                }
                "from_str" => {
                    from_str_arg = get_first_method_arg(method.sig.inputs);
                    from_str_result = get_method_return_type(method.sig.output);
                    from_str_body = Some(method.block);

                    if scalar.is_implicit_generic() {
                        if let Some(sc) = get_scalar(&from_str_result) {
                            scalar = scalar::Type::Concrete(sc)
                        }
                    }
                }
                _ => (),
            }
        }
    }

    Ok(Definition {
        impl_for_type: *ast.self_ty.clone(),
        generics: ast.generics.clone(),
        name,
        description: attr.description.as_deref().cloned(),
        scalar,
        specified_by_url: attr.specified_by_url.as_deref().cloned(),
        resolve_body: resolve_body.ok_or_else(|| {
            ERR.custom_error(ast.span(), "unable to find body of `resolve` method")
        })?,
        from_input_value_arg: from_input_value_arg.ok_or_else(|| {
            ERR.custom_error(
                ast.span(),
                "unable to find argument for `from_input_value` method",
            )
        })?,
        from_input_value_body: from_input_value_body.ok_or_else(|| {
            ERR.custom_error(
                ast.span(),
                "unable to find body of `from_input_value` method",
            )
        })?,
        from_input_value_result: from_input_value_result.ok_or_else(|| {
            ERR.custom_error(
                ast.span(),
                "unable to find return type of `from_input_value` method",
            )
        })?,
        from_str_arg: from_str_arg.ok_or_else(|| {
            ERR.custom_error(ast.span(), "unable to find argument for `from_str` method")
        })?,
        from_str_body: from_str_body.ok_or_else(|| {
            ERR.custom_error(ast.span(), "unable to find body of `from_str` method")
        })?,
        from_str_result: from_str_result.ok_or_else(|| {
            ERR.custom_error(
                ast.span(),
                "unable to find return type of `from_str` method",
            )
        })?,
    }
    .to_token_stream())
}

#[derive(Default)]
struct Attr {
    pub name: Option<SpanContainer<String>>,
    pub description: Option<SpanContainer<String>>,
    pub deprecation: Option<SpanContainer<DeprecationAttr>>,
    pub specified_by_url: Option<SpanContainer<Url>>,
    pub scalar: Option<SpanContainer<scalar::AttrValue>>,
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
                        ERR.custom_error(lit.span(), format!("failed to parse URL: {}", err))
                    })?;
                    out.specified_by_url
                        .replace(SpanContainer::new(ident.span(), Some(lit.span()), url))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "scalar" | "Scalar" | "ScalarValue" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<scalar::AttrValue>()?;
                    out.scalar
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
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
            scalar: try_merge_opt!(scalar: self, another),
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
    resolve_body: syn::Block,
    from_input_value_arg: syn::Ident,
    from_input_value_body: syn::Block,
    from_input_value_result: syn::Type,
    from_str_arg: syn::Ident,
    from_str_body: syn::Block,
    from_str_result: syn::Type,
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
        let resolve_body = &self.resolve_body;

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
                    Ok(#resolve_body)
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
        let resolve_body = &self.resolve_body;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::ToInputValue<#scalar> for #ty
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    let v = #resolve_body;
                    ::juniper::ToInputValue::to_input_value(&v)
                }
            }
        }
    }

    fn impl_from_input_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;
        let from_input_value_result = &self.from_input_value_result;
        let from_input_value_arg = &self.from_input_value_arg;
        let from_input_value_body = &self.from_input_value_body;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::FromInputValue<#scalar> for #ty
                #where_clause
            {
                type Error = <#from_input_value_result as ::juniper::macros::helper::ExtractError>::Error;

                fn from_input_value(#from_input_value_arg: &::juniper::InputValue<#scalar>) -> #from_input_value_result {
                    #from_input_value_body
                }
            }
        }
    }

    fn impl_parse_scalar_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;
        let from_str_result = &self.from_str_result;
        let from_str_arg = &self.from_str_arg;
        let from_str_body = &self.from_str_body;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::ParseScalarValue<#scalar> for #ty
                #where_clause
           {
               fn from_str<'a>(
                    #from_str_arg: ::juniper::parser::ScalarToken<'a>,
               ) -> #from_str_result {
                    #from_str_body
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

#[derive(Debug)]
struct ScalarCodegenInput {
    impl_for_type: Option<syn::PathSegment>,
    custom_data_type: Option<syn::PathSegment>,
    custom_data_type_is_struct: bool,
    resolve_body: Option<syn::Block>,
    from_input_value_arg: Option<syn::Ident>,
    from_input_value_body: Option<syn::Block>,
    from_input_value_result: Option<syn::Type>,
    from_str_arg: Option<syn::Ident>,
    from_str_body: Option<syn::Block>,
    from_str_result: Option<syn::Type>,
}

fn get_first_method_arg(
    inputs: syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]>,
) -> Option<syn::Ident> {
    if let Some(syn::FnArg::Typed(pat_type)) = inputs.first() {
        if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
            return Some(pat_ident.ident.clone());
        }
    }

    None
}

fn get_method_return_type(output: syn::ReturnType) -> Option<syn::Type> {
    match output {
        syn::ReturnType::Type(_, return_type) => Some(*return_type),
        _ => None,
    }
}

// Find the enum type by inspecting the type parameter on the return value
fn get_scalar(return_type: &Option<syn::Type>) -> Option<syn::Type> {
    if let Some(syn::Type::Path(type_path)) = return_type {
        let path_segment = type_path
            .path
            .segments
            .iter()
            .find(|ps| matches!(ps.arguments, syn::PathArguments::AngleBracketed(_)));

        if let Some(path_segment) = path_segment {
            if let syn::PathArguments::AngleBracketed(generic_args) = &path_segment.arguments {
                let generic_type_arg = generic_args.args.iter().find(|generic_type_arg| {
                    matches!(generic_type_arg, syn::GenericArgument::Type(_))
                });

                if let Some(syn::GenericArgument::Type(scalar)) = generic_type_arg {
                    return Some(scalar.clone());
                }
            }
        }
    }

    None
}

// Find the enum type by inspecting the type parameter on the return value
fn get_enum_type(return_type: &Option<syn::Type>) -> Option<syn::PathSegment> {
    if let Some(syn::Type::Path(type_path)) = return_type {
        let path_segment = type_path
            .path
            .segments
            .iter()
            .find(|ps| matches!(ps.arguments, syn::PathArguments::AngleBracketed(_)));

        if let Some(path_segment) = path_segment {
            if let syn::PathArguments::AngleBracketed(generic_args) = &path_segment.arguments {
                let generic_type_arg = generic_args.args.iter().find(|generic_type_arg| {
                    matches!(generic_type_arg, syn::GenericArgument::Type(_))
                });

                if let Some(syn::GenericArgument::Type(syn::Type::Path(type_path))) =
                    generic_type_arg
                {
                    if let Some(path_segment) = type_path.path.segments.first() {
                        return Some(path_segment.clone());
                    }
                }
            }
        }
    }

    None
}

impl syn::parse::Parse for ScalarCodegenInput {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let mut impl_for_type: Option<syn::PathSegment> = None;
        let mut enum_data_type: Option<syn::PathSegment> = None;
        let mut resolve_body: Option<syn::Block> = None;
        let mut from_input_value_arg: Option<syn::Ident> = None;
        let mut from_input_value_body: Option<syn::Block> = None;
        let mut from_input_value_result: Option<syn::Type> = None;
        let mut from_str_arg: Option<syn::Ident> = None;
        let mut from_str_body: Option<syn::Block> = None;
        let mut from_str_result: Option<syn::Type> = None;

        let parse_custom_scalar_value_impl: syn::ItemImpl = input.parse()?;
        // To implement a custom scalar for a struct, it's required to
        // specify a generic type and a type bound
        let custom_data_type_is_struct: bool =
            !parse_custom_scalar_value_impl.generics.params.is_empty();

        let mut self_ty = *parse_custom_scalar_value_impl.self_ty;

        while let syn::Type::Group(type_group) = self_ty {
            self_ty = *type_group.elem;
        }

        if let syn::Type::Path(type_path) = self_ty {
            if let Some(path_segment) = type_path.path.segments.first() {
                impl_for_type = Some(path_segment.clone());
            }
        }

        for impl_item in parse_custom_scalar_value_impl.items {
            if let syn::ImplItem::Method(method) = impl_item {
                match method.sig.ident.to_string().as_str() {
                    "resolve" => {
                        resolve_body = Some(method.block);
                    }
                    "from_input_value" => {
                        from_input_value_arg = get_first_method_arg(method.sig.inputs);
                        from_input_value_result = get_method_return_type(method.sig.output);
                        from_input_value_body = Some(method.block);
                    }
                    "from_str" => {
                        from_str_arg = get_first_method_arg(method.sig.inputs);
                        from_str_result = get_method_return_type(method.sig.output);

                        if !custom_data_type_is_struct {
                            enum_data_type = get_enum_type(&from_str_result);
                        }

                        from_str_body = Some(method.block);
                    }
                    _ => (),
                }
            }
        }

        let custom_data_type = if custom_data_type_is_struct {
            impl_for_type.clone()
        } else {
            enum_data_type
        };

        Ok(ScalarCodegenInput {
            impl_for_type,
            custom_data_type,
            custom_data_type_is_struct,
            resolve_body,
            from_input_value_arg,
            from_input_value_body,
            from_input_value_result,
            from_str_arg,
            from_str_body,
            from_str_result,
        })
    }
}

/// Generate code for the juniper::graphql_scalar proc macro.
pub fn build_scalar(
    attributes: TokenStream,
    body: TokenStream,
    error: GraphQLScope,
) -> syn::Result<TokenStream> {
    let body_span = body.span();

    let attrs = syn::parse2::<util::FieldAttributes>(attributes)?;
    let input = syn::parse2::<ScalarCodegenInput>(body)?;

    let impl_for_type = input.impl_for_type.ok_or_else(|| {
        error.custom_error(
            body_span,
            "unable to find target for implementation target for `GraphQLScalar`",
        )
    })?;
    let custom_data_type = input
        .custom_data_type
        .ok_or_else(|| error.custom_error(body_span, "unable to find custom scalar data type"))?;
    let resolve_body = input
        .resolve_body
        .ok_or_else(|| error.custom_error(body_span, "unable to find body of `resolve` method"))?;
    let from_input_value_arg = input.from_input_value_arg.ok_or_else(|| {
        error.custom_error(
            body_span,
            "unable to find argument for `from_input_value` method",
        )
    })?;
    let from_input_value_body = input.from_input_value_body.ok_or_else(|| {
        error.custom_error(
            body_span,
            "unable to find body of `from_input_value` method",
        )
    })?;
    let from_input_value_result = input.from_input_value_result.ok_or_else(|| {
        error.custom_error(
            body_span,
            "unable to find return type of `from_input_value` method",
        )
    })?;
    let from_str_arg = input.from_str_arg.ok_or_else(|| {
        error.custom_error(body_span, "unable to find argument for `from_str` method")
    })?;
    let from_str_body = input
        .from_str_body
        .ok_or_else(|| error.custom_error(body_span, "unable to find body of `from_str` method"))?;
    let from_str_result = input.from_str_result.ok_or_else(|| {
        error.custom_error(body_span, "unable to find return type of `from_str` method")
    })?;

    let name = attrs
        .name
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| impl_for_type.ident.to_string());
    let description = attrs.description.map(|val| quote!(.description(#val)));
    let specified_by_url = attrs.specified_by_url.map(|url| {
        let url_lit = url.as_str();
        quote!(.specified_by_url(#url_lit))
    });
    let async_generic_type = match input.custom_data_type_is_struct {
        true => quote!(__S),
        _ => quote!(#custom_data_type),
    };
    let async_generic_type_decl = match input.custom_data_type_is_struct {
        true => quote!(<#async_generic_type>),
        _ => quote!(),
    };
    let generic_type = match input.custom_data_type_is_struct {
        true => quote!(S),
        _ => quote!(#custom_data_type),
    };
    let generic_type_decl = match input.custom_data_type_is_struct {
        true => quote!(<#generic_type>),
        _ => quote!(),
    };
    let generic_type_bound = match input.custom_data_type_is_struct {
        true => quote!(where #generic_type: ::juniper::ScalarValue,),
        _ => quote!(),
    };

    let _async = quote!(
        impl#async_generic_type_decl ::juniper::GraphQLValueAsync<#async_generic_type> for #impl_for_type
        where
            Self: Sync,
            Self::TypeInfo: Sync,
            Self::Context: Sync,
            #async_generic_type: ::juniper::ScalarValue + Send + Sync,
        {
            fn resolve_async<'a>(
                &'a self,
                info: &'a Self::TypeInfo,
                selection_set: Option<&'a [::juniper::Selection<#async_generic_type>]>,
                executor: &'a ::juniper::Executor<Self::Context, #async_generic_type>,
            ) -> ::juniper::BoxFuture<'a, ::juniper::ExecutionResult<#async_generic_type>> {
                use ::juniper::futures::future;
                let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
                Box::pin(future::ready(v))
            }
        }
    );

    let content = quote!(
        #_async

        impl#generic_type_decl ::juniper::marker::IsInputType<#generic_type> for #impl_for_type
            #generic_type_bound { }

        impl#generic_type_decl ::juniper::marker::IsOutputType<#generic_type> for #impl_for_type
            #generic_type_bound { }

        impl#generic_type_decl ::juniper::GraphQLType<#generic_type> for #impl_for_type
        #generic_type_bound
        {
            fn name(_: &Self::TypeInfo) -> Option<&'static str> {
                Some(#name)
            }

            fn meta<'r>(
                info: &Self::TypeInfo,
                registry: &mut ::juniper::Registry<'r, #generic_type>,
            ) -> ::juniper::meta::MetaType<'r, #generic_type>
            where
                #generic_type: 'r,
            {
                registry.build_scalar_type::<Self>(info)
                    #description
                    #specified_by_url
                    .into_meta()
            }
        }

        impl#generic_type_decl ::juniper::GraphQLValue<#generic_type> for #impl_for_type
        #generic_type_bound
        {
            type Context = ();
            type TypeInfo = ();

            fn type_name<'__i>(&self, info: &'__i Self::TypeInfo) -> Option<&'__i str> {
                <Self as ::juniper::GraphQLType<#generic_type>>::name(info)
            }

            fn resolve(
                &self,
                info: &(),
                selection: Option<&[::juniper::Selection<#generic_type>]>,
                executor: &::juniper::Executor<Self::Context, #generic_type>,
            ) -> ::juniper::ExecutionResult<#generic_type> {
                Ok(#resolve_body)
            }
        }

        impl#generic_type_decl ::juniper::ToInputValue<#generic_type> for #impl_for_type
        #generic_type_bound
        {
            fn to_input_value(&self) -> ::juniper::InputValue<#generic_type> {
                let v = #resolve_body;
                ::juniper::ToInputValue::to_input_value(&v)
            }
        }

        impl#generic_type_decl ::juniper::FromInputValue<#generic_type> for #impl_for_type
        #generic_type_bound
        {
            type Error = <#from_input_value_result as ::juniper::macros::helper::ExtractError>::Error;

            fn from_input_value(#from_input_value_arg: &::juniper::InputValue<#generic_type>) -> #from_input_value_result {
                #from_input_value_body
            }
        }

        impl#generic_type_decl ::juniper::ParseScalarValue<#generic_type> for #impl_for_type
        #generic_type_bound
            {
                fn from_str<'a>(
                    #from_str_arg: ::juniper::parser::ScalarToken<'a>,
                ) -> #from_str_result {
                #from_str_body
            }
        }

        impl#generic_type_decl ::juniper::macros::reflection::BaseType<#generic_type> for #impl_for_type
            #generic_type_bound
        {
            const NAME: ::juniper::macros::reflection::Type = #name;
        }

        impl#generic_type_decl ::juniper::macros::reflection::BaseSubTypes<#generic_type> for #impl_for_type
            #generic_type_bound
        {
            const NAMES: ::juniper::macros::reflection::Types =
                &[<Self as ::juniper::macros::reflection::BaseType<#generic_type>>::NAME];
        }

        impl#generic_type_decl ::juniper::macros::reflection::WrappedType<#generic_type> for #impl_for_type
            #generic_type_bound
        {
            const VALUE: ::juniper::macros::reflection::WrappedValue = 1;
        }
    );

    Ok(content)
}
