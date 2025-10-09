use std::ptr;

use arcstr::ArcStr;
use derive_more::with_trait::Display;
use fnv::FnvHashMap;
#[cfg(feature = "schema-language")]
use graphql_parser::schema::Document;

use crate::{
    GraphQLEnum,
    ast::{Type, TypeModifier},
    executor::{Context, Registry},
    schema::meta::{Argument, InterfaceMeta, MetaType, ObjectMeta, PlaceholderMeta, UnionMeta},
    types::{base::GraphQLType, name::Name},
    value::{DefaultScalarValue, ScalarValue},
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
    /// The order of the generated definitions in the returned [`Document`] is NOT stable and may
    /// change without any real schema changes.
    #[must_use]
    pub fn as_document(&self) -> Document<'_, &str> {
        use crate::schema::translate::{
            SchemaTranslator as _, graphql_parser::GraphQLParserTranslator,
        };

        GraphQLParserTranslator::translate_schema(&self.schema)
    }
}

/// Metadata for a schema
#[derive(Debug)]
pub struct SchemaType<S> {
    pub(crate) description: Option<ArcStr>,
    pub(crate) types: FnvHashMap<Name, MetaType<S>>,
    pub(crate) query_type_name: String,
    pub(crate) mutation_type_name: Option<String>,
    pub(crate) subscription_type_name: Option<String>,
    directives: FnvHashMap<ArcStr, DirectiveType<S>>,
}

