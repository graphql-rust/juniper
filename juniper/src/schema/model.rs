use std::fmt;

use fnv::FnvHashMap;

use juniper_codegen::GraphQLEnumInternal as GraphQLEnum;

use crate::{
    ast::Type,
    executor::{Context, Registry},
    schema::meta::{Argument, InterfaceMeta, MetaType, ObjectMeta, PlaceholderMeta, UnionMeta},
    types::{base::GraphQLType, name::Name},
    value::{DefaultScalarValue, ScalarValue},
};

/// Root query node of a schema
///
/// This brings the mutation and query types together, and provides the
/// predefined metadata fields.
#[derive(Debug)]
pub struct RootNode<'a, QueryT: GraphQLType<S>, MutationT: GraphQLType<S>, S = DefaultScalarValue>
where
    S: ScalarValue,
{
    #[doc(hidden)]
    pub query_type: QueryT,
    #[doc(hidden)]
    pub query_info: QueryT::TypeInfo,
    #[doc(hidden)]
    pub mutation_type: MutationT,
    #[doc(hidden)]
    pub mutation_info: MutationT::TypeInfo,
    #[doc(hidden)]
    pub schema: SchemaType<'a, S>,
}

/// Metadata for a schema
#[derive(Debug)]
pub struct SchemaType<'a, S> {
    pub(crate) types: FnvHashMap<Name, MetaType<'a, S>>,
    query_type_name: String,
    mutation_type_name: Option<String>,
    directives: FnvHashMap<String, DirectiveType<'a, S>>,
}

impl<'a, S> Context for SchemaType<'a, S> {}

#[derive(Clone)]
pub enum TypeType<'a, S: 'a> {
    Concrete(&'a MetaType<'a, S>),
    NonNull(Box<TypeType<'a, S>>),
    List(Box<TypeType<'a, S>>),
}

#[derive(Debug)]
pub struct DirectiveType<'a, S> {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<DirectiveLocation>,
    pub arguments: Vec<Argument<'a, S>>,
}

#[derive(Clone, PartialEq, Eq, Debug, GraphQLEnum)]
#[graphql(name = "__DirectiveLocation")]
pub enum DirectiveLocation {
    Query,
    Mutation,
    Subscription,
    Field,
    #[graphql(name = "FRAGMENT_DEFINITION")]
    FragmentDefinition,
    #[graphql(name = "FRAGMENT_SPREAD")]
    FragmentSpread,
    #[graphql(name = "INLINE_FRAGMENT")]
    InlineFragment,
}

impl<'a, QueryT, MutationT, S> RootNode<'a, QueryT, MutationT, S>
where
    S: ScalarValue + 'a,
    QueryT: GraphQLType<S, TypeInfo = ()>,
    MutationT: GraphQLType<S, TypeInfo = ()>,
{
    /// Construct a new root node from query and mutation nodes
    ///
    /// If the schema should not support mutations, use the
    /// `new` constructor instead.
    pub fn new(query_obj: QueryT, mutation_obj: MutationT) -> Self {
        RootNode::new_with_info(query_obj, mutation_obj, (), ())
    }
}

impl<'a, S, QueryT, MutationT> RootNode<'a, QueryT, MutationT, S>
where
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    S: ScalarValue + 'a,
{
    /// Construct a new root node from query and mutation nodes,
    /// while also providing type info objects for the query and
    /// mutation types.
    pub fn new_with_info(
        query_obj: QueryT,
        mutation_obj: MutationT,
        query_info: QueryT::TypeInfo,
        mutation_info: MutationT::TypeInfo,
    ) -> Self {
        RootNode {
            query_type: query_obj,
            mutation_type: mutation_obj,
            schema: SchemaType::new::<QueryT, MutationT>(&query_info, &mutation_info),
            query_info,
            mutation_info,
        }
    }
}

impl<'a, S> SchemaType<'a, S> {
    pub fn new<QueryT, MutationT>(
        query_info: &QueryT::TypeInfo,
        mutation_info: &MutationT::TypeInfo,
    ) -> Self
    where
        S: ScalarValue + 'a,
        QueryT: GraphQLType<S>,
        MutationT: GraphQLType<S>,
    {
        let mut directives = FnvHashMap::default();
        let query_type_name: String;
        let mutation_type_name: String;

        let mut registry = Registry::new(FnvHashMap::default());
        query_type_name = registry
            .get_type::<QueryT>(query_info)
            .innermost_name()
            .to_owned();

        mutation_type_name = registry
            .get_type::<MutationT>(mutation_info)
            .innermost_name()
            .to_owned();

        registry.get_type::<SchemaType<S>>(&());

        directives.insert("skip".to_owned(), DirectiveType::new_skip(&mut registry));
        directives.insert(
            "include".to_owned(),
            DirectiveType::new_include(&mut registry),
        );

        let mut meta_fields = vec![
            registry.field::<SchemaType<S>>("__schema", &()),
            registry
                .field::<TypeType<S>>("__type", &())
                .argument(registry.arg::<String>("name", &())),
        ];

        if let Some(root_type) = registry.types.get_mut(&query_type_name) {
            if let MetaType::Object(ObjectMeta { ref mut fields, .. }) = *root_type {
                fields.append(&mut meta_fields);
            } else {
                panic!("Root type is not an object");
            }
        } else {
            panic!("Root type not found");
        }

        for meta_type in registry.types.values() {
            if let MetaType::Placeholder(PlaceholderMeta { ref of_type }) = *meta_type {
                panic!("Type {:?} is still a placeholder type", of_type);
            }
        }
        SchemaType {
            types: registry.types,
            query_type_name,
            mutation_type_name: if &mutation_type_name != "_EmptyMutation" {
                Some(mutation_type_name)
            } else {
                None
            },
            directives,
        }
    }

