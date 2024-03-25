use std::fmt;

use arcstr::ArcStr;
use fnv::FnvHashMap;
#[cfg(feature = "schema-language")]
use graphql_parser::schema::Document;

use crate::{
    ast::Type,
    executor::{Context, Registry},
    literal,
    schema::meta::{Argument, InterfaceMeta, MetaType, ObjectMeta, PlaceholderMeta, UnionMeta},
    types::{base::GraphQLType, name::Name},
    value::{DefaultScalarValue, ScalarValue},
    GraphQLEnum,
};

/// Root query node of a schema
///
/// This brings the mutation, subscription and query types together,
/// and provides the predefined metadata fields.
#[derive(Debug)]
pub struct RootNode<
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    SubscriptionT: GraphQLType<S>,
    S = DefaultScalarValue,
> where
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
    pub subscription_type: SubscriptionT,
    #[doc(hidden)]
    pub subscription_info: SubscriptionT::TypeInfo,
    #[doc(hidden)]
    pub schema: SchemaType<S>,
    #[doc(hidden)]
    pub introspection_disabled: bool,
}

/// Metadata for a schema
#[derive(Debug)]
pub struct SchemaType<S> {
    pub(crate) description: Option<ArcStr>,
    pub(crate) types: FnvHashMap<Name, MetaType<S>>,
    pub(crate) query_type_name: String,
    pub(crate) mutation_type_name: Option<String>,
    pub(crate) subscription_type_name: Option<String>,
    directives: FnvHashMap<String, DirectiveType<S>>,
}

impl<S> Context for SchemaType<S> {}

