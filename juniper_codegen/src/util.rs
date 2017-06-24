use syn::*;

pub fn get_graphl_attr(attrs: &Vec<Attribute>) -> Option<&Vec<NestedMetaItem>> {
    for attr in attrs {
        match attr.value {
            MetaItem::List(ref attr_name, ref items) => {
                if attr_name == "graphql" {
                    return Some(items);
                }
            },
            _ => {},
        }
    }
    None
}

pub fn keyed_item_value(item: &NestedMetaItem, name: &str, must_be_string: bool)
    -> Option<String>
{
    let item = match item {
        &NestedMetaItem::MetaItem(ref item) => item,
        _ => { return None; }
    };
    let lit = match item {
        &MetaItem::NameValue(ref ident, ref lit) => {
            if ident == name {
                lit
            } else {
                return None;
            }
        },
        _ => { return None; },
    };
    match lit {
        &Lit::Str(ref val, _) => {
            Some(val.clone())
        },
        _ => {
            if must_be_string {
                panic!(format!(
                    "Invalid format for attribute \"{:?}\": expected a string",
                    item));
            } else {
                None
            }
        },
    }
}