    pub fn add_directive(&mut self, directive: DirectiveType<'a, S>) {
        self.directives.insert(directive.name.clone(), directive);
    }

    pub fn type_by_name(&self, name: &str) -> Option<TypeType<S>> {
        self.types.get(name).map(|t| TypeType::Concrete(t))
    }

    pub fn concrete_type_by_name(&self, name: &str) -> Option<&MetaType<S>> {
        self.types.get(name)
    }

    pub(crate) fn lookup_type(&self, tpe: &Type) -> Option<&MetaType<S>> {
        match *tpe {
            Type::NonNullNamed(ref name) | Type::Named(ref name) => {
                self.concrete_type_by_name(name)
            }
            Type::List(ref inner) | Type::NonNullList(ref inner) => self.lookup_type(inner),
        }
    }

    pub fn query_type(&self) -> TypeType<S> {
        TypeType::Concrete(
            self.types
                .get(&self.query_type_name)
                .expect("Query type does not exist in schema"),
        )
    }

    pub fn concrete_query_type(&self) -> &MetaType<S> {
        self.types
            .get(&self.query_type_name)
            .expect("Query type does not exist in schema")
    }

    pub fn mutation_type(&self) -> Option<TypeType<S>> {
        if let Some(ref mutation_type_name) = self.mutation_type_name {
            Some(
                self.type_by_name(mutation_type_name)
                    .expect("Mutation type does not exist in schema"),
            )
        } else {
            None
        }
    }

    pub fn concrete_mutation_type(&self) -> Option<&MetaType<S>> {
        self.mutation_type_name.as_ref().map(|name| {
            self.concrete_type_by_name(name)
                .expect("Mutation type does not exist in schema")
        })
    }

    pub fn subscription_type(&self) -> Option<TypeType<S>> {
        // subscription is not yet in `RootNode`,
        // so return `None` for now
        None
    }

    pub fn concrete_subscription_type(&self) -> Option<&MetaType<S>> {
        // subscription is not yet in `RootNode`,
        // so return `None` for now
        None
    }

    pub fn type_list(&self) -> Vec<TypeType<S>> {
        self.types.values().map(|t| TypeType::Concrete(t)).collect()
    }

    pub fn concrete_type_list(&self) -> Vec<&MetaType<S>> {
        self.types.values().collect()
    }

    pub fn make_type(&self, t: &Type) -> TypeType<S> {
        match *t {
            Type::NonNullNamed(ref n) => TypeType::NonNull(Box::new(
                self.type_by_name(n).expect("Type not found in schema"),
            )),
            Type::NonNullList(ref inner) => {
                TypeType::NonNull(Box::new(TypeType::List(Box::new(self.make_type(inner)))))
            }
            Type::Named(ref n) => self.type_by_name(n).expect("Type not found in schema"),
            Type::List(ref inner) => TypeType::List(Box::new(self.make_type(inner))),
        }
    }

    pub fn directive_list(&self) -> Vec<&DirectiveType<S>> {
        self.directives.values().collect()
    }

    pub fn directive_by_name(&self, name: &str) -> Option<&DirectiveType<S>> {
        self.directives.get(name)
    }

    pub fn type_overlap(&self, t1: &MetaType<S>, t2: &MetaType<S>) -> bool {
        if (t1 as *const MetaType<S>) == (t2 as *const MetaType<S>) {
            return true;
        }

        match (t1.is_abstract(), t2.is_abstract()) {
            (true, true) => self
                .possible_types(t1)
                .iter()
                .any(|t| self.is_possible_type(t2, t)),
            (true, false) => self.is_possible_type(t1, t2),
            (false, true) => self.is_possible_type(t2, t1),
            (false, false) => false,
        }
    }

    pub fn possible_types(&self, t: &MetaType<S>) -> Vec<&MetaType<S>> {
        match *t {
            MetaType::Union(UnionMeta {
                ref of_type_names, ..
            }) => of_type_names
                .iter()
                .flat_map(|t| self.concrete_type_by_name(t))
                .collect(),
            MetaType::Interface(InterfaceMeta { ref name, .. }) => self
                .concrete_type_list()
                .into_iter()
                .filter(|t| match **t {
                    MetaType::Object(ObjectMeta {
                        ref interface_names,
                        ..
                    }) => interface_names.iter().any(|iname| iname == name),
                    _ => false,
                })
                .collect(),
            _ => panic!("Can't retrieve possible types from non-abstract meta type"),
        }
    }