#[derive(Clone)]
pub enum TypeType<'a, S: 'a> {
    Concrete(&'a MetaType<S>),
    NonNull(Box<TypeType<'a, S>>),
    List(Box<TypeType<'a, S>>, Option<usize>),
}

#[derive(Debug)]
pub struct DirectiveType<S> {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<DirectiveLocation>,
    pub arguments: Vec<Argument<S>>,
    pub is_repeatable: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, GraphQLEnum)]
#[graphql(name = "__DirectiveLocation", internal)]
pub enum DirectiveLocation {
    Query,
    Mutation,
    Subscription,
    Field,
    Scalar,
    #[graphql(name = "FRAGMENT_DEFINITION")]
    FragmentDefinition,
    #[graphql(name = "FIELD_DEFINITION")]
    FieldDefinition,
    #[graphql(name = "VARIABLE_DEFINITION")]
    VariableDefinition,
    #[graphql(name = "FRAGMENT_SPREAD")]
    FragmentSpread,
    #[graphql(name = "INLINE_FRAGMENT")]
    InlineFragment,
    #[graphql(name = "ENUM_VALUE")]
    EnumValue,
}

/// Allows seeing [Type] with different name/string representations
/// as the same type without allocating.
//
// TODO: Ideally this type should not exist, but the reason it currently does
// is that [Type] has a recursive design to allow arbitrary number of list wrappings.
// The list layout could instead be modelled as a modifier so that type becomes a tuple of (name, modifier).
//
// If [Type] is modelled like this it becomes easier to project it as a borrowed version of itself,
// i.e. [Type<ArcStr>] vs [Type<&str>].
#[derive(Clone, Copy, Debug)]
pub enum DynType<'a> {
    Named(&'a str),
    List(&'a dyn AsDynType, Option<usize>),
    NonNullNamed(&'a str),
    NonNullList(&'a dyn AsDynType, Option<usize>),
}

/// Trait for converting a [Type] into [DynType]
pub trait AsDynType: fmt::Debug {
    /// Project [self] as a [DynType].
    ///
    /// this function should not allocate memory.
    fn as_dyn_type(&self) -> DynType<'_>;
}

impl AsDynType for Type<ArcStr> {
    fn as_dyn_type(&self) -> DynType<'_> {
        match self {
            Self::Named(n) => DynType::Named(n.as_str()),
            Self::List(t, s) => DynType::List(t.as_ref(), *s),
            Self::NonNullNamed(n) => DynType::NonNullNamed(n.as_str()),
            Self::NonNullList(t, s) => DynType::NonNullList(t.as_ref(), *s),
        }
    }
}

impl<'a> AsDynType for Type<&'a str> {
    fn as_dyn_type(&self) -> DynType<'_> {
        match self {
            Self::Named(n) => DynType::Named(n),
            Self::List(t, s) => DynType::List(t.as_ref(), *s),
            Self::NonNullNamed(n) => DynType::NonNullNamed(n),
            Self::NonNullList(t, s) => DynType::NonNullList(t.as_ref(), *s),
        }
    }
}

impl<QueryT, MutationT, SubscriptionT>
    RootNode<QueryT, MutationT, SubscriptionT, DefaultScalarValue>
where
    QueryT: GraphQLType<DefaultScalarValue, TypeInfo = ()>,
    MutationT: GraphQLType<DefaultScalarValue, TypeInfo = ()>,
    SubscriptionT: GraphQLType<DefaultScalarValue, TypeInfo = ()>,
{
    /// Constructs a new [`RootNode`] from `query`, `mutation` and `subscription` nodes,
    /// parametrizing it with a [`DefaultScalarValue`].
    pub fn new(query: QueryT, mutation: MutationT, subscription: SubscriptionT) -> Self {
        Self::new_with_info(query, mutation, subscription, (), (), ())
    }
}

impl<QueryT, MutationT, SubscriptionT, S> RootNode<QueryT, MutationT, SubscriptionT, S>
where
    S: ScalarValue,
    QueryT: GraphQLType<S, TypeInfo = ()>,
    MutationT: GraphQLType<S, TypeInfo = ()>,
    SubscriptionT: GraphQLType<S, TypeInfo = ()>,
{
    /// Constructs a new [`RootNode`] from `query`, `mutation` and `subscription` nodes,
    /// parametrizing it with the provided [`ScalarValue`].
    pub fn new_with_scalar_value(
        query: QueryT,
        mutation: MutationT,
        subscription: SubscriptionT,
    ) -> Self {
        RootNode::new_with_info(query, mutation, subscription, (), (), ())
    }
}

impl<S, QueryT, MutationT, SubscriptionT> RootNode<QueryT, MutationT, SubscriptionT, S>
where
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    SubscriptionT: GraphQLType<S>,
    S: ScalarValue,
{
    /// Construct a new root node from query and mutation nodes,
    /// while also providing type info objects for the query and
    /// mutation types.
    pub fn new_with_info(
        query_obj: QueryT,
        mutation_obj: MutationT,
        subscription_obj: SubscriptionT,
        query_info: QueryT::TypeInfo,
        mutation_info: MutationT::TypeInfo,
        subscription_info: SubscriptionT::TypeInfo,
    ) -> Self {
        Self {
            query_type: query_obj,
            mutation_type: mutation_obj,
            subscription_type: subscription_obj,
            schema: SchemaType::new::<QueryT, MutationT, SubscriptionT>(
                &query_info,
                &mutation_info,
                &subscription_info,
            ),
            query_info,
            mutation_info,
            subscription_info,
            introspection_disabled: false,
        }
    }

    /// Disables introspection for this [`RootNode`], making it to return a [`FieldError`] whenever
    /// its `__schema` or `__type` field is resolved.
    ///
    /// By default, all introspection queries are allowed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use juniper::{
    /// #     graphql_object, graphql_vars, EmptyMutation, EmptySubscription, GraphQLError,
    /// #     RootNode,
    /// # };
    /// #
    /// pub struct Query;
    ///
    /// #[graphql_object]
    /// impl Query {
    ///     fn some() -> bool {
    ///         true
    ///     }
    /// }
    ///
    /// type Schema = RootNode<Query, EmptyMutation, EmptySubscription>;
    ///
    /// let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new())
    ///     .disable_introspection();
    ///
    /// # // language=GraphQL
    /// let query = "query { __schema { queryType { name } } }";
    ///
    /// match juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()) {
    ///     Err(GraphQLError::ValidationError(errs)) => {
    ///         assert_eq!(
    ///             errs.first().unwrap().message(),
    ///             "GraphQL introspection is not allowed, but the operation contained `__schema`",
    ///         );
    ///     }
    ///     res => panic!("expected `ValidationError`, returned: {res:#?}"),
    /// }
    /// ```
    pub fn disable_introspection(mut self) -> Self {
        self.introspection_disabled = true;
        self
    }

    /// Enables introspection for this [`RootNode`], if it was previously [disabled][1].
    ///
    /// By default, all introspection queries are allowed.
    ///
    /// [1]: RootNode::disable_introspection
    pub fn enable_introspection(mut self) -> Self {
        self.introspection_disabled = false;
        self
    }

    #[cfg(feature = "schema-language")]
    /// Returns this [`RootNode`] as a [`String`] containing the schema in [SDL (schema definition language)].
    ///
    /// # Sorted
    ///
    /// The order of the generated definitions is stable and is sorted in the "type-then-name" manner.
    ///
    /// If another sorting order is required, then the [`as_document()`] method should be used, which allows to sort the
    /// returned [`Document`] in the desired manner and then to convert it [`to_string()`].
    ///
    /// [`as_document()`]: RootNode::as_document
    /// [`to_string()`]: ToString::to_string
    /// [0]: https://graphql.org/learn/schema#type-language
    #[must_use]
    pub fn as_sdl(&self) -> String {
        use crate::schema::translate::graphql_parser::sort_schema_document;

        let mut doc = self.as_document();
        sort_schema_document(&mut doc);
        doc.to_string()
    }

    #[cfg(feature = "schema-language")]
    /// Returns this [`RootNode`] as a [`graphql_parser`]'s [`Document`].
    ///
    /// # Unsorted
    ///
    /// The order of the generated definitions in the returned [`Document`] is NOT stable and may change without any
    /// real schema changes.
    #[must_use]
    pub fn as_document(&self) -> Document<&str> {
        use crate::schema::translate::{
            graphql_parser::GraphQLParserTranslator, SchemaTranslator as _,
        };

        GraphQLParserTranslator::translate_schema(&self.schema)
    }
}

impl<S> SchemaType<S> {
    /// Create a new schema.
    pub fn new<QueryT, MutationT, SubscriptionT>(
        query_info: &QueryT::TypeInfo,
        mutation_info: &MutationT::TypeInfo,
        subscription_info: &SubscriptionT::TypeInfo,
    ) -> Self
    where
        S: ScalarValue,
        QueryT: GraphQLType<S>,
        MutationT: GraphQLType<S>,
        SubscriptionT: GraphQLType<S>,
    {
        let mut directives = FnvHashMap::default();
        let mut registry = Registry::new(FnvHashMap::default());

        let query_type_name: Box<str> = registry
            .get_type::<QueryT>(query_info)
            .innermost_name()
            .into();
        let mutation_type_name: Box<str> = registry
            .get_type::<MutationT>(mutation_info)
            .innermost_name()
            .into();
        let subscription_type_name: Box<str> = registry
            .get_type::<SubscriptionT>(subscription_info)
            .innermost_name()
            .into();

        registry.get_type::<SchemaType<S>>(&());

        directives.insert("skip".into(), DirectiveType::new_skip(&mut registry));
        directives.insert("include".into(), DirectiveType::new_include(&mut registry));
        directives.insert(
            "deprecated".into(),
            DirectiveType::new_deprecated(&mut registry),
        );
        directives.insert(
            "specifiedBy".into(),
            DirectiveType::new_specified_by(&mut registry),
        );

        let mut meta_fields = vec![
            registry.field::<SchemaType<S>>(literal!("__schema"), &()),
            registry
                .field::<TypeType<S>>(literal!("__type"), &())
                .argument(registry.arg::<String>(literal!("name"), &())),
        ];

        if let Some(root_type) = registry.types.get_mut(query_type_name.as_ref()) {
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
                panic!("Type {of_type:?} is still a placeholder type");
            }
        }
        SchemaType {
            description: None,
            types: registry.types,
            query_type_name: query_type_name.into(),
            mutation_type_name: if mutation_type_name.as_ref() != "_EmptyMutation" {
                Some(mutation_type_name.into())
            } else {
                None
            },
            subscription_type_name: if subscription_type_name.as_ref() != "_EmptySubscription" {
                Some(subscription_type_name.into())
            } else {
                None
            },
            directives,
        }
    }

    /// Add a description.
    pub fn set_description(&mut self, description: ArcStr) {
        self.description = Some(description);
    }

    /// Add a directive like `skip` or `include`.
    pub fn add_directive(&mut self, directive: DirectiveType<S>) {
        self.directives.insert(directive.name.clone(), directive);
    }

    /// Get a type by name.
    pub fn type_by_name(&self, name: &str) -> Option<TypeType<S>> {
        self.types.get(name).map(|t| TypeType::Concrete(t))
    }

    /// Get a concrete type by name.
    pub fn concrete_type_by_name(&self, name: &str) -> Option<&MetaType<S>> {
        self.types.get(name)
    }

    pub(crate) fn lookup_type<N: AsRef<str>>(&self, tpe: &Type<N>) -> Option<&MetaType<S>> {
        match *tpe {
            Type::NonNullNamed(ref name) | Type::Named(ref name) => {
                self.concrete_type_by_name(name.as_ref())
            }
            Type::List(ref inner, _) | Type::NonNullList(ref inner, _) => self.lookup_type(inner),
        }
    }

    /// Get the query type from the schema.
    pub fn query_type(&self) -> TypeType<S> {
        TypeType::Concrete(
            self.types
                .get(self.query_type_name.as_str())
                .expect("Query type does not exist in schema"),
        )
    }

    /// Get the concrete query type from the schema.
    pub fn concrete_query_type(&self) -> &MetaType<S> {
        self.types
            .get(self.query_type_name.as_str())
            .expect("Query type does not exist in schema")
    }

    /// Get the mutation type from the schema.
    pub fn mutation_type(&self) -> Option<TypeType<S>> {
        self.mutation_type_name.as_ref().map(|name| {
            self.type_by_name(name)
                .expect("Mutation type does not exist in schema")
        })
    }

    /// Get the concrete mutation type from the schema.
    pub fn concrete_mutation_type(&self) -> Option<&MetaType<S>> {
        self.mutation_type_name.as_ref().map(|name| {
            self.concrete_type_by_name(name)
                .expect("Mutation type does not exist in schema")
        })
    }

    /// Get the subscription type.
    pub fn subscription_type(&self) -> Option<TypeType<S>> {
        self.subscription_type_name.as_ref().map(|name| {
            self.type_by_name(name)
                .expect("Subscription type does not exist in schema")
        })
    }

    /// Get the concrete subscription type.
    pub fn concrete_subscription_type(&self) -> Option<&MetaType<S>> {
        self.subscription_type_name.as_ref().map(|name| {
            self.concrete_type_by_name(name)
                .expect("Subscription type does not exist in schema")
        })
    }

    /// Get a list of types.
    pub fn type_list(&self) -> Vec<TypeType<S>> {
        let mut types = self
            .types
            .values()
            .map(|t| TypeType::Concrete(t))
            .collect::<Vec<_>>();
        sort_concrete_types(&mut types);
        types
    }

    /// Get a list of concrete types.
    pub fn concrete_type_list(&self) -> Vec<&MetaType<S>> {
        self.types.values().collect()
    }

    /// Make a type.
    pub fn make_type(&self, t: DynType) -> TypeType<S> {
        match t {
            DynType::NonNullNamed(n) => TypeType::NonNull(Box::new(
                self.type_by_name(n.as_ref())
                    .expect("Type not found in schema"),
            )),
            DynType::NonNullList(inner, expected_size) => TypeType::NonNull(Box::new(
                TypeType::List(Box::new(self.make_type(inner.as_dyn_type())), expected_size),
            )),
            DynType::Named(n) => self
                .type_by_name(n.as_ref())
                .expect("Type not found in schema"),
            DynType::List(inner, expected_size) => {
                TypeType::List(Box::new(self.make_type(inner.as_dyn_type())), expected_size)
            }
        }
    }

    /// Get a list of directives.
    pub fn directive_list(&self) -> Vec<&DirectiveType<S>> {
        let mut directives = self.directives.values().collect::<Vec<_>>();
        sort_directives(&mut directives);
        directives
    }

    /// Get directive by name.
    pub fn directive_by_name(&self, name: &str) -> Option<&DirectiveType<S>> {
        self.directives.get(name)
    }

    /// Determine if there is an overlap between types.
    pub fn type_overlap(&self, t1: &MetaType<S>, t2: &MetaType<S>) -> bool {
        if std::ptr::eq(t1, t2) {
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

    /// A list of possible typeees for a given type.
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

    /// If the abstract type is possible.
    pub fn is_possible_type(
        &self,
        abstract_type: &MetaType<S>,
        possible_type: &MetaType<S>,
    ) -> bool {
        self.possible_types(abstract_type)
            .into_iter()
            .any(|t| (std::ptr::eq(t, possible_type)))
    }

    /// If the type is a subtype of another type.
    pub fn is_subtype<'b>(&self, sub_type: &DynType<'b>, super_type: &DynType<'b>) -> bool {
        use DynType::*;

        if super_type.equals(sub_type) {
            return true;
        }

        match (super_type, sub_type) {
            (&NonNullNamed(ref super_name), &NonNullNamed(ref sub_name))
            | (&Named(ref super_name), &Named(ref sub_name))
            | (&Named(ref super_name), &NonNullNamed(ref sub_name)) => {
                self.is_named_subtype(sub_name.as_ref(), super_name.as_ref())
            }
            (&NonNullList(super_inner, _), &NonNullList(sub_inner, _))
            | (&List(super_inner, _), &List(sub_inner, _))
            | (&List(super_inner, _), &NonNullList(sub_inner, _)) => {
                self.is_subtype(&sub_inner.as_dyn_type(), &super_inner.as_dyn_type())
            }
            _ => false,
        }
    }

    /// If the type is a named subtype.
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
            TypeType::NonNull(ref n) | TypeType::List(ref n, _) => n.innermost_concrete(),
        }
    }

    #[inline]
    pub fn list_contents(&self) -> Option<&TypeType<'a, S>> {
        match *self {
            TypeType::List(ref n, _) => Some(n),
            TypeType::NonNull(ref n) => n.list_contents(),
            _ => None,
        }
    }

