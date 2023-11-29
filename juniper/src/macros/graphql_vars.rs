//! [`graphql_vars!`] macro implementation.
//!
//! [`graphql_vars!`]: graphql_vars

/// Constructs [`Variables`] via JSON-like syntax.
///
/// [`Variables`] key should implement [`Into`]`<`[`String`]`>`.
/// ```rust
/// # use std::borrow::Cow;
/// #
/// # use juniper::{graphql_vars, Variables};
/// #
/// let code = 200;
/// let features = vec!["key", "value"];
/// let key: Cow<'static, str> = "key".into();
///
/// let value: Variables = graphql_vars! {
///     "code": code,
///     "success": code == 200,
///     features[0]: features[1],
///     key: @var,
/// };
/// ```
///
/// See [`graphql_input_value!`] for more info on syntax of value after `:`.
///
/// [`graphql_input_value!`]: crate::graphql_input_value
/// [`Variables`]: crate::Variables
#[macro_export]
macro_rules! graphql_vars {
    ////////////
    // Object //
    ////////////

    // Done.
    (@object $object:ident () () ()) => {};

    // Insert the current entry followed by trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.insert(($($key)+).into(), $value);
        $crate::graphql_vars! {@object $object () ($($rest)*) ($($rest)*)};
    };

    // Current entry followed by unexpected token.
    (@object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        $crate::graphql_vars! {@unexpected $unexpected};
    };

    // Insert the last entry without trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $object.insert(($($key)+).into(), $value);
    };

    // Next value is `null`.
    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            [$($key)+]
            ($crate::graphql_input_value!(null)) $($rest)*
        };
    };

    // Next value is `None`.
    (@object $object:ident ($($key:tt)+) (: None $($rest:tt)*) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            [$($key)+]
            ($crate::graphql_input_value!(None)) $($rest)*
        };
    };

    // Next value is a variable.
    (@object $object:ident ($($key:tt)+) (: @$var:ident $($rest:tt)*) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            [$($key)+]
            ($crate::graphql_input_value!(@$var)) $($rest)*
        };
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            [$($key)+]
            ($crate::graphql_input_value!([$($array)*])) $($rest)*
        };
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            [$($key)+]
            ($crate::graphql_input_value!({$($map)*})) $($rest)*
        };
    };

    // Next value is `true`, `false` or enum ident followed by a comma.
    (@object $object:ident ($($key:tt)+) (: $ident:ident , $($rest:tt)*) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            [$($key)+]
            ($crate::graphql_input_value!($ident)) , $($rest)*
        };
    };

    // Next value is `true`, `false` or enum ident without trailing comma.
    (@object $object:ident ($($key:tt)+) (: $last:ident ) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            [$($key)+]
            ($crate::graphql_input_value!($last))
        };
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            [$($key)+]
            ($crate::graphql_input_value!($value)) , $($rest)*
        };
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            [$($key)+]
            ($crate::graphql_input_value!($value))
        };
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::graphql_vars! {};
    };

    // Missing colon and value for last entry. Trigger a reasonable error
    // message.
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::graphql_vars! {};
    };

    // Misplaced colon. Trigger a reasonable error message.
    (@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        $crate::graphql_vars! {@unexpected $colon};
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        $crate::graphql_vars! {@unexpected $comma};
    };

    // Key is fully parenthesized. This avoids clippy double_parens false
    // positives because the parenthesization may be necessary here.
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object ($key) (: $($rest)*) (: $($rest)*)
        };
    };

    // Refuse to absorb colon token into key expression.
    (@object $object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        $crate::graphql_vars! {@unexpected $($unexpected)+};
    };

    // Munch a token into the current key.
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::graphql_vars! {
            @object $object
            ($($key)* $tt)
            ($($rest)*) ($($rest)*)
        };
    };

    ////////////
    // Errors //
    ////////////

    (@unexpected) => {};

    //////////////
    // Defaults //
    //////////////

    () => {{ $crate::Variables::<_>::new() }};

    ( $($map:tt)+ ) => {{
        let mut object = $crate::Variables::<_>::new();
        $crate::graphql_vars! {@object object () ($($map)*) ($($map)*)};
        object
    }};
}

#[cfg(test)]
mod tests {
    use indexmap::{indexmap, IndexMap};

    type V = crate::Variables;

    type IV = crate::InputValue;

    #[test]
    fn empty() {
        assert_eq!(graphql_vars! {}, V::new());
    }