    pub fn is_possible_type(
        &self,
        abstract_type: &MetaType<S>,
        possible_type: &MetaType<S>,
    ) -> bool {
        self.possible_types(abstract_type)
            .into_iter()
            .any(|t| (t as *const MetaType<S>) == (possible_type as *const MetaType<S>))
    }

    pub fn is_subtype<'b>(&self, sub_type: &Type<'b>, super_type: &Type<'b>) -> bool {
        use crate::ast::Type::*;

        if super_type == sub_type {
            return true;
        }

        match (super_type, sub_type) {
            (&NonNullNamed(ref super_name), &NonNullNamed(ref sub_name))
            | (&Named(ref super_name), &Named(ref sub_name))
            | (&Named(ref super_name), &NonNullNamed(ref sub_name)) => {
                self.is_named_subtype(sub_name, super_name)
            }
            (&NonNullList(ref super_inner), &NonNullList(ref sub_inner))
            | (&List(ref super_inner), &List(ref sub_inner))
            | (&List(ref super_inner), &NonNullList(ref sub_inner)) => {
                self.is_subtype(sub_inner, super_inner)
            }
            _ => false,
        }
    }

    pub fn is_named_subtype(&self, sub_type_name: &str, super_type_name: &str) -> bool {
        if sub_type_name == super_type_name {
            true
        } else if let (Some(sub_type), Some(super_type)) = (
            self.concrete_type_by_name(sub_type_name),
            self.concrete_type_by_name(super_type_name),
        ) {
            super_type.is_abstract() && self.is_possible_type(super_type, sub_type)
        } else {
            false
        }
    }
}

impl<'a, S> TypeType<'a, S> {
    #[inline]
    pub fn to_concrete(&self) -> Option<&'a MetaType<S>> {
        match *self {
            TypeType::Concrete(t) => Some(t),
            _ => None,
        }
    }

    #[inline]
    pub fn innermost_concrete(&self) -> &'a MetaType<S> {
        match *self {
            TypeType::Concrete(t) => t,
            TypeType::NonNull(ref n) | TypeType::List(ref n) => n.innermost_concrete(),
        }
    }

    #[inline]
    pub fn list_contents(&self) -> Option<&TypeType<'a, S>> {
        match *self {
            TypeType::List(ref n) => Some(n),
            TypeType::NonNull(ref n) => n.list_contents(),
            _ => None,
        }
    }

    #[inline]
    pub fn is_non_null(&self) -> bool {
        match *self {
            TypeType::NonNull(_) => true,
            _ => false,
        }
    }
}

impl<'a, S> DirectiveType<'a, S>
where
    S: ScalarValue + 'a,
{
    pub fn new(
        name: &str,
        locations: &[DirectiveLocation],
        arguments: &[Argument<'a, S>],
    ) -> DirectiveType<'a, S> {
        DirectiveType {
            name: name.to_owned(),
            description: None,
            locations: locations.to_vec(),
            arguments: arguments.to_vec(),
        }
    }

    fn new_skip(registry: &mut Registry<'a, S>) -> DirectiveType<'a, S>
    where
        S: ScalarValue,
    {
        Self::new(
            "skip",
            &[
                DirectiveLocation::Field,
                DirectiveLocation::FragmentSpread,
                DirectiveLocation::InlineFragment,
            ],
            &[registry.arg::<bool>("if", &())],
        )
    }

    fn new_include(registry: &mut Registry<'a, S>) -> DirectiveType<'a, S>
    where
        S: ScalarValue,
    {
        Self::new(
            "include",
            &[
                DirectiveLocation::Field,
                DirectiveLocation::FragmentSpread,
                DirectiveLocation::InlineFragment,
            ],
            &[registry.arg::<bool>("if", &())],
        )
    }

    pub fn description(mut self, description: &str) -> DirectiveType<'a, S> {
        self.description = Some(description.to_owned());
        self
    }
}

impl fmt::Display for DirectiveLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            DirectiveLocation::Query => "query",
            DirectiveLocation::Mutation => "mutation",
            DirectiveLocation::Subscription => "subscription",
            DirectiveLocation::Field => "field",
            DirectiveLocation::FragmentDefinition => "fragment definition",
            DirectiveLocation::FragmentSpread => "fragment spread",
            DirectiveLocation::InlineFragment => "inline fragment",
        })
    }
}

impl<'a, S> fmt::Display for TypeType<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TypeType::Concrete(t) => f.write_str(t.name().unwrap()),
            TypeType::List(ref i) => write!(f, "[{}]", i),
            TypeType::NonNull(ref i) => write!(f, "{}!", i),
        }
    }
}