    #[inline]
    pub fn is_non_null(&self) -> bool {
        matches!(*self, TypeType::NonNull(_))
    }
}

impl<S> DirectiveType<S>
where
    S: ScalarValue,
{
    pub fn new(
        name: &str,
        locations: &[DirectiveLocation],
        arguments: &[Argument<S>],
        is_repeatable: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            locations: locations.to_vec(),
            arguments: arguments.to_vec(),
            is_repeatable,
        }
    }

    fn new_skip(registry: &mut Registry<S>) -> DirectiveType<S>
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
            &[registry.arg::<bool>(literal!("if"), &())],
            false,
        )
    }

    fn new_include(registry: &mut Registry<S>) -> DirectiveType<S>
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
            &[registry.arg::<bool>(literal!("if"), &())],
            false,
        )
    }

    fn new_deprecated(registry: &mut Registry<S>) -> DirectiveType<S>
    where
        S: ScalarValue,
    {
        Self::new(
            "deprecated",
            &[
                DirectiveLocation::FieldDefinition,
                DirectiveLocation::EnumValue,
            ],
            &[registry.arg::<String>(literal!("reason"), &())],
            false,
        )
    }

    fn new_specified_by(registry: &mut Registry<S>) -> DirectiveType<S>
    where
        S: ScalarValue,
    {
        Self::new(
            "specifiedBy",
            &[DirectiveLocation::Scalar],
            &[registry.arg::<String>(literal!("url"), &())],
            false,
        )
    }

    pub fn description(mut self, description: &str) -> DirectiveType<S> {
        self.description = Some(description.into());
        self
    }
}

