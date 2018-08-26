/**
Expose GraphQL objects

This is a short-hand macro that implements the `GraphQLType` trait for a given
type. By using this macro instead of implementing it manually, you gain type
safety and reduce repetitive declarations.

# Examples

The simplest case exposes fields on a struct:

```rust
# #[macro_use] extern crate juniper;
struct User { id: String, name: String, group_ids: Vec<String> }

graphql_object!(User: () |&self| {
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

You can optionally add descriptions to the type itself, the fields, and field
arguments:

```rust
# #[macro_use] extern crate juniper;
struct User { id: String, name: String, group_ids: Vec<String> }

graphql_object!(User: () |&self| {
    description: "A user in the database"

    field id() -> &String as "The user's unique identifier" {
        &self.id
    }

    field name() -> &String as "The user's name" {
        &self.name
    }

    field member_of_group(
        group_id: String as "The group id you want to test membership against"
    ) -> bool as "Test if a user is member of a group" {
        self.group_ids.iter().any(|gid| gid == &group_id)
    }
});

# fn main() { }
```

## Generics and lifetimes

You can expose generic or pointer types by prefixing the type with the necessary
generic parameters:

```rust
# #[macro_use] extern crate juniper;
trait SomeTrait { fn id(&self) -> &str; }

graphql_object!(<'a> &'a SomeTrait: () as "SomeTrait" |&self| {
    field id() -> &str { self.id() }
});

struct GenericType<T> { items: Vec<T> }

graphql_object!(<T> GenericType<T>: () as "GenericType" |&self| {
    field count() -> i32 { self.items.len() as i32 }
});

# fn main() { }
```

## Implementing interfaces

You can use the `interfaces` item to implement interfaces:

```rust
# #[macro_use] extern crate juniper;
trait Interface {
    fn id(&self) -> &str;
    fn as_implementor(&self) -> Option<Implementor>;
}
struct Implementor { id: String }

graphql_interface!(<'a> &'a Interface: () as "Interface" |&self| {
    field id() -> &str { self.id() }

    instance_resolvers: |&context| {
        Implementor => self.as_implementor(),
    }
});

graphql_object!(Implementor: () |&self| {
    field id() -> &str { &self.id }

    interfaces: [&Interface]
});

# fn main() { }
```

Note that the implementing type does not need to implement the trait on the Rust
side - only what's in the GraphQL schema matters. The GraphQL interface doesn't
even have to be backed by a trait!

## Emitting errors

`FieldResult<T>` is a type alias for `Result<T, FieldError>`, where
`FieldResult` is a tuple that contains an error message and optionally a
JSON-like data structure. In the end, errors that fields emit are serialized
into strings in the response. However, the execution system will keep track of
the source of all errors, and will continue executing despite some fields
failing.

Anything that implements `std::fmt::Display` can be converted to a `FieldError`
automatically via the `?` operator, or you can construct them yourself using
`FieldError::new`.

```
# #[macro_use] extern crate juniper;
# use juniper::FieldResult;
struct User { id: String }

graphql_object!(User: () |&self| {
    field id() -> FieldResult<&String> {
        Ok(&self.id)
    }

    field name() -> FieldResult<&String> {
        Err("Does not have a name".to_owned())?
    }
});

# fn main() { }
```

# Syntax

The top-most syntax of this macro defines which type to expose, the context
type, which lifetime parameters or generics to define, and which name to use in
the GraphQL schema. It takes one of the following two forms:

```text
ExposedType: ContextType as "ExposedName" |&self| { items... }
<Generics> ExposedType: ContextType as "ExposedName" |&self| { items... }
```

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

[1]: struct.Executor.html

*/
#[macro_export(local_inner_macros)]
macro_rules! graphql_object {
    ( @as_item, $i:item) => { $i };
    ( @as_expr, $e:expr) => { $e };

    // field deprecated <reason> <name>(...) -> <type> as <description> { ... }
    (
        @gather_object_meta,
        $reg:expr, $acc:expr, $info:expr, $descr:expr, $ifaces:expr,
        field deprecated
            $reason:tt
            $name:ident
            $args:tt -> $t:ty
            as $desc:tt
            $body:block
            $( $rest:tt )*
    ) => {
        $acc.push(__graphql__args!(
            @apply_args,
            $reg,
            $reg.field_convert::<$t, _, Self::Context>(
                &$crate::to_camel_case(__graphql__stringify!($name)), $info)
                .description($desc)
                .deprecated($reason),
            $info,
            $args));

        graphql_object!(@gather_object_meta, $reg, $acc, $info, $descr, $ifaces, $( $rest )*);
    };

    // field deprecated <reason> <name>(...) -> <type> { ... }
    (
        @gather_object_meta,
        $reg:expr, $acc:expr, $info:expr, $descr:expr, $ifaces:expr,
        field deprecated $reason:tt $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        $acc.push(__graphql__args!(
            @apply_args,
            $reg,
            $reg.field_convert::<$t, _, Self::Context>(
                &$crate::to_camel_case(__graphql__stringify!($name)), $info)
                .deprecated($reason),
            $info,
            $args));

        graphql_object!(@gather_object_meta, $reg, $acc, $info, $descr, $ifaces, $( $rest )*);
    };

    // field <name>(...) -> <type> as <description> { ... }
    (
        @gather_object_meta,
        $reg:expr, $acc:expr, $info:expr, $descr:expr, $ifaces:expr,
        field $name:ident $args:tt -> $t:ty as $desc:tt $body:block $( $rest:tt )*
    ) => {
        $acc.push(__graphql__args!(
            @apply_args,
            $reg,
            $reg.field_convert::<$t, _, Self::Context>(
                &$crate::to_camel_case(__graphql__stringify!($name)), $info)
                .description($desc),
            $info,
            $args));

        graphql_object!(@gather_object_meta, $reg, $acc, $info, $descr, $ifaces, $( $rest )*);
    };

    // field <name>(...) -> <type> { ... }
    (
        @gather_object_meta,
        $reg:expr, $acc:expr, $info:expr, $descr:expr, $ifaces:expr,
        field $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        $acc.push(__graphql__args!(
            @apply_args,
            $reg,
            $reg.field_convert::<$t, _, Self::Context>(
                &$crate::to_camel_case(__graphql__stringify!($name)), $info),
            $info,
            $args));

        graphql_object!(@gather_object_meta, $reg, $acc, $info, $descr, $ifaces, $( $rest )*);
    };

    // description: <description>
    (
        @gather_object_meta,
        $reg:expr, $acc:expr, $info:expr, $descr:expr, $ifaces:expr,
        description : $value:tt $( $rest:tt )*
    ) => {
        $descr = Some(graphql_object!(@as_expr, $value));

        graphql_object!(@gather_object_meta, $reg, $acc, $info, $descr, $ifaces, $( $rest )*)
    };

    // interfaces: [...]
    (
        @gather_object_meta,
        $reg:expr, $acc:expr, $info:expr, $descr:expr, $ifaces:expr,
        interfaces : $value:tt $( $rest:tt )*
    ) => {
        graphql_object!(@assign_interfaces, $reg, $ifaces, $value);

        graphql_object!(@gather_object_meta, $reg, $acc, $info, $descr, $ifaces, $( $rest )*)
    };

    // eat commas
    (
        @gather_object_meta,
        $reg:expr, $acc:expr, $info:expr, $descr:expr, $ifaces:expr, , $( $rest:tt )*
    ) => {
        graphql_object!(@gather_object_meta, $reg, $acc, $info, $descr, $ifaces, $( $rest )*)
    };

    // base case
    (
        @gather_object_meta,
        $reg:expr, $acc:expr, $info:expr, $descr:expr, $ifaces:expr,
    ) => {};

    ( @assign_interfaces, $reg:expr, $tgt:expr, [ $($t:ty,)* ] ) => {
        $tgt = Some(__graphql__vec![
            $($reg.get_type::<$t>(&())),*
        ]);
    };

    ( @assign_interfaces, $reg:expr, $tgt:expr, [ $($t:ty),* ] ) => {
        $tgt = Some(__graphql__vec![
            $($reg.get_type::<$t>(&())),*
        ]);
    };

    (
        ( $($lifetime:tt)* );
        $name:ty; $ctxt:ty; $outname:expr; $mainself:ident; $($items:tt)*
    ) => {
        graphql_object!(@as_item, impl<$($lifetime)*> $crate::GraphQLType for $name {
            type Context = $ctxt;
            type TypeInfo = ();

            fn name(_: &()) -> Option<&str> {
                Some($outname)
            }

            #[allow(unused_assignments)]
            #[allow(unused_mut)]
            fn meta<'r>(
                info: &(),
                registry: &mut $crate::Registry<'r>
            ) -> $crate::meta::MetaType<'r> {
                let mut fields = Vec::new();
                let mut description = None;
                let mut interfaces: Option<Vec<$crate::Type>> = None;
                graphql_object!(
                    @gather_object_meta,
                    registry, fields, info, description, interfaces, $($items)*
                );
                let mut mt = registry.build_object_type::<$name>(info, &fields);

                if let Some(description) = description {
                    mt = mt.description(description);
                }

                if let Some(interfaces) = interfaces {
                    mt = mt.interfaces(&interfaces);
                }

                mt.into_meta()
            }

            fn concrete_type_name(&self, _: &Self::Context, _: &()) -> String {
                $outname.to_owned()
            }

            #[allow(unused_variables)]
            #[allow(unused_mut)]
            fn resolve_field(
                &$mainself,
                info: &(),
                field: &str,
                args: &$crate::Arguments,
                executor: &$crate::Executor<Self::Context>
            )
                -> $crate::ExecutionResult
            {
                __graphql__build_field_matches!(
                    ($outname, $mainself, field, args, executor),
                    (),
                    $($items)*);
            }
        });
    };

    (
        <$( $lifetime:tt ),*> $name:ty : $ctxt:ty as $outname:tt | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_object!(
            ( $($lifetime),* ); $name; $ctxt; $outname; $mainself; $( $items )*);
    };

    (
        $name:ty : $ctxt:ty as $outname:tt | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_object!(
            ( ); $name; $ctxt; $outname; $mainself; $( $items )*);
    };

    (
        $name:ty : $ctxt:ty | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_object!(
            ( ); $name; $ctxt; (__graphql__stringify!($name)); $mainself; $( $items )*);
    };
}