impl<S> Context for SchemaType<S> {}

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

        let deprecated_directive = DirectiveType::new_deprecated(&mut registry);
        let include_directive = DirectiveType::new_include(&mut registry);
        let one_of_directive = DirectiveType::new_one_of();
        let skip_directive = DirectiveType::new_skip(&mut registry);
        let specified_by_directive = DirectiveType::new_specified_by(&mut registry);
        directives.insert(include_directive.name.clone(), include_directive);
        directives.insert(skip_directive.name.clone(), skip_directive);
        directives.insert(deprecated_directive.name.clone(), deprecated_directive);
        directives.insert(specified_by_directive.name.clone(), specified_by_directive);
        directives.insert(one_of_directive.name.clone(), one_of_directive);

        let mut meta_fields = vec![
            registry.field::<SchemaType<S>>(arcstr::literal!("__schema"), &()),
            registry
                .field::<TypeType<S>>(arcstr::literal!("__type"), &())
                .argument(registry.arg::<String>(arcstr::literal!("name"), &())),
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
    pub fn set_description(&mut self, description: impl Into<ArcStr>) {
        self.description = Some(description.into());
    }

    /// Add a directive like `skip` or `include`.
    pub fn add_directive(&mut self, directive: DirectiveType<S>) {
        self.directives.insert(directive.name.clone(), directive);
    }

    /// Get a type by name.
    pub fn type_by_name(&self, name: impl AsRef<str>) -> Option<TypeType<'_, S>> {
        self.types.get(name.as_ref()).map(|t| TypeType::Concrete(t))
    }

    /// Get a concrete type by name.
    pub fn concrete_type_by_name(&self, name: impl AsRef<str>) -> Option<&MetaType<S>> {
        self.types.get(name.as_ref())
    }

    pub(crate) fn lookup_type(
        &self,
        ty: &Type<impl AsRef<str>, impl AsRef<[TypeModifier]>>,
    ) -> Option<&MetaType<S>> {
        if let Some(name) = ty.name() {
            self.concrete_type_by_name(name)
        } else {
            self.lookup_type(&ty.borrow_inner())
        }
    }

    /// Get the query type from the schema.
    pub fn query_type(&self) -> TypeType<'_, S> {
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
    pub fn mutation_type(&self) -> Option<TypeType<'_, S>> {
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
    pub fn subscription_type(&self) -> Option<TypeType<'_, S>> {
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
    pub fn type_list(&self) -> Vec<TypeType<'_, S>> {
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
    pub fn make_type(
        &self,
        ty: &Type<impl AsRef<str>, impl AsRef<[TypeModifier]>>,
    ) -> TypeType<'_, S> {
        let mut out = self
            .type_by_name(ty.innermost_name())
            .expect("Type not found in schema");
        for m in ty.modifiers() {
            out = match m {
                TypeModifier::NonNull => TypeType::NonNull(out.into()),
                TypeModifier::List(expected_size) => TypeType::List(out.into(), *expected_size),
            };
        }
        out
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

    /// A list of possible types for a given type.
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
            .any(|t| ptr::eq(t, possible_type))
    }

    /// If the type is a subtype of another type.
    pub fn is_subtype(
        &self,
        sub_type: &Type<impl AsRef<str>, impl AsRef<[TypeModifier]>>,
        super_type: &Type<impl AsRef<str>, impl AsRef<[TypeModifier]>>,
    ) -> bool {
        use TypeModifier::{List, NonNull};

        if super_type == sub_type {
            return true;
        }

        match (super_type.modifier(), sub_type.modifier()) {
            (Some(NonNull), Some(NonNull)) => {
                self.is_subtype(&sub_type.borrow_inner(), &super_type.borrow_inner())
            }
            (None | Some(List(..)), Some(NonNull)) => {
                self.is_subtype(&sub_type.borrow_inner(), super_type)
            }
            (Some(List(..)), Some(List(..))) => {
                self.is_subtype(&sub_type.borrow_inner(), &super_type.borrow_inner())
            }
            (None, None) => {
                self.is_named_subtype(sub_type.innermost_name(), super_type.innermost_name())
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

#[derive(Clone, Display)]
pub enum TypeType<'a, S: 'a> {
    #[display("{}", _0.name().unwrap())]
    Concrete(&'a MetaType<S>),
    #[display("{}!", **_0)]
    NonNull(Box<TypeType<'a, S>>),
    #[display("[{}]", **_0)]
    List(Box<TypeType<'a, S>>, Option<usize>),
}

impl<'a, S> TypeType<'a, S> {
    pub fn to_concrete(&self) -> Option<&'a MetaType<S>> {
        match self {
            Self::Concrete(t) => Some(t),
            Self::List(..) | Self::NonNull(..) => None,
        }
    }

    pub fn innermost_concrete(&self) -> &'a MetaType<S> {
        match self {
            Self::Concrete(t) => t,
            Self::NonNull(n) | Self::List(n, ..) => n.innermost_concrete(),
        }
    }

    pub fn list_contents(&self) -> Option<&Self> {
        match self {
            Self::List(n, ..) => Some(n),
            Self::NonNull(n) => n.list_contents(),
            Self::Concrete(..) => None,
        }
    }

    pub fn is_non_null(&self) -> bool {
        matches!(self, TypeType::NonNull(..))
    }
}

#[derive(Debug)]
pub struct DirectiveType<S> {
    pub name: ArcStr,
    pub description: Option<ArcStr>,
    pub locations: Vec<DirectiveLocation>,
    pub arguments: Vec<Argument<S>>,
    pub is_repeatable: bool,
}

impl<S> DirectiveType<S> {
    pub fn new(
        name: impl Into<ArcStr>,
        locations: &[DirectiveLocation],
        arguments: &[Argument<S>],
        is_repeatable: bool,
    ) -> Self
    where
        S: Clone,
    {
        Self {
            name: name.into(),
            description: None,
            locations: locations.to_vec(),
            arguments: arguments.to_vec(),
            is_repeatable,
        }
    }

    fn new_deprecated(registry: &mut Registry<S>) -> Self
    where
        S: ScalarValue,
    {
        Self::new(
            arcstr::literal!("deprecated"),
            &[
                DirectiveLocation::FieldDefinition,
                DirectiveLocation::ArgumentDefinition,
                DirectiveLocation::InputFieldDefinition,
                DirectiveLocation::EnumValue,
            ],
            &[registry.arg_with_default::<String>(
                arcstr::literal!("reason"),
                &"No longer supported".into(),
                &(),
            )],
            false,
        )
    }

    fn new_include(registry: &mut Registry<S>) -> Self
    where
        S: ScalarValue,
    {
        Self::new(
            arcstr::literal!("include"),
            &[
                DirectiveLocation::Field,
                DirectiveLocation::FragmentSpread,
                DirectiveLocation::InlineFragment,
            ],
            &[registry.arg::<bool>(arcstr::literal!("if"), &())],
            false,
        )
    }

    fn new_one_of() -> Self
    where
        S: ScalarValue,
    {
        Self::new(
            arcstr::literal!("oneOf"),
            &[DirectiveLocation::InputObject],
            &[],
            false,
        )
    }

    fn new_skip(registry: &mut Registry<S>) -> Self
    where
        S: ScalarValue,
    {
        Self::new(
            arcstr::literal!("skip"),
            &[
                DirectiveLocation::Field,
                DirectiveLocation::FragmentSpread,
                DirectiveLocation::InlineFragment,
            ],
            &[registry.arg::<bool>(arcstr::literal!("if"), &())],
            false,
        )
    }

    fn new_specified_by(registry: &mut Registry<S>) -> Self
    where
        S: ScalarValue,
    {
        Self::new(
            arcstr::literal!("specifiedBy"),
            &[DirectiveLocation::Scalar],
            &[registry.arg::<String>(arcstr::literal!("url"), &())],
            false,
        )
    }

    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Clone, Debug, Display, Eq, GraphQLEnum, PartialEq)]
#[graphql(name = "__DirectiveLocation", internal)]
pub enum DirectiveLocation {
    #[display("query")]
    Query,
    #[display("mutation")]
    Mutation,
    #[display("subscription")]
    Subscription,
    #[display("field")]
    Field,
    #[display("fragment definition")]
    FragmentDefinition,
    #[display("fragment spread")]
    FragmentSpread,
    #[display("inline fragment")]
    InlineFragment,
    #[display("variable definition")]
    VariableDefinition,
    #[display("schema")]
    Schema,
    #[display("scalar")]
    Scalar,
    #[display("object")]
    Object,
    #[display("field definition")]
    FieldDefinition,
    #[display("argument definition")]
    ArgumentDefinition,
    #[display("interface")]
    Interface,
    #[display("union")]
    Union,
    #[display("enum")]
    Enum,
    #[display("enum value")]
    EnumValue,
    #[display("input object")]
    InputObject,
    #[display("input field definition")]
    InputFieldDefinition,
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
            TypeType::Concrete(MetaType::Enum(..)) => 0,
            TypeType::Concrete(MetaType::InputObject(..)) => 1,
            TypeType::Concrete(MetaType::Interface(..)) => 2,
            TypeType::Concrete(MetaType::Scalar(..)) => 3,
            TypeType::Concrete(MetaType::Object(..)) => 4,
            TypeType::Concrete(MetaType::Union(..)) => 5,
            // NOTE: The following types are not part of the introspected types.
            TypeType::Concrete(
                MetaType::List(..) | MetaType::Nullable(..) | MetaType::Placeholder(..),
            ) => 6,
            // NOTE: Other variants will not appear since we're only sorting concrete types.
            TypeType::List(..) | TypeType::NonNull(..) => 7,
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
                MetaType::List(..) | MetaType::Nullable(..) | MetaType::Placeholder(..),
            )
            // NOTE: Other variants will not appear since we're only sorting concrete types.
            | TypeType::List(..)
            | TypeType::NonNull(..) => None,
        }
    }
}

#[cfg(test)]
mod root_node_test {
    #[cfg(feature = "schema-language")]
    mod as_document {
        use crate::{EmptyMutation, EmptySubscription, RootNode, graphql_object};

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
            EmptyMutation, EmptySubscription, GraphQLEnum, GraphQLInputObject, GraphQLObject,
            GraphQLUnion, RootNode, graphql_object,
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