impl fmt::Display for DirectiveLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Self::Query => "query",
            Self::Mutation => "mutation",
            Self::Subscription => "subscription",
            Self::Field => "field",
            Self::FieldDefinition => "field definition",
            Self::FragmentDefinition => "fragment definition",
            Self::FragmentSpread => "fragment spread",
            Self::InlineFragment => "inline fragment",
            Self::VariableDefinition => "variable definition",
            Self::Scalar => "scalar",
            Self::EnumValue => "enum value",
        })
    }
}

impl<'a, S> fmt::Display for TypeType<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Concrete(t) => f.write_str(t.name().unwrap()),
            Self::List(i, _) => write!(f, "[{i}]"),
            Self::NonNull(i) => write!(f, "{i}!"),
        }
    }
}

impl<'a> DynType<'a> {
    pub fn equals(&self, other: &DynType) -> bool {
        match (self, other) {
            (DynType::Named(n0), DynType::Named(n1)) => n0 == n1,
            (DynType::List(t0, s0), DynType::List(t1, s1)) => {
                t0.as_dyn_type().equals(&t1.as_dyn_type()) && s0 == s1
            }
            (DynType::NonNullNamed(n0), DynType::NonNullNamed(n1)) => n0 == n1,
            (DynType::NonNullList(t0, s0), DynType::NonNullList(t1, s1)) => {
                t0.as_dyn_type().equals(&t1.as_dyn_type()) && s0 == s1
            }
            _ => false,
        }
    }

