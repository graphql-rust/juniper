/**
Create an input object

Input objects are used as data carriers for complex input values to
fields and mutations. Unlike the other helper macros,
`graphql_input_object!` actually *creates* the struct you define. It
does not add anything to the struct definition itself - what you type
is what will be generated:

```rust
# #[macro_use] extern crate juniper;

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
        ( $($field_name:ident : $field_type:ty $(as $descr:tt)* $(,)* ),* )
    ) => {
        Some($name {
            $( $field_name: {
                let n: String = $crate::to_snake_case(stringify!($field_name));
                let v: Option<&&$crate::InputValue> = $var.get(&n[..]);

                if let Some(v) = v {
                    $crate::FromInputValue::from(v).unwrap()
                } else {
                    $crate::FromInputValue::from(&$crate::InputValue::null()).unwrap()
                }
            } ),*
        })
    };

    // Generate the struct declaration, including (Rust) meta attributes
    (
        @generate_struct_fields,
        ( $($meta:tt)* ), $name:tt,
        ( $($field_name:ident : $field_type:ty $(as $descr:tt)* $(,)* ),* )
    ) => {
        $($meta)* struct $name {
            $( $field_name: $field_type, )*
        }
    };

    // Generate the input field meta list, i.e. &[Argument].
    (
        @generate_meta_fields,
        $reg:tt,
        ( $($field_name:ident : $field_type:ty $(as $descr:tt)* $(,)* ),* )
    ) => {
        &[
            $(
                graphql_input_object!(
                    @apply_description,
                    $($descr)*,
                    $reg.arg::<$field_type>(
                        &$crate::to_snake_case(stringify!($field_name))))
            ),*
        ]
    };

    // #[...] struct $name { ... }
    // struct $name { ... }
    (
        @parse,
        ( $_ignore1:tt, $_ignore2:tt, $_ignore3:tt, $_ignore4:tt, $descr:tt ),
        $(#[$meta:meta])* struct $name:ident { $($fields:tt)* } $($rest:tt)*
    ) => {
        graphql_input_object!(
            @parse,
            ( ( $(#[$meta])* ), $name, (stringify!($name)), ($($fields)*), $descr ),
            $($rest)*
        );
    };

    // #[...] struct $name as "GraphQLName" { ... }
    // struct $name as "GraphQLName" { ... }
    (
        @parse,
        ( $_ignore1:tt, $_ignore2:tt, $_ignore3:tt, $_ignore4:tt, $descr:tt ),
        $(#[$meta:meta])* struct $name:ident as $outname:tt { $($fields:tt)* } $($rest:tt)*
    ) => {
        graphql_input_object!(
            @parse,
            ( ( $($meta)* ), $name, $outname, ($($fields)*), $descr ),
            $($rest)*
        );
    };

    // description: <description>
    (
        @parse,
        ( $meta:tt, $name:tt, $outname:tt, $fields:tt, $_ignore:tt ),
        description: $descr:tt $($rest:tt)*
    ) => {
        graphql_input_object!(
            @parse,
            ( $meta, $name, $outname, $fields, $descr ),
            $($rest)*
        );
    };

    // No more data to parse, generate the struct and impls
    (
        @parse,
        ( $meta:tt, $name:tt, $outname:tt, $fields:tt, $descr:tt ),
    ) => {
        graphql_input_object!(@generate_struct_fields, $meta, $name, $fields);

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

        impl $crate::GraphQLType for $name {
            type Context = ();

            fn name() -> Option<&'static str> {
                Some($outname)
            }

            fn meta(registry: &mut $crate::Registry) -> $crate::meta::MetaType {
                graphql_input_object!(
                    @maybe_apply, $descr, description,
                    registry.build_input_object_type::<$name>()(
                        graphql_input_object!(@generate_meta_fields, registry, $fields)
                    )).into_meta()
            }
        }
    };

    // Entry point: parse calls starting with the struct declaration
    ( $(#[$meta:meta])* struct $($items:tt)* ) => {
        graphql_input_object!(
            @parse,
            ( ( ), None, None, None, None ),
            $(#[$meta])* struct $($items)*
        );
    };

    // Entry point: parse calls starting with the description
    ( description: $($items:tt)* ) => {
        graphql_input_object!(
            @parse,
            ( ( ), None, None, None, None ),
            description: $($items)*
        );
    };
}
