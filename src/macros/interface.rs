/**
Expose GraphQL interfaces

Mapping interfaces to GraphQL can be tricky: there is no direct counterpart to
GraphQL interfaces in Rust, and downcasting is not possible in the general case.
Many other GraphQL implementations in other languages use instance checks and
either dynamic typing or forced downcasts to support these features.

A GraphQL interface defines fields that the implementing types also need to
implement. A GraphQL interface also needs to be able to determine the concrete
type name as well as downcast the general type to the actual concrete type.

## Syntax

See the documentation for [`graphql_object!`][1] on the general item and type
syntax. `graphql_interface!` requires an additional `instance_resolvers` item,
and does _not_ support the `interfaces` item.

`instance_resolvers` is a list/lambda hybrid used to resolve the concrete
instance type of the interface. It starts with a context argument and continues
with an array of expressions, each resolving into an `Option<T>` of the possible
instances:

```rust,ignore
instance_resolvers: |&context| [
    context.get_human(self.id()), // returns Option<Human>
    context.get_droid(self.id()), // returns Option<Droid>
],
```

Each item in the array will be executed in order when the concrete type is
required.

## Example

A simplified extract from the StarWars schema example shows how to use the
shared context to implement downcasts.

```rust
# #[macro_use] extern crate juniper;
# use std::collections::HashMap;
struct Human { id: String }
struct Droid { id: String }
struct Database {
    humans: HashMap<String, Human>,
    droids: HashMap<String, Droid>,
}

trait Character {
    fn id(&self) -> &str;
}

impl Character for Human {
    fn id(&self) -> &str { &self.id }
}

impl Character for Droid {
    fn id(&self) -> &str { &self.id }
}

graphql_object!(Human: Database as "Human" |&self| {
    field id() -> &str { &self.id }
});

graphql_object!(Droid: Database as "Droid" |&self| {
    field id() -> &str { &self.id }
});

// You can introduce lifetimes or generic parameters by < > before the name.
graphql_interface!(<'a> &'a Character: Database as "Character" |&self| {
    field id() -> &str { self.id() }

    instance_resolvers: |&context| [
        context.humans.get(self.id()),
        context.droids.get(self.id()),
    ]
});

# fn main() { }
```

[1]: macro.graphql_object!.html

*/
#[macro_export]
macro_rules! graphql_interface {
    ( @as_item, $i:item) => { $i };
    ( @as_expr, $e:expr) => { $e };

    // field deprecated <reason> <name>(...) -> <type> as <description> { ... }
    (
        @gather_meta,
        $reg:expr, $acc:expr, $descr:expr,
        field deprecated $reason:tt $name:ident $args:tt -> $t:ty as $desc:tt $body:block $( $rest:tt )*
    ) => {
        $acc.push(__graphql__args!(
            @apply_args,
            $reg,
            $reg.field_convert::<$t, _>(
                &$crate::to_snake_case(stringify!($name)))
                .description($desc)
                .deprecated($reason),
            $args));

        graphql_interface!(@gather_meta, $reg, $acc, $descr, $( $rest )*);
    };

    // field deprecated <reason> <name>(...) -> <type> { ... }
    (
        @gather_meta,
        $reg:expr, $acc:expr, $descr:expr,
        field deprecated $reason:tt $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        $acc.push(__graphql__args!(
            @apply_args,
            $reg,
            $reg.field_convert::<$t, _>(
                &$crate::to_snake_case(stringify!($name)))
                .deprecated($reason),
            $args));

        graphql_interface!(@gather_meta, $reg, $acc, $descr, $( $rest )*);
    };

    // field <name>(...) -> <type> as <description> { ... }
    (
        @gather_meta,
        $reg:expr, $acc:expr, $descr:expr,
        field $name:ident $args:tt -> $t:ty as $desc:tt $body:block $( $rest:tt )*
    ) => {
        $acc.push(__graphql__args!(
            @apply_args,
            $reg,
            $reg.field_convert::<$t, _>(
                &$crate::to_snake_case(stringify!($name)))
                .description($desc),
            $args));

        graphql_interface!(@gather_meta, $reg, $acc, $descr, $( $rest )*);
    };

    // field <name>(...) -> <type> { ... }
    (
        @gather_meta,
        $reg:expr, $acc:expr, $descr:expr,
        field $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        $acc.push(__graphql__args!(
            @apply_args,
            $reg,
            $reg.field_convert::<$t, _>(
                &$crate::to_snake_case(stringify!($name))),
            $args));

        graphql_interface!(@gather_meta, $reg, $acc, $descr, $( $rest )*);
    };

    // description: <description>
    (
        @gather_meta,
        $reg:expr, $acc:expr, $descr:expr,
        description : $value:tt $( $rest:tt )*
    ) => {
        $descr = Some(graphql_interface!(@as_expr, $value));

        graphql_interface!(@gather_meta, $reg, $acc, $descr, $( $rest )*)
    };

    // instance_resolvers: | <ctxtvar> | [...]
    (
        @gather_meta,
        $reg:expr, $acc:expr, $descr:expr,
        instance_resolvers: | $ctxtvar:pat | $resolvers:tt $( $rest:tt )*
    ) => {
        graphql_interface!(@gather_meta, $reg, $acc, $descr, $( $rest )*)
    };

    ( @gather_meta, $reg:expr, $acc:expr, $descr:expr, , $( $rest:tt )* ) => {
        graphql_interface!(@gather_meta, $reg, $acc, $descr, $( $rest )*)
    };

    ( @gather_meta, $reg:expr, $acc:expr, $descr:expr, ) => {
    };

    // field deprecated <reason> <name>(...) -> <type> as <description> { ... }
    (
        @resolve_into_type,
        $buildargs:tt,
        field deprecated $reason:tt $name:ident $args:tt -> $t:ty as $descr:tt $body:block $( $rest:tt )*
    ) => {
        graphql_interface!(@resolve_into_type, $buildargs, $( $rest )*)
    };

    // field deprecated <reason> <name>(...) -> <type> { ... }
    (
        @resolve_into_type,
        $buildargs:tt,
        field deprecated $reason:tt $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        graphql_interface!(@resolve_into_type, $buildargs, $( $rest )*)
    };

    // field <name>(...) -> <type> as <description> { ... }
    (
        @resolve_into_type,
        $buildargs:tt,
        field $name:ident $args:tt -> $t:ty as $descr:tt $body:block $( $rest:tt )*
    ) => {
        graphql_interface!(@resolve_into_type, $buildargs, $( $rest )*)
    };

    // field <name>(...) -> <type> { ... }
    (
        @resolve_into_type,
        $buildargs:tt,
        field $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        graphql_interface!(@resolve_into_type, $buildargs, $( $rest )*)
    };

    // description: <description>
    (
        @resolve_into_type,
        $buildargs:tt, description : $value:tt $( $rest:tt )*
    ) => {
        graphql_interface!(@resolve_into_type, $buildargs, $( $rest )*)
    };

    // field deprecated <reason> <name>(...) -> <type> as <description> { ... }
    (
        @concrete_type_name,
        $buildargs:tt,
        field deprecated $reason:tt $name:ident $args:tt -> $t:ty as $descr:tt $body:block $( $rest:tt )*
    ) => {
        graphql_interface!(@concrete_type_name, $buildargs, $( $rest )*)
    };

    // field deprecated <reason> <name>(...) -> <type> { ... }
    (
        @concrete_type_name,
        $buildargs:tt,
        field deprecated $reason:tt $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        graphql_interface!(@concrete_type_name, $buildargs, $( $rest )*)
    };

    // field <name>(...) -> <type> as <description> { ... }
    (
        @concrete_type_name,
        $buildargs:tt,
        field $name:ident $args:tt -> $t:ty as $descr:tt $body:block $( $rest:tt )*
    ) => {
        graphql_interface!(@concrete_type_name, $buildargs, $( $rest )*)
    };

    // field <name>(...) -> <type> { ... }
    (
        @concrete_type_name,
        $buildargs:tt,
        field $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        graphql_interface!(@concrete_type_name, $buildargs, $( $rest )*)
    };

    // description: <description>
    (
        @concrete_type_name,
        $buildargs:tt, description : $value:tt $( $rest:tt )*
    ) => {
        graphql_interface!(@concrete_type_name, $buildargs, $( $rest )*)
    };

    // instance_resolvers: | <ctxtvar> | [...]
    (
        @concrete_type_name,
        ($outname:tt, $ctxtarg:ident, $ctxttype:ty),
        instance_resolvers : | $ctxtvar:pat | [ $( $resolver:expr ),* $(,)* ] $( $rest:tt )*
    ) => {
        let $ctxtvar = &$ctxtarg;

        fn inner_type_of<T>(_: T) -> String where T: $crate::GraphQLType<$ctxttype> {
            T::name().unwrap().to_owned()
        }

        $(
            if let Some(ref v) = $resolver {
                return inner_type_of(v);
            }
        )*

        panic!("Concrete type not handled by instance resolvers on {}", $outname);
    };

    ( @concrete_type_name, $buildargs:tt, ) => {
        ()
    };

    // instance_resolvers: | <ctxtvar> |
    (
        @resolve_into_type,
        ($outname:tt, $typenamearg:ident, $execarg:ident, $ctxttype:ty),
        instance_resolvers : | $ctxtvar:pat | [ $( $resolver:expr ),* $(,)* ] $( $rest:tt )*
    ) => {
        let $ctxtvar = &$execarg.context();

        fn inner_type_of<T>(_: T) -> String where T: $crate::GraphQLType<$ctxttype> {
            T::name().unwrap().to_owned()
        }

        $(
            if let Some(ref v) = $resolver {
                if inner_type_of(v) == $typenamearg {
                    return $execarg.resolve(v);
                }
            }
        )*

        return Ok($crate::Value::null());
    };

    ( @resolve_into_type, $buildargs:tt, ) => {
        ()
    };

    (
        ( $($lifetime:tt),* ) $name:ty : $ctxt:ty as $outname:tt | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_interface!(@as_item, impl<$($lifetime)*> $crate::GraphQLType<$ctxt> for $name {
            fn name() -> Option<&'static str> {
                Some($outname)
            }

            #[allow(unused_assignments)]
            #[allow(unused_mut)]
            fn meta(registry: &mut $crate::Registry<$ctxt>) -> $crate::meta::MetaType {
                let mut fields = Vec::new();
                let mut description = None;
                graphql_interface!(@gather_meta, registry, fields, description, $($items)*);
                let mut mt = registry.build_interface_type::<$name>()(&fields);

                if let Some(description) = description {
                    mt = mt.description(description);
                }

                mt.into_meta()
            }

            #[allow(unused_variables)]
            #[allow(unused_mut)]
            fn resolve_field(&$mainself, field: &str, args: &$crate::Arguments, mut executor: &mut $crate::Executor<$ctxt>) -> $crate::ExecutionResult {
                __graphql__build_field_matches!(($outname, $mainself, field, args, executor), (), $($items)*);
            }

            fn concrete_type_name(&$mainself, context: &$ctxt) -> String {
                graphql_interface!(
                    @concrete_type_name,
                    ($outname, context, $ctxt),
                    $($items)*);
            }

            fn resolve_into_type(&$mainself, type_name: &str, _: Option<Vec<$crate::Selection>>, executor: &mut $crate::Executor<$ctxt>) -> $crate::ExecutionResult {
                graphql_interface!(
                    @resolve_into_type,
                    ($outname, type_name, executor, $ctxt),
                    $($items)*);
            }
        });

        impl<$($lifetime)*> $crate::IntoFieldResult<$name> for $name {
            fn into(self) -> $crate::FieldResult<$name> {
                Ok(self)
            }
        }
    };

    (
        <$($lifetime:tt),*> $name:ty : $ctxt:ty as $outname:tt | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_interface!(
            ($($lifetime),*) $name : $ctxt as $outname | &$mainself | { $( $items )* });
    };

    (
        $name:ty : $ctxt:ty as $outname:tt | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_interface!(() $name : $ctxt as $outname | &$mainself | { $( $items )* });
    };

    (
        $name:ty : $ctxt:ty | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_interface!(() $name : $ctxt as (stringify!($name)) | &$mainself | { $( $items )* });
    };
}
