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

// Note: duplicated from juniper crate!
#[doc(hidden)]
pub fn to_camel_case(s: &str) -> String {
    let mut dest = String::new();

    for (i, part) in s.split('_').enumerate() {
        if i > 0 && part.len() == 1 {
            dest.push_str(&part.to_uppercase());
        }
        else if i > 0 && part.len() > 1 {
            let first = part.chars().next().unwrap().to_uppercase().collect::<String>();
            let second = &part[1..];

            dest.push_str(&first);
            dest.push_str(second);
        }
        else if i == 0 {
            dest.push_str(part);
        }
    }

    dest
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
