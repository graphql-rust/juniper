//! [`graphql_value!`] macro implementation.
//!
//! [`graphql_value!`]: graphql_value

/// Constructs [`Value`]s via JSON-like syntax.
///
/// [`Value`] objects are used mostly when creating custom errors from fields.
///
/// [`Value::Object`] key should implement [`AsRef`]`<`[`str`]`>`.
/// ```rust
/// # use juniper::{graphql_value, Value};
/// #
/// let code = 200;
/// let features = ["key", "value"];
///
/// let value: Value = graphql_value!({
///     "code": code,
///     "success": code == 200,
///     "payload": {
///         features[0]: features[1],
///     },
/// });
/// ```
///
/// # Example
///
/// Resulting JSON will look just like what you passed in.
/// ```rust
/// # use juniper::{graphql_value, DefaultScalarValue, Value};
/// #
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
///
/// [`Value`]: crate::Value
/// [`Value::Object`]: crate::Value::Object
#[macro_export]
macro_rules! graphql_value {
    ///////////
    // Array //
    ///////////

    // Done with trailing comma.
    (@array [$($elems:expr,)*]) => {
        $crate::Value::list(vec![
            $( $elems, )*
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

    // Next element is `None`.
    (@array [$($elems:expr,)*] None $($rest:tt)*) => {
        $crate::graphql_value!(
            @array [$($elems,)* $crate::graphql_value!(None)] $($rest)*
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
        $crate::graphql_value!(@unexpected $unexpected)
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
        $crate::graphql_value!(
            @object $object
            [$($key)+]
            ($crate::graphql_value!(null)) $($rest)*
        );
    };

    // Next value is `None`.
    (@object $object:ident ($($key:tt)+) (: None $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(
            @object $object
            [$($key)+]
            ($crate::graphql_value!(None)) $($rest)*
        );
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(
            @object $object
            [$($key)+]
            ($crate::graphql_value!([$($array)*])) $($rest)*
        );
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(
            @object $object
            [$($key)+]
            ($crate::graphql_value!({$($map)*})) $($rest)*
        );
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(
            @object $object
            [$($key)+]
            ($crate::graphql_value!($value)) , $($rest)*
        );
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::graphql_value!(
            @object $object
            [$($key)+]
            ($crate::graphql_value!($value))
        );
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
        $crate::graphql_value!(@unexpected $colon);
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        $crate::graphql_value!(@unexpected $comma);
    };

    // Key is fully parenthesized. This avoids `clippy::double_parens` false
    // positives because the parenthesization may be necessary here.
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };

    // Refuse to absorb colon token into key expression.
    (@object $object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        $crate::graphql_value!(@unexpected $($unexpected)+);
    };

    // Munch a token into the current key.
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::graphql_value!(
            @object $object
            ($($key)* $tt)
            ($($rest)*) ($($rest)*)
        );
    };

    ////////////
    // Errors //
    ////////////

    (@unexpected) => {};

    //////////////
    // Defaults //
    //////////////

    ([ $($arr:tt)* ]$(,)?) => {
        $crate::graphql_value!(@array [] $($arr)*)
    };

    ({}$(,)?) => {
        $crate::Value::object($crate::Object::with_capacity(0))
    };

    ({ $($map:tt)+ }$(,)?) => {
        $crate::Value::object({
            let mut object = $crate::Object::with_capacity(0);
            $crate::graphql_value!(@object object () ($($map)*) ($($map)*));
            object
        })
    };

    (null$(,)?) => ($crate::Value::null());

    (None$(,)?) => ($crate::Value::null());

    ($e:expr$(,)?) => ($crate::Value::from($e));
}

#[cfg(test)]
mod tests {
    type V = crate::Value;

    #[test]
    fn null() {
        assert_eq!(graphql_value!(null), V::Null);
    }

    #[test]
    fn scalar() {
        let val = 42;

        assert_eq!(graphql_value!(1), V::scalar(1));
        assert_eq!(graphql_value!("val"), V::scalar("val"));
        assert_eq!(graphql_value!(1.34), V::scalar(1.34));
        assert_eq!(graphql_value!(false), V::scalar(false));
        assert_eq!(graphql_value!(1 + 2), V::scalar(3));
        assert_eq!(graphql_value!(val), V::scalar(42));
    }

    #[test]
    fn list() {
        let val = 42;

        assert_eq!(graphql_value!([]), V::list(vec![]));

        assert_eq!(graphql_value!([null]), V::list(vec![V::Null]));

        assert_eq!(graphql_value!([1]), V::list(vec![V::scalar(1)]));
        assert_eq!(graphql_value!([1 + 2]), V::list(vec![V::scalar(3)]));
        assert_eq!(graphql_value!([val]), V::list(vec![V::scalar(42)]));

        assert_eq!(
            graphql_value!([1, [2], 3]),
            V::list(vec![
                V::scalar(1),
                V::list(vec![V::scalar(2)]),
                V::scalar(3),
            ]),
        );
        assert_eq!(
            graphql_value!(["string", [2 + 3], true]),
            V::list(vec![
                V::scalar("string"),
                V::list(vec![V::scalar(5)]),
                V::scalar(true),
            ]),
        );
    }

    #[test]
    fn object() {
        let val = 42;

        assert_eq!(
            graphql_value!({}),
            V::object(Vec::<(String, _)>::new().into_iter().collect()),
        );
        assert_eq!(
            graphql_value!({ "key": null }),
            V::object(vec![("key", V::Null)].into_iter().collect()),
        );
        assert_eq!(
            graphql_value!({ "key": 123 }),
            V::object(vec![("key", V::scalar(123))].into_iter().collect()),
        );
        assert_eq!(
            graphql_value!({ "key": 1 + 2 }),
            V::object(vec![("key", V::scalar(3))].into_iter().collect()),
        );
        assert_eq!(
            graphql_value!({ "key": [] }),
            V::object(vec![("key", V::list(vec![]))].into_iter().collect()),
        );
        assert_eq!(
            graphql_value!({ "key": [null] }),
            V::object(vec![("key", V::list(vec![V::Null]))].into_iter().collect()),
        );
        assert_eq!(
            graphql_value!({ "key": [1] }),
            V::object(
                vec![("key", V::list(vec![V::scalar(1)]))]
                    .into_iter()
                    .collect(),
            ),
        );
        assert_eq!(
            graphql_value!({ "key": [1 + 2] }),
            V::object(
                vec![("key", V::list(vec![V::scalar(3)]))]
                    .into_iter()
                    .collect(),
            ),
        );
        assert_eq!(
            graphql_value!({ "key": [val] }),
            V::object(
                vec![("key", V::list(vec![V::scalar(42)]))]
                    .into_iter()
                    .collect(),
            ),
        );
    }

    #[test]
    fn option() {
        let val = Some(42);

        assert_eq!(graphql_value!(None), V::Null);
        assert_eq!(graphql_value!(Some(42)), V::scalar(42));
        assert_eq!(graphql_value!(val), V::scalar(42));
    }
}