    #[test]
    fn scalar() {
        let val = 42;

        assert_eq!(
            graphql_vars! {"key": 123},
            vec![("key".into(), IV::scalar(123))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": "val"},
            vec![("key".into(), IV::scalar("val"))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": 1.23},
            vec![("key".into(), IV::scalar(1.23))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": 1 + 2},
            vec![("key".into(), IV::scalar(3))].into_iter().collect(),
        );
        assert_eq!(
            graphql_vars! {"key": false},
            vec![("key".into(), IV::scalar(false))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": (val)},
            vec![("key".into(), IV::scalar(42))]
                .into_iter()
                .collect::<V>(),
        );
    }

    #[test]
    fn r#enum() {
        assert_eq!(
            graphql_vars! {"key": ENUM},
            vec![("key".into(), IV::enum_value("ENUM"))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": lowercase},
            vec![("key".into(), IV::enum_value("lowercase"))]
                .into_iter()
                .collect::<V>(),
        );
    }

    #[test]
    fn variable() {
        assert_eq!(
            graphql_vars! {"key": @var},
            vec![("key".into(), IV::variable("var"))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": @array},
            vec![("key".into(), IV::variable("array"))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": @object},
            vec![("key".into(), IV::variable("object"))]
                .into_iter()
                .collect::<V>(),
        );
    }

    #[test]
    fn list() {
        let val = 42;

        assert_eq!(
            graphql_vars! {"key": []},
            vec![("key".into(), IV::list(vec![]))]
                .into_iter()
                .collect::<V>(),
        );

        assert_eq!(
            graphql_vars! {"key": [null]},
            vec![("key".into(), IV::list(vec![IV::Null]))]
                .into_iter()
                .collect::<V>(),
        );

        assert_eq!(
            graphql_vars! {"key": [1]},
            vec![("key".into(), IV::list(vec![IV::scalar(1)]))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": [1 + 2]},
            vec![("key".into(), IV::list(vec![IV::scalar(3)]))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": [(val)]},
            vec![("key".into(), IV::list(vec![IV::scalar(42)]))]
                .into_iter()
                .collect::<V>(),
        );

        assert_eq!(
            graphql_vars! {"key": [ENUM]},
            vec![("key".into(), IV::list(vec![IV::enum_value("ENUM")]))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": [lowercase]},
            vec![("key".into(), IV::list(vec![IV::enum_value("lowercase")]))]
                .into_iter()
                .collect::<V>(),
        );

        assert_eq!(
            graphql_vars! {"key": [@var]},
            vec![("key".into(), IV::list(vec![IV::variable("var")]))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": [@array]},
            vec![("key".into(), IV::list(vec![IV::variable("array")]))]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": [@object]},
            vec![("key".into(), IV::list(vec![IV::variable("object")]))]
                .into_iter()
                .collect::<V>(),
        );

        assert_eq!(
            graphql_vars! {"key": [1, [2], 3]},
            vec![(
                "key".into(),
                IV::list(vec![
                    IV::scalar(1),
                    IV::list(vec![IV::scalar(2)]),
                    IV::scalar(3),
                ]),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": [1, [2 + 3], 3]},
            vec![(
                "key".into(),
                IV::list(vec![
                    IV::scalar(1),
                    IV::list(vec![IV::scalar(5)]),
                    IV::scalar(3),
                ]),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": [1, [ENUM], (val)]},
            vec![(
                "key".into(),
                IV::list(vec![
                    IV::scalar(1),
                    IV::list(vec![IV::enum_value("ENUM")]),
                    IV::scalar(42),
                ]),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": [1 + 2, [(val)], @val]},
            vec![(
                "key".into(),
                IV::list(vec![
                    IV::scalar(3),
                    IV::list(vec![IV::scalar(42)]),
                    IV::variable("val"),
                ]),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": [1, [@val], ENUM]},
            vec![(
                "key".into(),
                IV::list(vec![
                    IV::scalar(1),
                    IV::list(vec![IV::variable("val")]),
                    IV::enum_value("ENUM"),
                ]),
            )]
            .into_iter()
            .collect::<V>(),
        );
    }

    #[test]
    fn object() {
        let val = 42;

        assert_eq!(
            graphql_vars! {"key": {}},
            vec![("key".into(), IV::object(IndexMap::<String, _>::new()))]
                .into_iter()
                .collect::<V>(),
        );

        assert_eq!(
            graphql_vars! {"key": {"key": null}},
            vec![("key".into(), IV::object(indexmap! {"key" => IV::Null}))]
                .into_iter()
                .collect::<V>(),
        );

        assert_eq!(
            graphql_vars! {"key": {"key": 123}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::scalar(123)}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": 1 + 2}},
            vec![("key".into(), IV::object(indexmap! {"key" => IV::scalar(3)}),)]
                .into_iter()
                .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": (val)}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::scalar(42)}),
            )]
            .into_iter()
            .collect::<V>(),
        );

        assert_eq!(
            graphql_vars! {"key": {"key": []}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::list(vec![])}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": [null]}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::list(vec![IV::Null])}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": [1]}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::list(vec![IV::scalar(1)])}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": [1 + 2]}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::list(vec![IV::scalar(3)])}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": [(val)]}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::list(vec![IV::scalar(42)])}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": ENUM}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::enum_value("ENUM")}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": lowercase}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::enum_value("lowercase")}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": @val}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::variable("val")}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {"key": {"key": @array}},
            vec![(
                "key".into(),
                IV::object(indexmap! {"key" => IV::variable("array")}),
            )]
            .into_iter()
            .collect::<V>(),
        );
        assert_eq!(
            graphql_vars! {
                "inner": {
                    "key1": (val),
                    "key2": "val",
                    "key3": [{
                        "inner": 42,
                    }, {
                        "inner": ENUM,
                        "even-more": {
                            "var": @var,
                        },
                    }],
                    "key4": [1, ["val", 1 + 3], null, @array],
                },
                "more": @var,
            },
            vec![
                (
                    "inner".into(),
                    IV::object(indexmap! {
                        "key1" => IV::scalar(42),
                        "key2" => IV::scalar("val"),
                        "key3" => IV::list(vec![
                            IV::object(indexmap! {
                                "inner" => IV::scalar(42),
                            }),
                            IV::object(indexmap! {
                                "inner" => IV::enum_value("ENUM"),
                                "even-more" => IV::object(indexmap! {
                                    "var" => IV::variable("var"),
                                }),
                            }),
                        ]),
                        "key4" => IV::list(vec![
                            IV::scalar(1),
                            IV::list(vec![
                                IV::scalar("val"),
                                IV::scalar(4),
                            ]),
                            IV::Null,
                            IV::variable("array"),
                        ]),
                    }),
                ),
                ("more".into(), IV::variable("var")),
            ]
            .into_iter()
            .collect::<V>(),
        );
    }
}
