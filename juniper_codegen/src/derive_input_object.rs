use syn;
use syn::*;
use quote::Tokens;

use util::*;


#[derive(Default, Debug)]
struct ObjAttrs {
    name: Option<String>,
    description: Option<String>,
}

impl ObjAttrs {
    fn from_input(input: &DeriveInput) -> ObjAttrs {
        let mut res = ObjAttrs::default();

        // Check attributes for name and description.
        if let Some(items) = get_graphl_attr(&input.attrs) {
            for item in items {
                if let Some(val) = keyed_item_value(item, "name", true) {
                    res.name = Some(val);
                    continue;
                }
                if let Some(val) = keyed_item_value(item, "description", true) {
                    res.description = Some(val);
                    continue;
                }
                panic!(format!(
                    "Unknown attribute for #[derive(GraphQLInputObject)]: {:?}",
                    item
                ));
            }
        }
        res
    }
}

#[derive(Default)]
struct ObjFieldAttrs {
    name: Option<String>,
    description: Option<String>,
    default: Option<String>,
}

impl ObjFieldAttrs {
    fn from_input(variant: &Field) -> ObjFieldAttrs {
        let mut res = ObjFieldAttrs::default();

        // Check attributes for name and description.
        if let Some(items) = get_graphl_attr(&variant.attrs) {
            for item in items {
                if let Some(val) = keyed_item_value(item, "name", true) {
                    res.name = Some(val);
                    continue;
                }
                if let Some(val) = keyed_item_value(item, "description", true) {
                    res.description = Some(val);
                    continue;
                }
                if let Some(val) = keyed_item_value(item, "default", true) {
                    res.default = Some(val);
                    continue;
                }
                panic!(format!(
                    "Unknown attribute for #[derive(GraphQLInputObject)]: {:?}",
                    item
                ));
            }
        }
        res
    }
}

pub fn impl_input_object(ast: &syn::DeriveInput) -> Tokens {
    let fields = match ast.body {
        Body::Struct(ref data) => match data {
            &VariantData::Struct(ref fields) => fields,
            _ => {
                panic!(
                    "#[derive(GraphQLInputObject)] may only be used on regular structs with fields"
                );
            }
        },
        Body::Enum(_) => {
            panic!("#[derive(GraphlQLInputObject)] may only be applied to structs, not to enums");
        }
    };

    // Parse attributes.
    let ident = &ast.ident;
    let attrs = ObjAttrs::from_input(ast);
    let name = attrs.name.unwrap_or(ast.ident.to_string());

    let meta_description = match attrs.description {
        Some(descr) => quote!{ let meta = meta.description(#descr); },
        None => quote!{ let meta = meta; },
    };

    let mut meta_fields = Vec::<Tokens>::new();
    let mut from_inputs = Vec::<Tokens>::new();
    let mut to_inputs = Vec::<Tokens>::new();

    for field in fields {
        let field_ty = &field.ty;
        let field_attrs = ObjFieldAttrs::from_input(field);
        let field_ident = field.ident.as_ref().unwrap();

        // Build value.
        let name = match field_attrs.name {
            Some(ref name) => {
                // Custom name specified.
                name.to_string()
            }
            None => {
                // Note: auto camel casing when no custom name specified.
                ::util::to_camel_case(field_ident.as_ref())
            }
        };
        let field_description = match field_attrs.description {
            Some(s) => quote!{ let field = field.description(#s); },
            None => quote!{ let field = field; },
        };

        let default = match field_attrs.default {
            Some(ref def) => match syn::parse_token_trees(def) {
                Ok(t) => Some(quote!{ #(#t)* }),
                Err(_) => {
                    panic!("#graphql(default = ?) must be a valid Rust expression inside a string");
                }
            },
            None => None,
        };

        let create_meta_field = match default {
            Some(ref def) => {
                quote!{
                    let field = registry.arg_with_default::<#field_ty>( #name, &#def, &());
                }
            }
            None => {
                quote!{
                    let field = registry.arg::<#field_ty>(#name, &());
                }
            }
        };
        let meta_field = quote!{
            {
                #create_meta_field
                #field_description
                field
            },
        };
        meta_fields.push(meta_field);

        // Buil from_input clause.

        let from_input_default = match default {
            Some(ref def) => {
                quote!{
                    Some(&&::juniper::InputValue::Null) | None if true => #def,
                }
            }
            None => quote!{},
        };

        let from_input = quote!{
            #field_ident: {
                // TODO: investigate the unwraps here, they seem dangerous!
                match obj.get(#name) {
                    #from_input_default
                    Some(v) => ::juniper::FromInputValue::from_input_value(v).unwrap(),
                    _ => ::juniper::FromInputValue::from_input_value(&::juniper::InputValue::null()).unwrap()
                }
            },
        };
        from_inputs.push(from_input);

        // Build to_input clause.
        let to_input = quote!{
            (#name, self.#field_ident.to()),
        };
        to_inputs.push(to_input);
    }

    quote! {
        impl ::juniper::GraphQLType for #ident {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &()) -> Option<&'static str> {
                Some(#name)
            }

            fn meta<'r>(_: &(), registry: &mut ::juniper::Registry<'r>) -> ::juniper::meta::MetaType<'r> {
                let fields = &[
                    #(#meta_fields)*
                ];
                let meta = registry.build_input_object_type::<#ident>(&(), fields);
                #meta_description
                meta.into_meta()
            }
        }

        impl ::juniper::FromInputValue for #ident {
            fn from_input_value(value: &::juniper::InputValue) -> Option<#ident> {
                if let Some(obj) = value.to_object_value() {
                    let item = #ident {
                        #(#from_inputs)*
                    };
                    Some(item)
                }
                else {
                    None
                }
            }
        }

        impl ::juniper::ToInputValue for #ident {
            fn to(&self) -> ::juniper::InputValue {
                ::juniper::InputValue::object(vec![
                    #(#to_inputs)*
                ].into_iter().collect())
            }
        }
    }
}
