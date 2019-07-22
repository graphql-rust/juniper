/**
## DEPRECATION WARNING

The `graphql_object!` macro is deprecated and will be removed soon.
Use the new [object proc macro](https://docs.rs/juniper_codegen/latest/juniper_codegen/attr.object.html) instead.

Expose GraphQL objects


This is a short-hand macro that implements the `GraphQLType` trait for a given
type. By using this macro instead of implementing it manually, you gain type
safety and reduce repetitive declarations.

# Examples

The simplest case exposes fields on a struct:

```rust
# extern crate juniper;
struct User { id: String, name: String, group_ids: Vec<String> }

juniper::graphql_object!(User: () |&self| {
    field id() -> &String {
        &self.id
    }

    field name() -> &String {
        &self.name
    }

    // Field and argument names will be converted from snake case to camel case,
    // as is the common naming convention in GraphQL. The following field would
    // be named "memberOfGroup", and the argument "groupId".
    field member_of_group(group_id: String) -> bool {
        self.group_ids.iter().any(|gid| gid == &group_id)
    }
});

# fn main() { }
```

## Documentation and descriptions

You can optionally add descriptions to the type itself, the fields,
and field arguments. For field and argument descriptions it is
possible to use normal rustdoc comments or doc
attributes. Alternatively the same syntax as for the type could be
used

```rust
# extern crate juniper;
struct User { id: String, name: String, group_ids: Vec<String> }

juniper::graphql_object!(User: () |&self| {
    description: "A user in the database"


    /// The user's unique identifier
    field id() -> &String {
        &self.id
    }

    field name() -> &String as "The user's name" {
        &self.name
    }

    #[doc = "Test if a user is member of a group"]
    field member_of_group(
        /// The group id you want to test membership against
        /// second line
        group_id: String
    ) -> bool {
        self.group_ids.iter().any(|gid| gid == &group_id)
    }
});

# fn main() { }
```

## Generics and lifetimes

You can expose generic or pointer types by prefixing the type with the necessary
generic parameters:

```rust
# extern crate juniper;
trait SomeTrait { fn id(&self) -> &str; }

juniper::graphql_object!(<'a> &'a SomeTrait: () as "SomeTrait" |&self| {
    field id() -> &str { self.id() }
});

struct GenericType<T> { items: Vec<T> }

juniper::graphql_object!(<T> GenericType<T>: () as "GenericType" |&self| {
    field count() -> i32 { self.items.len() as i32 }
});

# fn main() { }
```

## Implementing interfaces

You can use the `interfaces` item to implement interfaces:

```rust
# extern crate juniper;
trait Interface {
    fn id(&self) -> &str;
    fn as_implementor(&self) -> Option<Implementor>;
}
struct Implementor { id: String }

juniper::graphql_interface!(<'a> &'a Interface: () as "Interface" |&self| {
    field id() -> &str { self.id() }

    instance_resolvers: |&context| {
        Implementor => self.as_implementor(),
    }
});

juniper::graphql_object!(Implementor: () |&self| {
    field id() -> &str { &self.id }

    interfaces: [&Interface]
});

# fn main() { }
```

Note that the implementing type does not need to implement the trait on the Rust
side - only what's in the GraphQL schema matters. The GraphQL interface doesn't
even have to be backed by a trait!

## Emitting errors

`FieldResult<T, S = DefaultScalarValue>` is a type alias for `Result<T, FieldError<S>>`, where
`FieldError` is a tuple that contains an error message and optionally a
JSON-like data structure. In the end, errors that fields emit are serialized
into strings in the response. However, the execution system will keep track of
the source of all errors, and will continue executing despite some fields
failing.

Anything that implements `std::fmt::Display` can be converted to a `FieldError`
automatically via the `?` operator, or you can construct them yourself using
`FieldError::new`.

```
# extern crate juniper;
# use juniper::FieldResult;
struct User { id: String }

juniper::graphql_object!(User: () |&self| {
    field id() -> FieldResult<&String> {
        Ok(&self.id)
    }

    field name() -> FieldResult<&String> {
        Err("Does not have a name".to_owned())?
    }
});

# fn main() { }
```

## Specify scalar value representation

Sometimes it is necessary to use a other scalar value representation as the default
one provided by `DefaultScalarValue`.
It is possible to specify a specific scalar value type using the `where Scalar = Type`
syntax.
Additionally it is possible to use a generic parameter for the scalar value type
(in such a way that the type implements `GraphQLType` for all possible scalar value
representation). Similary to the specific type case the syntax here is
`where Scalar = <S>` where `S` is a freely choosable type parameter, that also could
be used as type parameter to the implementing type.

Example for using a generic scalar value type

```rust
# extern crate juniper;
struct User { id: String }

juniper::graphql_object!(User: () where Scalar = <S> |&self| {
    field id() -> &String {
        &self.id
    }

});

# fn main() { }
```

# Syntax

The top-most syntax of this macro defines which type to expose, the context
type, which lifetime parameters or generics to define,  which name to use in
the GraphQL schema and which scalar value type is used. It takes the
following form:

```text
<Generics> ExposedType: ContextType as "ExposedName" where Scalar = <S> |&self| { items... }
<Generics> ExposedType: ContextType as "ExposedName" where Scalar = SpecificType |&self| { items... }
```

The following parts are optional:
* `<Generics>`, if not set no generics are defined
* `as "ExposedName"`, if not set `ExposedType` is used as name
* `where Scalar = <S>` / `where Scalar = SpecificType` if not set `DefaultScalarValue`
is used as scalar value

## Items

Each item within the brackets of the top level declaration has its own syntax.
The order of individual items does not matter. `graphql_object!` supports a
number of different items.

### Top-level description

```text
description: "Top level description"
```

Adds documentation to the type in the schema, usable by tools such as GraphiQL.

### Interfaces

```text
interfaces: [&Interface, ...]
```

Informs the schema that the type implements the specified interfaces. This needs
to be _GraphQL_ interfaces, not necessarily Rust traits. The Rust types do not
need to have any connection, only what's exposed in the schema matters.

### Fields

```text
field name(args...) -> Type { }
field name(args...) -> Type as "Field description" { }
field deprecated "Reason" name(args...) -> Type { }
field deprecated "Reason" name(args...) -> Type as "Field description" { }
```

Defines a field on the object. The name is converted to camel case, e.g.
`user_name` is exposed as `userName`. The `as "Field description"` adds the
string as documentation on the field.

A field's description and deprecation can also be set using the
builtin `doc` and `deprecated` attributes.

```text
/// Field description
field name(args...) -> Type { }

#[doc = "Field description"]
field name(args...) -> Type {}

#[deprecated] // no reason required
field name(args...) -> Type { }

#[deprecated(note = "Reason")]
field name(args...) -> Type { }

/// Field description
#[deprecated(note = "Reason")] // deprecated must come after doc
field deprecated "Reason" name(args...) -> Type { }
```

### Field arguments

```text
&executor
arg_name: ArgType
arg_name = default_value: ArgType
arg_name: ArgType as "Argument description"
arg_name = default_value: ArgType as "Argument description"
```

Field arguments can take many forms. If the field needs access to the executor
or context, it can take an [Executor][1] instance by specifying `&executor`
as the first argument.

The other cases are similar to regular Rust arguments, with two additions:
argument documentation can be added by appending `as "Description"` after the
type, and a default value can be specified by appending `= value` after the
argument name.

Arguments are required (i.e. non-nullable) by default. If you specify _either_ a
default value, _or_ make the type into an `Option<>`, the argument becomes
optional. For example:

```text
arg_name: i32               -- required
arg_name: Option<i32>       -- optional, None if unspecified
arg_name = 123: i32         -- optional, "123" if unspecified
```

Due to some syntactical limitations in the macros, you must parentesize more
complex default value expressions:

```text
arg_name = (Point { x: 1, y: 2 }): Point
arg_name = ("default".to_owned()): String
```

A description can also be provided using normal doc comments or doc attributes.

```text
/// Argument description
arg_name: ArgType
#[doc = "Argument description"]
arg_name: ArgType
```

[1]: struct.Executor.html
*/
#[macro_export]
macro_rules! graphql_object {
    (
        @generate,
        meta = {
            lifetimes = [$($lifetimes:tt,)*],
            name = $name: ty,
            ctx = $ctx: ty,
            main_self = $main_self: ident,
            outname = {$($outname: tt)*},
            scalar = {$($scalar:tt)*},
            $(description = $desciption: expr,)*
            $(additional = {
                $(interfaces = [$($interface:ty,)*],)*
            },)*
        },
        items = [$({
            name = $fn_name: ident,
            body = $body: block,
            return_ty = $return_ty: ty,
            args = [$({
                arg_name = $arg_name : ident,
                arg_ty = $arg_ty: ty,
                $(arg_default = $arg_default: expr,)*
                $(arg_description = $arg_description: expr,)*
                $(arg_docstring = $arg_docstring: expr,)*
            },)*],
            $(decs = $fn_description: expr,)*
            $(docstring = $docstring: expr,)*
            $(deprecated = $deprecated: expr,)*
            $(executor_var = $executor: ident,)*
        },)*],
    ) => {
        $crate::__juniper_impl_trait!(
            impl<$($scalar)* $(, $lifetimes)* > GraphQLType for $name {
                type Context = $ctx;
                type TypeInfo = ();

                fn name(_ : &Self::TypeInfo) -> Option<&str> {
                    Some($($outname)*)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut $crate::Registry<'r, $crate::__juniper_insert_generic!($($scalar)+)>
                ) -> $crate::meta::MetaType<'r, $crate::__juniper_insert_generic!($($scalar)+)>
                where for<'__b> &'__b $crate::__juniper_insert_generic!($($scalar)+): $crate::ScalarRefValue<'__b>,
                    $crate::__juniper_insert_generic!($($scalar)+): 'r
                {
                    let fields = &[$(
                        registry.field_convert::<$return_ty, _, Self::Context>(
                            &$crate::to_camel_case(stringify!($fn_name)),
                            info
                        )
                            $(.description($fn_description))*
                            .push_docstring(&[$($docstring,)*])
                            $(.deprecated($deprecated))*
                            $(.argument(
                                $crate::__juniper_create_arg!(
                                    registry = registry,
                                    info = info,
                                    arg_ty = $arg_ty,
                                    arg_name = $arg_name,
                                    $(default = $arg_default,)*
                                    $(description = $arg_description,)*
                                    $(docstring = $arg_docstring,)*
                                )
                            ))*,
                    )*];
                    registry.build_object_type::<$name>(
                        info, fields
                    )
                        $(.description($desciption))*
                        $($(.interfaces(&[
                           $(registry.get_type::<$interface>(&()),)*
                        ]))*)*
                        .into_meta()
                }

                fn concrete_type_name(&self, _: &Self::Context, _: &Self::TypeInfo) -> String {
                    $($outname)*.to_owned()
                }

                #[allow(unused_variables)]
                fn resolve_field(
                    &$main_self,
                    info: &Self::TypeInfo,
                    field: &str,
                    args: &$crate::Arguments<$crate::__juniper_insert_generic!($($scalar)+)>,
                    executor: &$crate::Executor<Self::Context, $crate::__juniper_insert_generic!($($scalar)+)>
                ) -> $crate::ExecutionResult<$crate::__juniper_insert_generic!($($scalar)+)> {
                    $(
                        if field == &$crate::to_camel_case(stringify!($fn_name)) {
                            let result: $return_ty = (|| {
                                $(
                                    let $arg_name: $arg_ty = args.get(&$crate::to_camel_case(stringify!($arg_name)))
                                        .expect(concat!(
                                            "Argument ",
                                            stringify!($arg_name),
                                            " missing - validation must have failed"
                                        ));
                                )*
                                $(
                                    let $executor = &executor;
                                )*
                                $body
                            })();

                            return $crate::IntoResolvable::into(result, executor.context())
                                .and_then(|res| {
                                    match res {
                                        Some((ctx, r)) => {
                                            executor.replaced_context(ctx)
                                                .resolve_with_ctx(&(), &r)
                                        }
                                        None => Ok($crate::Value::null())
                                    }
                                });
                        }
                    )*

                    panic!("Field {} not found on type {}", field, $($outname)*);
                }
            }
        );
    };

    (
        @parse_interfaces,
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {
            lifetimes = [$($lifetime:tt,)*],
            name = $name:ty,
            ctx = $ctxt: ty,
            main_self = $mainself: ident,
            outname = {$($outname:tt)*},
            scalar = {$($scalar:tt)*},
            $(description = $desciption: tt,)*
            $(additional = {
                $(interfaces = [$($_interface:ty,)*],)*
            },)*

        },
        items = [$({$($items: tt)*},)*],
        rest = [$($interface: ty),+]  $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {
                lifetimes = [$($lifetime,)*],
                name = $name,
                ctx = $ctxt,
                main_self = $mainself,
                outname = {$($outname)*},
                scalar = {$($scalar)*},
                $(description = $desciption,)*
                additional = {
                        interfaces = [$($interface,)*],
                },
            },
            items = [$({$($items)*},)*],
            rest = $($rest)*
        );
    };

    (
        @parse_interfaces,
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = { $($meta:tt)* },
        items = [$({$($items: tt)*},)*],
        rest = interfaces: $($rest:tt)*
    ) => {
        $crate::graphql_object!(
            @parse_interfaces,
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = { $($meta)* },
            items = [$({$($items)*},)*],
            rest = $($rest)*
        );
    };


    (
        @parse,
        meta = {$($meta:tt)*},
        rest = $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = graphql_object,
            additional_parser = {
                callback = graphql_object,
                header = {@parse_interfaces,},
            },
            meta = {$($meta)*},
            items = [],
            rest = $($rest)*
        );
    };

    (@$($stuff:tt)*) => {
        compile_error!("Invalid syntax for `graphql_object!`");
    };

    (
        $($rest:tt)*
    ) => {
        $crate::__juniper_parse_object_header!(
            callback = graphql_object,
            rest = $($rest)*
        );
    }

}
