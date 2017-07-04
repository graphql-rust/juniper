/**
Create an input object

Input objects are used as data carriers for complex input values to
fields and mutations. Unlike the other helper macros,
`graphql_input_object!` actually *creates* the struct you define. It
does not add anything to the struct definition itself - what you type
is what will be generated:

```rust
# #[macro_use] extern crate juniper;
#
graphql_input_object!(
    description: "Coordinates for the user"

    struct Coordinates {
        longitude: f64 as "The X coordinate, from -180 to +180",
        latitude: f64 as "The Y coordinate, from -90 to +90",
    }
);

# fn main() { }
```

This macro creates the struct as specified and implements
`FromInputValue` to automatically parse values provided from variables
and arguments.

If you want to expose the struct under a different name than the Rust
type, you can write `struct Coordinates as "MyCoordinates" { ...`.

You can specify *default values* for input object fields; the syntax
is similar to argument default values:

```rust
# #[macro_use] extern crate juniper;
#
graphql_input_object!(
    struct SampleObject {
        foo = 123: i32 as "A sample field, defaults to 123 if omitted"
    }
);

# fn main() { }
```

*/
#[macro_export]
macro_rules! graphql_input_object {
    // Calls $val.$func($arg) if $arg is not None
    ( @maybe_apply, None, $func:ident, $val:expr ) => { $val };
    ( @maybe_apply, $arg:tt, $func:ident, $val:expr ) => { $val.$func($arg) };

    // Calls $val.description($descr) when $descr is not empty
    ( @apply_description, , $val:expr ) => { $val };
    ( @apply_description, $descr:tt , $val:expr ) => { $val.description($descr) };

    // Generate the FromInputValue::from method body, provided a
    // HashMap<&str, &InputValue> in $var
    (
        @generate_from_input_value,
        $name:tt, $var:tt,
        ( $($field_name:ident $(= $default:tt)* : $field_type:ty $(as $descr:tt)* $(,)* ),* )
    ) => {
        Some($name {
            $( $field_name: {
                let n = $crate::to_camel_case(stringify!($field_name));
                let v: Option<&&$crate::InputValue> = $var.get(&n[..]);

                match v {
                    $( Some(&&$crate::InputValue::Null) | None if true => $default, )*
                        Some(v) => $crate::FromInputValue::from(v).unwrap(),
                        _ => $crate::FromInputValue::from(&$crate::InputValue::null()).unwrap()
                }
            } ),*
        })
    };

    // Generate the ToInputValue::To method body, provided self in $self
    (
        @generate_to_input_value,
        $name:tt, $selfvar:tt,
        ( $($field_name:ident $(= $default:tt)* : $field_type:ty $(as $descr:tt)* $(,)* ),* )
    ) => {
        $crate::InputValue::object(vec![
            $(
                ($crate::to_camel_case(stringify!($field_name)), $selfvar.$field_name.to())
            ),*
        ].into_iter().collect())
    };

    // Generate the struct declaration, including (Rust) meta attributes
    (
        @generate_struct_fields,
        ( $($meta:tt)* ), ( $($pubmod:tt)* ), $name:tt,
        ( $($field_name:ident $(= $default:tt)* : $field_type:ty $(as $descr:tt)* $(,)* ),* )
    ) => {
        $($meta)* $($pubmod)* struct $name {
            $( $field_name: $field_type, )*
        }
    };

    // Generate single field meta for field with default value
    (
        @generate_single_meta_field,
        $reg:tt,
        ( $field_name:ident = $default:tt : $field_type:ty $(as $descr:tt)* )
    ) => {
        graphql_input_object!(
            @apply_description,
            $($descr)*,
            $reg.arg_with_default::<$field_type>(
                &$crate::to_camel_case(stringify!($field_name)),
                &$default))
    };

    // Generate single field meta for field without default value
    (
        @generate_single_meta_field,
        $reg:tt,
        ( $field_name:ident : $field_type:ty $(as $descr:tt)* )
    ) => {
        graphql_input_object!(
            @apply_description,
            $($descr)*,
            $reg.arg::<$field_type>(
                &$crate::to_camel_case(stringify!($field_name))))
    };

    // Generate the input field meta list, i.e. &[Argument] for
    (
        @generate_meta_fields,
        $reg:tt,
        ( $($field_name:ident $(= $default:tt)* : $field_type:ty $(as $descr:tt)* $(,)* ),* )
    ) => {
        &[
            $(
                graphql_input_object!(
                    @generate_single_meta_field,
                    $reg,
                    ( $field_name $(= $default)* : $field_type $(as $descr)* )
                )
            ),*
        ]
    };

    // #[...] struct $name { ... }
    // struct $name { ... }
    (
        @parse,
        ( $_ignore1:tt, $_ignore2:tt, $_ignore3:tt, $_ignore4:tt, $_ignore5:tt, $descr:tt ),
        $(#[$meta:meta])* struct $name:ident { $($fields:tt)* } $($rest:tt)*
    ) => {
        graphql_input_object!(
            @parse,
            ( ( $(#[$meta])* ), ( ), $name, (stringify!($name)), ($($fields)*), $descr ),
            $($rest)*
        );
    };

    // #[...] pub struct $name { ... }
    // pub struct $name { ... }
    (
        @parse,
        ( $_ignore1:tt, $_ignore2:tt, $_ignore3:tt, $_ignore4:tt, $_ignore5:tt, $descr:tt ),
        $(#[$meta:meta])* pub struct $name:ident { $($fields:tt)* } $($rest:tt)*
    ) => {
        graphql_input_object!(
            @parse,
            ( ( $(#[$meta])* ), ( pub ), $name, (stringify!($name)), ($($fields)*), $descr ),
            $($rest)*
        );
    };

    // #[...] struct $name as "GraphQLName" { ... }
    // struct $name as "GraphQLName" { ... }
    (
        @parse,
        ( $_ignore1:tt, $_ignore2:tt, $_ignore3:tt, $_ignore4:tt, $_ignore5:tt, $descr:tt ),
        $(#[$meta:meta])* struct $name:ident as $outname:tt { $($fields:tt)* } $($rest:tt)*
    ) => {
        graphql_input_object!(
            @parse,
            ( ( $($meta)* ), ( ), $name, $outname, ($($fields)*), $descr ),
            $($rest)*
        );
    };

    // #[...] pub struct $name as "GraphQLName" { ... }
    // pub struct $name as "GraphQLName" { ... }
    (
        @parse,
        ( $_ignore1:tt, $_ignore2:tt, $_ignore3:tt, $_ignore4:tt, $_ignore5:tt, $descr:tt ),
        $(#[$meta:meta])* pub struct $name:ident as $outname:tt { $($fields:tt)* } $($rest:tt)*
    ) => {
        graphql_input_object!(
            @parse,
            ( ( $($meta)* ), ( pub ), $name, $outname, ($($fields)*), $descr ),
            $($rest)*
        );
    };

    // description: <description>
    (
        @parse,
        ( $meta:tt, $pubmod:tt, $name:tt, $outname:tt, $fields:tt, $_ignore:tt ),
        description: $descr:tt $($rest:tt)*
    ) => {
        graphql_input_object!(
            @parse,
            ( $meta, $pubmod, $name, $outname, $fields, $descr ),
            $($rest)*
        );
    };

    // No more data to parse, generate the struct and impls
    (
        @parse,
        ( $meta:tt, $pubmod:tt, $name:tt, $outname:tt, $fields:tt, $descr:tt ),
    ) => {
        graphql_input_object!(@generate_struct_fields, $meta, $pubmod, $name, $fields);

        impl $crate::FromInputValue for $name {
            fn from(value: &$crate::InputValue) -> Option<$name> {
                if let Some(obj) = value.to_object_value() {
                    graphql_input_object!(@generate_from_input_value, $name, obj, $fields)
                }
                else {
                   None
                }
            }
        }

        impl $crate::ToInputValue for $name {
            fn to(&self) -> $crate::InputValue {
                graphql_input_object!(@generate_to_input_value, $name, self, $fields)
            }
        }

        impl $crate::GraphQLType for $name {
            type Context = ();

            fn name() -> Option<&'static str> {
                Some($outname)
            }

            fn meta<'r>(registry: &mut $crate::Registry<'r>) -> $crate::meta::MetaType<'r> {
                let fields = graphql_input_object!(@generate_meta_fields, registry, $fields);
                graphql_input_object!(
                    @maybe_apply, $descr, description,
                    registry.build_input_object_type::<$name>(fields)).into_meta()
            }
        }
    };

    // Entry point: parse calls starting with a struct declaration
    ( $(#[$meta:meta])* struct $($items:tt)* ) => {
        graphql_input_object!(
            @parse,
            ( ( ), ( ), None, None, None, None ),
            $(#[$meta])* struct $($items)*
        );
    };

    // Entry point: parse calls starting with a public struct declaration
    ( $(#[$meta:meta])* pub struct $($items:tt)* ) => {
        graphql_input_object!(
            @parse,
            ( ( ), ( ), None, None, None, None ),
            $(#[$meta])* pub struct $($items)*
        );
    };

    // Entry point: parse calls starting with the description
    ( description: $($items:tt)* ) => {
        graphql_input_object!(
            @parse,
            ( ( ), ( ), None, None, None, None ),
            description: $($items)*
        );
    };
}
