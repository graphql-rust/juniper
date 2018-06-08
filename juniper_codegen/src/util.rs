use syn::{
    Attribute,
    Meta,
    MetaNameValue,
    NestedMeta,
    Lit,
};
use regex::Regex;

// Gets doc comment.
pub fn get_doc_comment(attrs: &Vec<Attribute>) -> Option<String> {
    if let Some(items) = get_doc_attr(attrs) {
        if let Some(doc_strings) = get_doc_strings(&items) {
            return Some(join_doc_strings(&doc_strings));
        }
    }
    None
}

// Concatenates doc strings into one string.
fn join_doc_strings(docs: &Vec<String>) -> String {
    let s: String = docs.iter()
        // Convert empty comments to newlines.
        .map(|x| if x == "" { "\n".to_string() } else { x.clone() })
        .collect::<Vec<String>>()
        .join(" ");
    // Clean up spacing on empty lines.
    s.replace(" \n ", "\n")
}

// Gets doc strings from doc comment attributes.
fn get_doc_strings(items: &Vec<MetaNameValue>) -> Option<Vec<String>> {
    let mut docs = Vec::new();
    for item in items {
        match item.lit {
            Lit::Str(ref strlit) => {
                docs.push(strlit.value().trim().to_string());
            },
            _ => panic!("doc attributes only have string literal"),
        }
    }
    if !docs.is_empty() {
        return Some(docs);
    }
    None
}

// Gets doc comment attributes.
fn get_doc_attr(attrs: &Vec<Attribute>) -> Option<Vec<MetaNameValue>> {
    let mut docs = Vec::new();
    for attr in attrs {
        match attr.interpret_meta() {
            Some(Meta::NameValue(ref nv)) if nv.ident == "doc" => {
                docs.push(nv.clone())
            }
            _ => {}
        }
    }
    if !docs.is_empty() {
        return Some(docs);
    }
    None
}

// Get the nested items of a a #[graphql(...)] attribute.
pub fn get_graphl_attr(attrs: &Vec<Attribute>) -> Option<Vec<NestedMeta>> {
    for attr in attrs {
        match attr.interpret_meta() {
            Some(Meta::List(ref list)) if list.ident == "graphql" => {
                return Some(list.nested.iter().map(|x| x.clone()).collect());
            },
            _ => {}
        }
    }
    None
}

pub fn keyed_item_value(item: &NestedMeta, name: &str, must_be_string: bool) -> Option<String> {
    match item {
        &NestedMeta::Meta(Meta::NameValue(ref nameval)) if nameval.ident == name => {
            match &nameval.lit {
                &Lit::Str(ref strlit) => {
                    Some(strlit.value())
                },
                _ => if must_be_string {
                    panic!(format!(
                        "Invalid format for attribute \"{:?}\": expected a string",
                        item
                    ));
                } else {
                    None
                },
            }
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
            let first = part.chars()
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

#[doc(hidden)]
pub fn is_valid_name(field_name: &str) -> bool {
    lazy_static!{
        static ref GRAPHQL_NAME_SPEC: Regex = Regex::new("^[_A-Za-z][_0-9A-Za-z]*$").unwrap();
    }
    GRAPHQL_NAME_SPEC.is_match(field_name)
}

#[test]
fn test_is_valid_name(){
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
