use std::borrow::Cow;

/// Convert string to camel case.
///
/// Note: needs to be public because several macros use it.
#[doc(hidden)]
pub fn to_camel_case<'a>(s: &'a str) -> Cow<'a, str> {
    let mut dest = Cow::Borrowed(s);

    for (i, part) in s.split('_').enumerate() {
        if i > 0 && part.len() == 1 {
            dest += Cow::Owned(part.to_uppercase());
        } else if i > 0 && part.len() > 1 {
            let first = part
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .collect::<String>();
            let second = &part[1..];

            dest += Cow::Owned(first);
            dest += second;
        } else if i == 0 {
            dest = Cow::Borrowed(part);
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
