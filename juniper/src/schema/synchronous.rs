//! Detection of the output types that resolve without awaiting.

use std::{
    collections::{HashMap, HashSet},
    iter,
};

use arcstr::ArcStr;
use auto_enums::auto_enum;
use fnv::FnvHashMap;

use crate::{schema::meta::MetaType, types::name::Name};

/// Sets the `synchronous` flags of the interface fields and the composite types in `types`.
///
/// An interface field is synchronous if the field is synchronous in every object that implements
/// the interface. An object resolves synchronously if all its fields are synchronous and all their
/// types resolve synchronously. An interface or a union resolves synchronously if all its possible
/// objects do.
pub(super) fn mark_synchronous_types<S>(types: &mut FnvHashMap<Name, MetaType<S>>) {
    let implementers = implementers(types);
    let synchronous_types = synchronous_types(types, &implementers);
    let synchronous_interface_fields = synchronous_interface_fields(types, &implementers);

    for meta in types.values_mut() {
        match meta {
            MetaType::Object(object) => {
                object.synchronous = synchronous_types.contains(&object.name);
            }
            MetaType::Union(union) => union.synchronous = synchronous_types.contains(&union.name),
            MetaType::Interface(interface) => {
                interface.synchronous = synchronous_types.contains(&interface.name);
                for field in interface.fields.iter_mut().filter(|f| !f.is_builtin()) {
                    field.synchronous = synchronous_interface_fields
                        .contains(&(interface.name.clone(), field.name.clone()));
                }
            }
            MetaType::Enum(..)
            | MetaType::InputObject(..)
            | MetaType::List(..)
            | MetaType::Nullable(..)
            | MetaType::Placeholder(..)
            | MetaType::Scalar(..) => {}
        }
    }
}

/// Returns the names of the objects that implement each interface in `types`.
fn implementers<S>(types: &FnvHashMap<Name, MetaType<S>>) -> HashMap<ArcStr, Vec<ArcStr>> {
    let mut implementers = HashMap::<_, Vec<_>>::new();
    for meta in types.values() {
        if let MetaType::Object(object) = meta {
            for interface in &object.interface_names {
                implementers
                    .entry(interface.clone())
                    .or_default()
                    .push(object.name.clone());
            }
        }
    }
    implementers
}

/// Returns the interface name and the field name of each synchronous interface field in `types`.
fn synchronous_interface_fields<S>(
    types: &FnvHashMap<Name, MetaType<S>>,
    implementers: &HashMap<ArcStr, Vec<ArcStr>>,
) -> HashSet<(ArcStr, ArcStr)> {
    let mut fields = HashSet::new();
    for meta in types.values() {
        let MetaType::Interface(interface) = meta else {
            continue;
        };
        let objects = implementers
            .get(&interface.name)
            .map_or(&[][..], Vec::as_slice);
        for field in interface.fields.iter().filter(|f| !f.is_builtin()) {
            let synchronous = objects.iter().all(|object| {
                types
                    .get(object.as_str())
                    .and_then(|object| object.field_by_name(&field.name))
                    .is_some_and(|field| field.synchronous)
            });
            if synchronous {
                fields.insert((interface.name.clone(), field.name.clone()));
            }
        }
    }
    fields
}

/// Returns the names of the composite types in `types` that resolve synchronously.
///
/// Types can depend on each other in cycles. So every candidate starts as synchronous. Each pass
/// removes the candidates that depend on a type outside the candidates. The passes stop when no
/// candidate is removed.
fn synchronous_types<S>(
    types: &FnvHashMap<Name, MetaType<S>>,
    implementers: &HashMap<ArcStr, Vec<ArcStr>>,
) -> HashSet<ArcStr> {
    let mut candidates = types
        .values()
        .filter_map(|meta| match meta {
            MetaType::Object(object) => object
                .fields
                .iter()
                .all(|f| f.synchronous)
                .then(|| object.name.clone()),
            MetaType::Interface(interface) => Some(interface.name.clone()),
            MetaType::Union(union) => Some(union.name.clone()),
            MetaType::Enum(..)
            | MetaType::InputObject(..)
            | MetaType::List(..)
            | MetaType::Nullable(..)
            | MetaType::Placeholder(..)
            | MetaType::Scalar(..) => None,
        })
        .collect::<HashSet<_>>();

    loop {
        let asynchronous = candidates
            .iter()
            .filter(|name| {
                dependencies(types, implementers, name)
                    .any(|dependency| !resolves_synchronously(types, &candidates, dependency))
            })
            .cloned()
            .collect::<Vec<_>>();
        if asynchronous.is_empty() {
            return candidates;
        }
        for name in &asynchronous {
            candidates.remove(name);
        }
    }
}