    pub fn innermost_name(&self) -> &'a str {
        match self {
            Self::Named(n) | Self::NonNullNamed(n) => n,
            Self::List(l, _) | Self::NonNullList(l, _) => l.as_dyn_type().innermost_name(),
        }
    }
}

impl<'a> fmt::Display for DynType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(n) => write!(f, "{n}"),
            Self::NonNullNamed(n) => write!(f, "{n}!"),
            Self::List(t, _) => write!(f, "[{}]", t.as_dyn_type()),
            Self::NonNullList(t, _) => write!(f, "[{}]!", t.as_dyn_type()),
        }
    }
}

/// Sorts the provided [`TypeType`]s in the "type-then-name" manner.
fn sort_concrete_types<S>(types: &mut [TypeType<S>]) {
    types.sort_by(|a, b| {
        concrete_type_sort::by_type(a)
            .cmp(&concrete_type_sort::by_type(b))
            .then_with(|| concrete_type_sort::by_name(a).cmp(&concrete_type_sort::by_name(b)))
    });
}

/// Sorts the provided [`DirectiveType`]s by name.
fn sort_directives<S>(directives: &mut [&DirectiveType<S>]) {
    directives.sort_by(|a, b| a.name.cmp(&b.name));
}

/// Evaluation of a [`TypeType`] weights for sorting (for concrete types only).
///
/// Used for deterministic introspection output.
mod concrete_type_sort {
    use crate::meta::MetaType;

