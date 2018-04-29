use syn::*;
use regex::Regex;

pub fn get_graphl_attr(attrs: &Vec<Attribute>) -> Option<&Vec<NestedMetaItem>> {
    for attr in attrs {
        match attr.value {
            MetaItem::List(ref attr_name, ref items) => if attr_name == "graphql" {
                return Some(items);
            },
            _ => {}
        }
    }
    None
}

pub fn keyed_item_value(item: &NestedMetaItem, name: &str, must_be_string: bool) -> Option<String> {
    let item = match item {
        &NestedMetaItem::MetaItem(ref item) => item,
        _ => {
            return None;
        }
    };
    let lit = match item {
        &MetaItem::NameValue(ref ident, ref lit) => if ident == name {
            lit
        } else {
            return None;
        },
        _ => {
            return None;
        }
    };
    match lit {
        &Lit::Str(ref val, _) => Some(val.clone()),
        _ => if must_be_string {
            panic!(format!(
                "Invalid format for attribute \"{:?}\": expected a string",
                item
            ));
        } else {
            None
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


#[doc(hidden)]
pub fn is_camel_case(field_name: &str) -> bool {
    lazy_static!{
        static ref CAMELCASE: Regex = Regex::new("^[a-z][a-z0-9]*(?:[A-Z][a-z0-9]*)*$").unwrap();
    }
    CAMELCASE.is_match(field_name)
}

#[test]
fn test_is_camel_case(){
    assert_eq!(is_camel_case("yesItIs"), true);
    assert_eq!(is_camel_case("NoitIsnt"), false); 
    assert_eq!(is_camel_case("iso6301"), true); 
    assert_eq!(is_camel_case("thisIsATest"), true); 
    assert_eq!(is_camel_case("i6Op"), true);
    assert_eq!(is_camel_case("i!"), false);
    assert_eq!(is_camel_case(""), false);   
    assert_eq!(is_camel_case("aTest"), true);
}

#[doc(hidden)]
pub fn is_pascal_case(obj_name: &str) -> bool {
    lazy_static!{
        static ref PASCALCASE: Regex = Regex::new("^_{0,2}[A-Z][a-z0-9]*(?:[A-Z][a-z0-9]*)*$").unwrap();
    }
    PASCALCASE.is_match(obj_name)
}

#[test]
fn test_is_pascal_case(){
    assert_eq!(is_pascal_case("YesItIs"), true);
    assert_eq!(is_pascal_case("NoitIsnt"), true); 
    assert_eq!(is_pascal_case("Iso6301"), true); 
    assert_eq!(is_pascal_case("ThisIsATest"), true); 
    assert_eq!(is_pascal_case("i6Op"), false);
    assert_eq!(is_pascal_case("i!"), false);
    assert_eq!(is_pascal_case(""), false); 
    assert_eq!(is_pascal_case("aTest"), false);
    assert_eq!(is_pascal_case("Test_Test"), false);
}

#[doc(hidden)]
pub fn is_upper_snakecase(enum_field: &str) -> bool {
    lazy_static!{
        static ref UPPERCASE: Regex = Regex::new("^[A-Z](?:[A-Z0-9]+_?)*$").unwrap();
    }
    UPPERCASE.is_match(enum_field)
}

#[test]
fn test_is_upper_snakecase(){
    assert_eq!(is_upper_snakecase("YESITIS"), true);
    assert_eq!(is_upper_snakecase("no_It_Isnt"), false); 
    assert_eq!(is_upper_snakecase("ISO6301"), true); 
    assert_eq!(is_upper_snakecase("This"), false); 
    assert_eq!(is_upper_snakecase("i6Op"), false);
    assert_eq!(is_upper_snakecase("i!"), false);
    assert_eq!(is_upper_snakecase(""), false); 
    assert_eq!(is_upper_snakecase("TEST_TEST"), true);
    assert_eq!(is_upper_snakecase("Test_Test"), false);
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