/// Returns the names of the types that the composite type with the `name` resolves through.
///
/// An object depends on the types of its fields. An interface depends on its implementers, and a
/// union depends on its possible types.
#[auto_enum(Iterator)]
fn dependencies<'t, S>(
    types: &'t FnvHashMap<Name, MetaType<S>>,
    implementers: &'t HashMap<ArcStr, Vec<ArcStr>>,
    name: &str,
) -> impl Iterator<Item = &'t str> {
    match types.get(name) {
        Some(MetaType::Object(object)) => {
            object.fields.iter().map(|f| f.field_type.innermost_name())
        }
        Some(MetaType::Interface(interface)) => implementers
            .get(&interface.name)
            .into_iter()
            .flatten()
            .map(ArcStr::as_str),
        Some(MetaType::Union(union)) => union.of_type_names.iter().map(ArcStr::as_str),
        Some(
            MetaType::Enum(..)
            | MetaType::InputObject(..)
            | MetaType::List(..)
            | MetaType::Nullable(..)
            | MetaType::Placeholder(..)
            | MetaType::Scalar(..),
        )
        | None => iter::empty(),
    }
}

/// Indicates whether the type with the `name` resolves synchronously, if the `candidates` are the
/// synchronous composite types.
fn resolves_synchronously<S>(
    types: &FnvHashMap<Name, MetaType<S>>,
    candidates: &HashSet<ArcStr>,
    name: &str,
) -> bool {
    match types.get(name) {
        Some(MetaType::Enum(..) | MetaType::Scalar(..)) => true,
        Some(MetaType::Interface(..) | MetaType::Object(..) | MetaType::Union(..)) => {
            candidates.contains(name)
        }
        Some(
            MetaType::InputObject(..)
            | MetaType::List(..)
            | MetaType::Nullable(..)
            | MetaType::Placeholder(..),
        )
        | None => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        EmptyMutation, EmptySubscription, GraphQLUnion, RootNode, graphql_interface,
        graphql_object, schema::meta::MetaType,
    };

    struct Leaf;

    #[graphql_object]
    impl Leaf {
        fn value() -> i32 {
            1
        }
    }

    struct AsyncLeaf;

    #[graphql_object]
    impl AsyncLeaf {
        async fn value() -> i32 {
            1
        }
    }

    struct Parent;

    #[graphql_object]
    impl Parent {
        fn leaf() -> AsyncLeaf {
            AsyncLeaf
        }
    }

    struct Node;

    #[graphql_object]
    impl Node {
        fn next() -> Option<Node> {
            None
        }
    }

    struct First;

    #[graphql_object]
    impl First {
        fn second() -> Option<Second> {
            None
        }
    }

    struct Second;

    #[graphql_object]
    impl Second {
        fn first() -> Option<First> {
            None
        }

        async fn value() -> i32 {
            1
        }
    }

    #[graphql_interface(for = [SyncNamed, AsyncNamed])]
    #[expect(dead_code, reason = "GraphQL schema definition")]
    trait Named {
        fn name(&self) -> &str;
    }

    #[graphql_interface(for = SyncNamed)]
    #[expect(dead_code, reason = "GraphQL schema definition")]
    trait Labeled {
        fn label(&self) -> &str;
    }

    struct SyncNamed;

    #[graphql_object(impl = [NamedValue, LabeledValue])]
    impl SyncNamed {
        fn name(&self) -> &str {
            "sync"
        }

        fn label(&self) -> &str {
            "sync"
        }
    }

    struct AsyncNamed;

    #[graphql_object(impl = NamedValue)]
    impl AsyncNamed {
        async fn name(&self) -> &str {
            "async"
        }
    }

    #[derive(GraphQLUnion)]
    #[expect(dead_code, reason = "GraphQL schema definition")]
    enum SyncUnion {
        Leaf(Leaf),
        Node(Node),
    }

    #[derive(GraphQLUnion)]
    #[expect(dead_code, reason = "GraphQL schema definition")]
    enum MixedUnion {
        Leaf(Leaf),
        AsyncLeaf(AsyncLeaf),
    }

    struct Query;

    #[graphql_object]
    impl Query {
        fn leaf() -> Leaf {
            Leaf
        }

        fn parent() -> Parent {
            Parent
        }

        fn node() -> Node {
            Node
        }

        fn first() -> First {
            First
        }

        fn named() -> Option<NamedValue> {
            None
        }

        fn labeled() -> Option<LabeledValue> {
            None
        }

        fn sync_union() -> Option<SyncUnion> {
            None
        }

        fn mixed_union() -> Option<MixedUnion> {
            None
        }
    }

    type Schema = RootNode<Query, EmptyMutation, EmptySubscription>;

    fn is_synchronous_type(schema: &Schema, type_name: &str) -> bool {
        schema
            .schema
            .concrete_type_by_name(type_name)
            .unwrap_or_else(|| panic!("no type `{type_name}`"))
            .is_synchronous()
    }

    fn is_synchronous_interface_field(schema: &Schema, type_name: &str, field_name: &str) -> bool {
        let MetaType::Interface(interface) =
            schema.schema.concrete_type_by_name(type_name).unwrap()
        else {
            panic!("`{type_name}` is not an interface");
        };
        interface
            .fields
            .iter()
            .find(|f| f.name == field_name)
            .unwrap_or_else(|| panic!("no field `{type_name}.{field_name}`"))
            .synchronous
    }

    mod object {
        use super::{Query, Schema, is_synchronous_type};
        use crate::{EmptyMutation, EmptySubscription};

        #[test]
        fn with_synchronous_fields_is_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert!(is_synchronous_type(&schema, "Leaf"));
        }

        #[test]
        fn with_async_field_is_not_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert!(!is_synchronous_type(&schema, "AsyncLeaf"));
        }

        #[test]
        fn with_field_of_async_type_is_not_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert!(!is_synchronous_type(&schema, "Parent"));
        }

        #[test]
        fn referring_to_itself_is_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert!(is_synchronous_type(&schema, "Node"));
        }

        #[test]
        fn in_cycle_with_async_field_is_not_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert!(!is_synchronous_type(&schema, "First"));
            assert!(!is_synchronous_type(&schema, "Second"));
        }

        #[test]
        fn introspection_types_are_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            for name in [
                "__Schema",
                "__Type",
                "__Field",
                "__InputValue",
                "__EnumValue",
            ] {
                assert!(
                    is_synchronous_type(&schema, name),
                    "`{name}` is not synchronous"
                );
            }
        }
    }

    mod interface {
        use super::{Query, Schema, is_synchronous_interface_field, is_synchronous_type};
        use crate::{EmptyMutation, EmptySubscription};

        #[test]
        fn with_synchronous_implementers_is_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert!(is_synchronous_type(&schema, "Labeled"));
            assert!(is_synchronous_interface_field(&schema, "Labeled", "label"));
        }

        #[test]
        fn with_async_implementer_is_not_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert!(!is_synchronous_type(&schema, "Named"));
            assert!(!is_synchronous_interface_field(&schema, "Named", "name"));
        }
    }

    mod union {
        use super::{Query, Schema, is_synchronous_type};
        use crate::{EmptyMutation, EmptySubscription};

        #[test]
        fn with_synchronous_members_is_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert!(is_synchronous_type(&schema, "SyncUnion"));
        }

        #[test]
        fn with_async_member_is_not_synchronous() {
            let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert!(!is_synchronous_type(&schema, "MixedUnion"));
        }
    }
}
