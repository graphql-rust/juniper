/// Construct JSON-like [`InputValue`]s by using JSON syntax.
///
/// __Note:__ [`InputValue::List`]s and [`InputValue::Object`]s will be created
///           in a [`Spanning::unlocated`].
///
/// # Example
///
/// The resulting JSON will look just like what you passed in.
/// ```rust
/// # use juniper::{graphql_input_value, DefaultScalarValue, InputValue};
/// # type V = InputValue<DefaultScalarValue>;
/// #
/// # let _: V =
/// graphql_input_value!(null);
/// # let _: V =
/// graphql_input_value!(1234);
/// # let _: V =
/// graphql_input_value!("test");
/// # let _: V =
/// graphql_input_value!([1234, "test", true]);
/// # let _: V =
/// graphql_input_value!({"key": "value", "foo": 1234});
/// ```
#[macro_export]
macro_rules! graphql_input_value {
    ///////////
    // Array //
    ///////////

    // Done with trailing comma.
    (@@array [$($elems:expr,)*]) => {
        $crate::InputValue::list(vec![
            $( $elems, )*
        ])
    };

    // Done without trailing comma.
    (@@array [$($elems:expr),*]) => {
        $crate::InputValue::list(vec![
            $( $elems, )*
        ])
    };

    // Next element is `null`.
    (@@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::graphql_input_value!(
            @@array [$($elems,)* $crate::graphql_input_value!(null)] $($rest)*
        )
    };

    // Next element is `variable`.
    (@@array [$($elems:expr,)*] @$var:ident $($rest:tt)*) => {
        $crate::graphql_input_value!(
            @@array [$($elems,)* $crate::graphql_input_value!(@$var)] $($rest)*
        )
    };

    // Next element is `enum`.
    (@@array [$($elems:expr,)*] $enum:ident $($rest:tt)*) => {
        $crate::graphql_input_value!(
            @@array [$($elems,)* $crate::graphql_input_value!($enum)] $($rest)*
        )
    };


    // Next element is an array.
    (@@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::graphql_input_value!(
            @@array [$($elems,)* $crate::graphql_input_value!([$($array)*])] $($rest)*
        )
    };

    // Next element is a map.
    (@@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::graphql_input_value!(
            @@array [$($elems,)* $crate::graphql_input_value!({$($map)*})] $($rest)*
        )
    };

    // Next element is an expression followed by comma.
    (@@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::graphql_input_value!(
            @@array [$($elems,)* $crate::graphql_input_value!($next),] $($rest)*
        )
    };

    // Last element is an expression with no trailing comma.
    (@@array [$($elems:expr,)*] $last:expr) => {
        $crate::graphql_input_value!(
            @@array [$($elems,)* $crate::graphql_input_value!($last)]
        )
    };

    // Comma after the most recent element.
    (@@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::graphql_input_value!(@@array [$($elems,)*] $($rest)*)
    };

    // Unexpected token after most recent element.
    (@@array [$($elems:expr),*] $unexpected:tt $($rest:tt)*) => {
        $crate::graphql_input_value!(@unexpected $unexpected)
    };

    ////////////
    // Object //
    ////////////

    // Done.
    (@@object $object:ident () () ()) => {};

    // Insert the current entry followed by trailing comma.
    (@@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.push(($crate::Spanning::unlocated(($($key)+).into()), $crate::Spanning::unlocated($value)));
        $crate::graphql_input_value!(@@object $object () ($($rest)*) ($($rest)*));
    };

    // Current entry followed by unexpected token.
    (@@object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        $crate::graphql_input_value!(@unexpected $unexpected);
    };

    // Insert the last entry without trailing comma.
    (@@object $object:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $object.push(($crate::Spanning::unlocated(($($key)+).into()), $crate::Spanning::unlocated($value)));
    };

    // Next value is `null`.
    (@@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::graphql_input_value!(@@object $object [$($key)+] ($crate::graphql_input_value!(null)) $($rest)*);
    };

    // Next value is `variable`.
    (@@object $object:ident ($($key:tt)+) (: @$var:ident $($rest:tt)*) $copy:tt) => {
        $crate::graphql_input_value!(@@object $object [$($key)+] ($crate::graphql_input_value!(@$var)) $($rest)*);
    };

    // Next value is `enum`.
    (@@object $object:ident ($($key:tt)+) (: $enum:ident $($rest:tt)*) $copy:tt) => {
        $crate::graphql_input_value!(@@object $object [$($key)+] ($crate::graphql_input_value!($enum)) $($rest)*);
    };

    // Next value is an array.
    (@@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::graphql_input_value!(@@object $object [$($key)+] ($crate::graphql_input_value!([$($array)*])) $($rest)*);
    };

    // Next value is a map.
    (@@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::graphql_input_value!(@@object $object [$($key)+] ($crate::graphql_input_value!({$($map)*})) $($rest)*);
    };

    // Next value is an expression followed by comma.
    (@@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::graphql_input_value!(@@object $object [$($key)+] ($crate::graphql_input_value!($value)) , $($rest)*);
    };

    // Last value is an expression with no trailing comma.
    (@@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::graphql_input_value!(@@object $object [$($key)+] ($crate::graphql_input_value!($value)));
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::graphql_input_value!();
    };

    // Missing colon and value for last entry. Trigger a reasonable error
    // message.
    (@@object $object:ident ($($key:tt)+) () $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::graphql_input_value!();
    };

    // Misplaced colon. Trigger a reasonable error message.
    (@@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        $crate::graphql_input_value!(@unexpected $colon);
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        $crate::graphql_input_value!(@unexpected $comma);
    };

    // Key is fully parenthesized. This avoids clippy double_parens false
    // positives because the parenthesization may be necessary here.
    (@@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::graphql_input_value!(@@object $object ($key) (: $($rest)*) (: $($rest)*));
    };

    // Refuse to absorb colon token into key expression.
    (@@object $object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        $crate::graphql_input_value!(@unexpected $($unexpected)+);
    };

    // Munch a token into the current key.
    (@@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::graphql_input_value!(@@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    ////////////
    // Errors //
    ////////////

    (@unexpected) => {};

    //////////////
    // Defaults //
    //////////////

    ([ $($arr:tt)* ]) => {
        $crate::graphql_input_value!(@@array [] $($arr)*)
    };

    ({}) => {
        $crate::InputValue::parsed_object(vec![])
    };

    ({ $($map:tt)+ }) => {
        $crate::InputValue::parsed_object({
            let mut object = vec![];
            $crate::graphql_input_value!(@@object object () ($($map)*) ($($map)*));
            object
        })
    };

    (null) => ($crate::InputValue::null());

    (@$var:ident) => ($crate::InputValue::variable(stringify!($var)));

    ($enum:ident) => ($crate::InputValue::enum_value(stringify!($enum)));

    (($e:expr)) => ($crate::InputValue::scalar($e));

    ($e:expr) => ($crate::InputValue::scalar($e));
}

#[cfg(test)]
mod tests {
    use indexmap::{indexmap, IndexMap};

    use crate::{DefaultScalarValue, InputValue};

    type V = InputValue<DefaultScalarValue>;

    #[test]
    fn null() {
        assert_eq!(graphql_input_value!(null), V::Null);
    }

    #[test]
    fn scalar() {
        let val = 42;
        assert_eq!(graphql_input_value!(1), V::scalar(1));
        assert_eq!(graphql_input_value!("val"), V::scalar("val"));
        assert_eq!(graphql_input_value!(1.34), V::scalar(1.34));
        assert_eq!(graphql_input_value!(false), V::scalar(false));
        assert_eq!(graphql_input_value!(1 + 2), V::scalar(3));
        assert_eq!(graphql_input_value!((val)), V::scalar(42));
    }

    #[test]
    fn enums() {
        assert_eq!(graphql_input_value!(ENUM), V::enum_value("ENUM"));
        assert_eq!(graphql_input_value!(lowercase), V::enum_value("lowercase"));
    }

    #[test]
    fn variables() {
        assert_eq!(graphql_input_value!(@var), V::variable("var"));
        assert_eq!(graphql_input_value!(@array), V::variable("array"));
        assert_eq!(graphql_input_value!(@object), V::variable("object"));
    }

    #[test]
    fn lists() {
        let val = 42;
        assert_eq!(graphql_input_value!([]), V::list(vec![]));

        assert_eq!(graphql_input_value!([null]), V::list(vec![V::Null]));

        assert_eq!(graphql_input_value!([1]), V::list(vec![V::scalar(1)]));
        assert_eq!(graphql_input_value!([1 + 2]), V::list(vec![V::scalar(3)]));
        assert_eq!(graphql_input_value!([(val)]), V::list(vec![V::scalar(42)]));

        assert_eq!(
            graphql_input_value!([ENUM]),
            V::list(vec![V::enum_value("ENUM")]),
        );
        assert_eq!(
            graphql_input_value!([lowercase]),
            V::list(vec![V::enum_value("lowercase")]),
        );

        assert_eq!(
            graphql_input_value!([@var]),
            V::list(vec![V::variable("var")]),
        );
        assert_eq!(
            graphql_input_value!([@array]),
            V::list(vec![V::variable("array")]),
        );
        assert_eq!(
            graphql_input_value!([@object]),
            V::list(vec![V::variable("object")]),
        );

        assert_eq!(
            graphql_input_value!([1, [2], 3]),
            V::list(vec![
                V::scalar(1),
                V::list(vec![V::scalar(2)]),
                V::scalar(3),
            ]),
        );
        assert_eq!(
            graphql_input_value!([1, [2 + 3], 3]),
            V::list(vec![
                V::scalar(1),
                V::list(vec![V::scalar(5)]),
                V::scalar(3),
            ]),
        );
        assert_eq!(
            graphql_input_value!([1, [ENUM], (val),]),
            V::list(vec![
                V::scalar(1),
                V::list(vec![V::enum_value("ENUM")]),
                V::scalar(42),
            ]),
        );
        assert_eq!(
            graphql_input_value!([1 + 2, [(val)], @val,]),
            V::list(vec![
                V::scalar(3),
                V::list(vec![V::scalar(42)]),
                V::variable("val"),
            ]),
        );
        assert_eq!(
            graphql_input_value!([1, [@val], ENUM,]),
            V::list(vec![
                V::scalar(1),
                V::list(vec![V::variable("val")]),
                V::enum_value("ENUM"),
            ]),
        );
        assert_eq!(
            graphql_input_value!({ "key": ENUM }),
            V::object(indexmap! { "key" => V::enum_value("ENUM") }),
        );
        assert_eq!(
            graphql_input_value!({ "key": lowercase }),
            V::object(indexmap! { "key" => V::enum_value("lowercase") }),
        );
        assert_eq!(
            graphql_input_value!({ "key": @var }),
            V::object(indexmap! { "key" => V::variable("var") }),
        );
        assert_eq!(
            graphql_input_value!({ "key": @array }),
            V::object(indexmap! { "key" => V::variable("array") }),
        );
    }

    #[test]
    fn objects() {
        let val = 42;
        assert_eq!(
            graphql_input_value!({}),
            V::object(IndexMap::<String, _>::new()),
        );

        assert_eq!(
            graphql_input_value!({ "key": null }),
            V::object(indexmap! { "key" => V::Null }),
        );

        assert_eq!(
            graphql_input_value!({ "key": 123 }),
            V::object(indexmap! { "key" => V::scalar(123) }),
        );
        assert_eq!(
            graphql_input_value!({ "key": 1 + 2 }),
            V::object(indexmap! { "key" => V::scalar(3) }),
        );
        assert_eq!(
            graphql_input_value!({ "key": (val) }),
            V::object(indexmap! { "key" => V::scalar(42) }),
        );

        assert_eq!(
            graphql_input_value!({ "key": [] }),
            V::object(indexmap! { "key" => V::list(vec![]) }),
        );
        assert_eq!(
            graphql_input_value!({ "key": [null] }),
            V::object(indexmap! { "key" => V::list(vec![V::Null]) }),
        );
        assert_eq!(
            graphql_input_value!({ "key": [1] }),
            V::object(indexmap! { "key" => V::list(vec![V::scalar(1)]) }),
        );
        assert_eq!(
            graphql_input_value!({ "key": [1 + 2] }),
            V::object(indexmap! { "key" => V::list(vec![V::scalar(3)]) }),
        );
        assert_eq!(
            graphql_input_value!({ "key": [(val)] }),
            V::object(indexmap! { "key" => V::list(vec![V::scalar(42)]) }),
        );
        assert_eq!(
            graphql_input_value!({ "key": ENUM }),
            V::object(indexmap! { "key" => V::enum_value("ENUM") }),
        );
        assert_eq!(
            graphql_input_value!({ "key": lowercase }),
            V::object(indexmap! { "key" => V::enum_value("lowercase") }),
        );
        assert_eq!(
            graphql_input_value!({ "key": @val }),
            V::object(indexmap! { "key" => V::variable("val") }),
        );
        assert_eq!(
            graphql_input_value!({ "key": @array }),
            V::object(indexmap! { "key" => V::variable("array") }),
        );

        assert_eq!(
            graphql_input_value!({
                "inner": {
                    "key1": (val),
                    "key2": "val",
                    "key3": [
                        {
                            "inner": 42,
                        },
                        {
                            "inner": ENUM,
                            "even-more": {
                                "var": @var,
                            },
                        }
                    ],
                    "key4": [1, ["val", 1 + 3], null, @array],
                },
                "more": @var,
            }),
            V::object(indexmap! {
                "inner" => V::object(indexmap! {
                    "key1" => V::scalar(42),
                    "key2" => V::scalar("val"),
                    "key3" => V::list(vec![
                        V::object(indexmap! {
                            "inner" => V::scalar(42),
                        }),
                        V::object(indexmap! {
                            "inner" => V::enum_value("ENUM"),
                            "even-more" => V::object(indexmap! {
                                "var" => V::variable("var"),
                            })
                        }),
                    ]),
                    "key4" => V::list(vec![
                        V::scalar(1),
                        V::list(vec![
                            V::scalar("val"),
                            V::scalar(4),
                        ]),
                        V::Null,
                        V::variable("array"),
                    ]),
                }),
                "more" => V::variable("var"),
            }),
        );
    }
}