    use super::TypeType;

    /// Returns a [`TypeType`] sorting weight by its type.
    pub fn by_type<S>(t: &TypeType<S>) -> u8 {
        match t {
            TypeType::Concrete(MetaType::Enum(_)) => 0,
            TypeType::Concrete(MetaType::InputObject(_)) => 1,
            TypeType::Concrete(MetaType::Interface(_)) => 2,
            TypeType::Concrete(MetaType::Scalar(_)) => 3,
            TypeType::Concrete(MetaType::Object(_)) => 4,
            TypeType::Concrete(MetaType::Union(_)) => 5,
            // NOTE: The following types are not part of the introspected types.
            TypeType::Concrete(
                MetaType::List(_) | MetaType::Nullable(_) | MetaType::Placeholder(_),
            ) => 6,
            // NOTE: Other variants will not appear since we're only sorting concrete types.
            TypeType::List(..) | TypeType::NonNull(_) => 7,
        }
    }

    /// Returns a [`TypeType`] sorting weight by its name.
    pub fn by_name<'a, S>(t: &'a TypeType<'a, S>) -> Option<&'a str> {
        match t {
            TypeType::Concrete(MetaType::Enum(meta)) => Some(&meta.name),
            TypeType::Concrete(MetaType::InputObject(meta)) => Some(&meta.name),
            TypeType::Concrete(MetaType::Interface(meta)) => Some(&meta.name),
            TypeType::Concrete(MetaType::Scalar(meta)) => Some(&meta.name),
            TypeType::Concrete(MetaType::Object(meta)) => Some(&meta.name),
            TypeType::Concrete(MetaType::Union(meta)) => Some(&meta.name),
            TypeType::Concrete(
                // NOTE: The following types are not part of the introspected types.
                MetaType::List(_) | MetaType::Nullable(_) | MetaType::Placeholder(_),
            )
            // NOTE: Other variants will not appear since we're only sorting concrete types.
            | TypeType::List(..)
            | TypeType::NonNull(_) => None,
        }
    }
}

