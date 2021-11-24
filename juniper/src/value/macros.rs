use super::{DefaultScalarValue, Value};

/// Construct JSON-like [`Value`]s by using JSON syntax.
///
/// [`Value`] objects are used mostly when creating custom errors from fields.
///
/// # Example
///
/// Resulting JSON will look just like what you passed in.
/// ```rust
/// # use juniper::{graphql_value, DefaultScalarValue, Value};
/// # type V = Value<DefaultScalarValue>;
/// #
/// # let _: V =
/// graphql_value!(null);
/// # let _: V =
/// graphql_value!(1234);
/// # let _: V =
/// graphql_value!("test");
/// # let _: V =
/// graphql_value!([1234, "test", true]);
/// # let _: V =
/// graphql_value!({"key": "value", "foo": 1234});
/// ```
#[macro_export]
macro_rules! graphql_value {
    ///////////
    // Array //
    ///////////

    // Done with trailing comma.
    (@array [$($elems:expr,)*]) => {
        $crate::Value::list(vec![
            $( $crate::graphql_value!($elems), )*
        ])
    };

    // Done without trailing comma.
    (@array [$($elems:expr),*]) => {
        $crate::Value::list(vec![
            $( $crate::graphql_value!($elems), )*
        ])
    };

    // Next element is `null`.
    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::graphql_value!(
            @array [$($elems,)* $crate::graphql_value!(null)] $($rest)*
        )
    };

    // Next element is an array.
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::graphql_value!(
            @array [$($elems,)* $crate::graphql_value!([$($array)*])] $($rest)*
        )
    };

    // Next element is a map.
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::graphql_value!(
            @array [$($elems,)* $crate::graphql_value!({$($map)*})] $($rest)*
        )
    };

    // Next element is an expression followed by comma.
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::graphql_value!(
            @array [$($elems,)* $crate::graphql_value!($next),] $($rest)*
        )
    };

    // Last element is an expression with no trailing comma.
    (@array [$($elems:expr,)*] $last:expr) => {
        $crate::graphql_value!(
            @array [$($elems,)* $crate::graphql_value!($last)]
        )
    };

    // Comma after the most recent element.
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::graphql_value!(@array [$($elems,)*] $($rest)*)
    };

    // Unexpected token after most recent element.
    (@array [$($elems:expr),*] $unexpected:tt $($rest:tt)*) => {
        crate::graphql_value!(@unexpected $unexpected)
    };

    ////////////
    // Object //
    ////////////

    // Done.
    (@object $object:ident () () ()) => {};

    // Insert the current entry followed by trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.add_field(($($key)+), $value);
        $crate::graphql_value!(@object $object () ($($rest)*) ($($rest)*));
    };

    // Current entry followed by unexpected token.
    (@object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        $crate::graphql_value!(@unexpected $unexpected);
    };

    // Insert the last entry without trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $object.add_field(($($key)+), $value);
    };

    // Next value is `null`.
    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(@object $object [$($key)+] ($crate::graphql_value!(null)) $($rest)*);
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(@object $object [$($key)+] ($crate::graphql_value!([$($array)*])) $($rest)*);
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(@object $object [$($key)+] ($crate::graphql_value!({$($map)*})) $($rest)*);
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(@object $object [$($key)+] ($crate::graphql_value!($value)) , $($rest)*);
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::graphql_value!(@object $object [$($key)+] ($crate::graphql_value!($value)));
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::graphql_value!();
    };

    // Missing colon and value for last entry. Trigger a reasonable error
    // message.
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::graphql_value!();
    };

    // Misplaced colon. Trigger a reasonable error message.
    (@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        crate::graphql_value!(@unexpected $colon);
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        crate::graphql_value!(@unexpected $comma);
    };

    // Key is fully parenthesized. This avoids clippy double_parens false
    // positives because the parenthesization may be necessary here.
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };

    // Refuse to absorb colon token into key expression.
    (@object $object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        crate::graphql_value!(@unexpected $($unexpected)+);
    };

    // Munch a token into the current key.
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    ////////////
    // Errors //
    ////////////

    (@unexpected) => {};

    //////////////
    // Defaults //
    //////////////

    ([ $($arr:tt)* ]) => {
        $crate::graphql_value!(@array [] $($arr)*)
    };

    ({}) => {
        $crate::Value::object($crate::Object::with_capacity(0))
    };

    ({ $($map:tt)+ }) => {
        $crate::Value::object({
            let mut object = $crate::Object::with_capacity(0);
            $crate::graphql_value!(@object object () ($($map)*) ($($map)*));
            object
        })
    };

    (null) => ($crate::Value::null());

    ($e:expr) => ($crate::Value::from($e));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_macro_string() {
        let s: Value<DefaultScalarValue> = graphql_value!("test");
        assert_eq!(s, Value::scalar("test"));
    }

    #[test]
    fn value_macro_int() {
        let s: Value<DefaultScalarValue> = graphql_value!(123);
        assert_eq!(s, Value::scalar(123));
    }

    #[test]
    fn value_macro_float() {
        let s: Value<DefaultScalarValue> = graphql_value!(123.5);
        assert_eq!(s, Value::scalar(123.5));
    }

    #[test]
    fn value_macro_boolean() {
        let s: Value<DefaultScalarValue> = graphql_value!(false);
        assert_eq!(s, Value::scalar(false));
    }

    #[test]
    fn value_macro_option() {
        let s: Value<DefaultScalarValue> = graphql_value!(Some("test"));
        assert_eq!(s, Value::scalar("test"));
        let s: Value<DefaultScalarValue> = graphql_value!(null);
        assert_eq!(s, Value::null());
    }

    #[test]
    fn value_macro_list() {
        let s: Value<DefaultScalarValue> = graphql_value!([123, "Test", false]);
        assert_eq!(
            s,
            Value::list(vec![
                Value::scalar(123),
                Value::scalar("Test"),
                Value::scalar(false),
            ])
        );
        let s: Value<DefaultScalarValue> = graphql_value!([123, [456], 789]);
        assert_eq!(
            s,
            Value::list(vec![
                Value::scalar(123),
                Value::list(vec![Value::scalar(456)]),
                Value::scalar(789),
            ])
        );
        let s: Value<DefaultScalarValue> = graphql_value!([123, [1 + 2], 789]);
        assert_eq!(
            s,
            Value::list(vec![
                Value::scalar(123),
                Value::list(vec![Value::scalar(3)]),
                Value::scalar(789),
            ])
        );
    }

    #[test]
    fn value_macro_object() {
        let s: Value<DefaultScalarValue> = graphql_value!({ "key": 123, "next": true });
        assert_eq!(
            s,
            Value::object(
                vec![("key", Value::scalar(123)), ("next", Value::scalar(true))]
                    .into_iter()
                    .collect(),
            )
        );
        let s: Value<DefaultScalarValue> = graphql_value!({ "key": 1 + 2, "next": true });
        assert_eq!(
            s,
            Value::object(
                vec![("key", Value::scalar(3)), ("next", Value::scalar(true))]
                    .into_iter()
                    .collect(),
            )
        );
    }

    #[test]
    fn display_null() {
        let s: Value<DefaultScalarValue> = graphql_value!(null);
        assert_eq!("null", format!("{}", s));
    }

    #[test]
    fn display_int() {
        let s: Value<DefaultScalarValue> = graphql_value!(123);
        assert_eq!("123", format!("{}", s));
    }

    #[test]
    fn display_float() {
        let s: Value<DefaultScalarValue> = graphql_value!(123.456);
        assert_eq!("123.456", format!("{}", s));
    }

    #[test]
    fn display_string() {
        let s: Value<DefaultScalarValue> = graphql_value!("foo");
        assert_eq!("\"foo\"", format!("{}", s));
    }

    #[test]
    fn display_bool() {
        let s: Value<DefaultScalarValue> = graphql_value!(false);
        assert_eq!("false", format!("{}", s));

        let s: Value<DefaultScalarValue> = graphql_value!(true);
        assert_eq!("true", format!("{}", s));
    }

    #[test]
    fn display_list() {
        let s: Value<DefaultScalarValue> = graphql_value!([1, null, "foo"]);
        assert_eq!("[1, null, \"foo\"]", format!("{}", s));
    }

    #[test]
    fn display_list_one_element() {
        let s: Value<DefaultScalarValue> = graphql_value!([1]);
        assert_eq!("[1]", format!("{}", s));
    }

    #[test]
    fn display_list_empty() {
        let s: Value<DefaultScalarValue> = graphql_value!([]);
        assert_eq!("[]", format!("{}", s));
    }

    #[test]
    fn display_object() {
        let s: Value<DefaultScalarValue> = graphql_value!({
            "int": 1,
            "null": null,
            "string": "foo",
        });
        assert_eq!(
            r#"{"int": 1, "null": null, "string": "foo"}"#,
            format!("{}", s)
        );
    }

    #[test]
    fn display_object_one_field() {
        let s: Value<DefaultScalarValue> = graphql_value!({
            "int": 1,
        });
        assert_eq!(r#"{"int": 1}"#, format!("{}", s));
    }

    #[test]
    fn display_object_empty() {
        let s: Value<DefaultScalarValue> = graphql_value!({});
        assert_eq!(r#"{}"#, format!("{}", s));
    }
}