#[cfg(test)]
mod root_node_test {
    #[cfg(feature = "schema-language")]
    mod as_document {
        use crate::{graphql_object, EmptyMutation, EmptySubscription, RootNode};

        struct Query;

        #[graphql_object]
        impl Query {
            fn blah() -> bool {
                true
            }
        }

        #[test]
        fn generates_correct_document() {
            let schema = RootNode::new(
                Query,
                EmptyMutation::<()>::new(),
                EmptySubscription::<()>::new(),
            );
            let ast = graphql_parser::parse_schema::<&str>(
                //language=GraphQL
                r#"
                type Query {
                    blah: Boolean!
                }

                schema {
                  query: Query
                }
                "#,
            )
            .unwrap();

            assert_eq!(ast.to_string(), schema.as_document().to_string());
        }
    }

    #[cfg(feature = "schema-language")]
    mod as_sdl {
        use crate::{
            graphql_object, EmptyMutation, EmptySubscription, GraphQLEnum, GraphQLInputObject,
            GraphQLObject, GraphQLUnion, RootNode,
        };

        #[derive(GraphQLObject, Default)]
        struct Cake {
            fresh: bool,
        }

        #[derive(GraphQLObject, Default)]
        struct IceCream {
            cold: bool,
        }

        #[derive(GraphQLUnion)]
        enum GlutenFree {
            Cake(Cake),
            IceCream(IceCream),
        }

        #[derive(GraphQLEnum)]
        enum Fruit {
            Apple,
            Orange,
        }

        #[derive(GraphQLInputObject)]
        struct Coordinate {
            latitude: f64,
            longitude: f64,
        }

        struct Query;

        #[graphql_object]
        impl Query {
            fn blah() -> bool {
                true
            }

            /// This is whatever's description.
            fn whatever() -> String {
                "foo".into()
            }

            fn arr(stuff: Vec<Coordinate>) -> Option<&'static str> {
                (!stuff.is_empty()).then_some("stuff")
            }

            fn fruit() -> Fruit {
                Fruit::Apple
            }

            fn gluten_free(flavor: String) -> GlutenFree {
                if flavor == "savory" {
                    GlutenFree::Cake(Cake::default())
                } else {
                    GlutenFree::IceCream(IceCream::default())
                }
            }

            #[deprecated]
            fn old() -> i32 {
                42
            }

            #[deprecated(note = "This field is deprecated, use another.")]
            fn really_old() -> f64 {
                42.0
            }
        }

        #[test]
        fn generates_correct_sdl() {
            let actual = RootNode::new(
                Query,
                EmptyMutation::<()>::new(),
                EmptySubscription::<()>::new(),
            );
            let expected = graphql_parser::parse_schema::<&str>(
                //language=GraphQL
                r#"
                schema {
                  query: Query
                }
                enum Fruit {
                    APPLE
                    ORANGE
                }
                input Coordinate {
                    latitude: Float!
                    longitude: Float!
                }
                type Cake {
                    fresh: Boolean!
                }
                type IceCream {
                    cold: Boolean!
                }
                type Query {
                  blah: Boolean!
                  "This is whatever's description."
                  whatever: String!
                  arr(stuff: [Coordinate!]!): String
                  fruit: Fruit!
                  glutenFree(flavor: String!): GlutenFree!
                  old: Int! @deprecated
                  reallyOld: Float! @deprecated(reason: "This field is deprecated, use another.")
                }
                union GlutenFree = Cake | IceCream
                "#,
            )
            .unwrap();

            assert_eq!(actual.as_sdl(), expected.to_string());
        }
    }
}
