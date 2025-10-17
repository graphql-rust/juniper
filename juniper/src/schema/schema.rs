//! GraphQL [Type System Definitions][0].
//!
//! [0]:https://spec.graphql.org/September2025/#sec-Schema-Introspection.Schema-Introspection-Schema

use arcstr::ArcStr;

use crate::{
    ast::Selection,
    executor::{ExecutionResult, Executor, Registry},
    graphql_object,
    types::{
        async_await::{GraphQLTypeAsync, GraphQLValueAsync},
        base::{Arguments, GraphQLType, GraphQLValue, TypeKind},
    },
    value::{ScalarValue, Value},
};

use crate::schema::{
    meta::{
        Argument, EnumMeta, EnumValue, Field, InputObjectMeta, InterfaceMeta, MetaType, ObjectMeta,
        UnionMeta,
    },
    model::{DirectiveLocation, DirectiveType, RootNode, SchemaType, TypeType},
};

impl<S, QueryT, MutationT, SubscriptionT> GraphQLType<S>
    for RootNode<QueryT, MutationT, SubscriptionT, S>
where
    S: ScalarValue,
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S, Context = QueryT::Context>,
    SubscriptionT: GraphQLType<S, Context = QueryT::Context>,
{
    fn name(info: &Self::TypeInfo) -> Option<ArcStr> {
        QueryT::name(info)
    }

    fn meta(info: &Self::TypeInfo, registry: &mut Registry<S>) -> MetaType<S> {
        QueryT::meta(info, registry)
    }
}

impl<S, QueryT, MutationT, SubscriptionT> GraphQLValue<S>
    for RootNode<QueryT, MutationT, SubscriptionT, S>
where
    S: ScalarValue,
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S, Context = QueryT::Context>,
    SubscriptionT: GraphQLType<S, Context = QueryT::Context>,
{
    type Context = QueryT::Context;
    type TypeInfo = QueryT::TypeInfo;

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        QueryT::name(info)
    }

    fn concrete_type_name(&self, context: &Self::Context, info: &Self::TypeInfo) -> String {
        self.query_type.concrete_type_name(context, info)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        args: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        match field {
            "__schema" => executor
                .replaced_context(&self.schema)
                .resolve(&(), &self.schema),
            "__type" => {
                let type_name: String = args.get("name")?.unwrap();
                executor
                    .replaced_context(&self.schema)
                    .resolve(&(), &self.schema.type_by_name(&type_name))
            }
            _ => self.query_type.resolve_field(info, field, args, executor),
        }
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        use crate::{types::base::resolve_selection_set_into, value::Object};
        if let Some(selection_set) = selection_set {
            let mut result = Object::with_capacity(selection_set.len());
            if resolve_selection_set_into(self, info, selection_set, executor, &mut result) {
                Ok(Value::Object(result))
            } else {
                Ok(Value::null())
            }
        } else {
            // TODO: this panic seems useless, investigate why it is here.
            panic!("resolve() must be implemented by non-object output types");
        }
    }
}

impl<S, QueryT, MutationT, SubscriptionT> GraphQLValueAsync<S>
    for RootNode<QueryT, MutationT, SubscriptionT, S>
where
    QueryT: GraphQLTypeAsync<S>,
    QueryT::TypeInfo: Sync,
    QueryT::Context: Sync,
    MutationT: GraphQLTypeAsync<S, Context = QueryT::Context>,
    MutationT::TypeInfo: Sync,
    SubscriptionT: GraphQLType<S, Context = QueryT::Context> + Sync,
    SubscriptionT::TypeInfo: Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_field_async<'b>(
        &'b self,
        info: &'b Self::TypeInfo,
        field_name: &'b str,
        arguments: &'b Arguments<S>,
        executor: &'b Executor<Self::Context, S>,
    ) -> crate::BoxFuture<'b, ExecutionResult<S>> {
        use std::future;

        match field_name {
            "__schema" | "__type" => {
                let v = self.resolve_field(info, field_name, arguments, executor);
                Box::pin(future::ready(v))
            }
            _ => self
                .query_type
                .resolve_field_async(info, field_name, arguments, executor),
        }
    }
}

#[graphql_object]
#[graphql(
    name = "__Schema"
    context = SchemaType<S>,
    scalar = S,
    internal,
)]
impl<S: ScalarValue> SchemaType<S> {
    fn description(&self) -> Option<&ArcStr> {
        self.description.as_ref()
    }

    fn types(&self) -> Vec<TypeType<'_, S>> {
        self.type_list()
            .into_iter()
            .filter(|t| {
                t.to_concrete()
                    .map(|t| {
                        !(t.name().map(ArcStr::as_str) == Some("_EmptyMutation")
                            || t.name().map(ArcStr::as_str) == Some("_EmptySubscription"))
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    #[graphql(name = "queryType")]
    fn query_type_(&self) -> TypeType<'_, S> {
        self.query_type()
    }

    #[graphql(name = "mutationType")]
    fn mutation_type_(&self) -> Option<TypeType<'_, S>> {
        self.mutation_type()
    }

    #[graphql(name = "subscriptionType")]
    fn subscription_type_(&self) -> Option<TypeType<'_, S>> {
        self.subscription_type()
    }

    fn directives(&self) -> Vec<&DirectiveType<S>> {
        self.directive_list()
    }
}

#[graphql_object]
#[graphql(
    name = "__Type",
    context = SchemaType<S>,
    scalar = S,
    internal,
)]
impl<'a, S: ScalarValue + 'a> TypeType<'a, S> {
    fn kind(&self) -> TypeKind {
        match self {
            Self::Concrete(t) => t.type_kind(),
            Self::List(..) => TypeKind::List,
            Self::NonNull(..) => TypeKind::NonNull,
        }
    }

    fn name(&self) -> Option<&ArcStr> {
        match self {
            Self::Concrete(t) => t.name(),
            Self::List(..) | Self::NonNull(..) => None,
        }
    }

    fn description(&self) -> Option<&ArcStr> {
        match self {
            Self::Concrete(t) => t.description(),
            Self::List(..) | Self::NonNull(..) => None,
        }
    }

    #[graphql(name = "specifiedByURL")]
    fn specified_by_url(&self) -> Option<&ArcStr> {
        match self {
            Self::Concrete(t) => t.specified_by_url(),
            Self::List(..) | Self::NonNull(..) => None,
        }
    }

    fn fields(&self, #[graphql(default)] include_deprecated: bool) -> Option<Vec<&Field<S>>> {
        match self {
            Self::Concrete(t) => match t {
                MetaType::Interface(InterfaceMeta { fields, .. })
                | MetaType::Object(ObjectMeta { fields, .. }) => Some(
                    fields
                        .iter()
                        .filter(|f| include_deprecated || !f.deprecation_status.is_deprecated())
                        .filter(|f| !f.name.starts_with("__"))
                        .collect(),
                ),
                MetaType::Enum(..)
                | MetaType::InputObject(..)
                | MetaType::List(..)
                | MetaType::Nullable(..)
                | MetaType::Placeholder(..)
                | MetaType::Scalar(..)
                | MetaType::Union(..) => None,
            },
            Self::List(..) | Self::NonNull(..) => None,
        }
    }

    fn interfaces<'s>(&self, context: &'s SchemaType<S>) -> Option<Vec<TypeType<'s, S>>> {
        match self {
            Self::Concrete(t) => match t {
                MetaType::Interface(InterfaceMeta {
                    interface_names, ..
                })
                | MetaType::Object(ObjectMeta {
                    interface_names, ..
                }) => Some(
                    interface_names
                        .iter()
                        .filter_map(|n| context.type_by_name(n))
                        .collect(),
                ),
                MetaType::Enum(..)
                | MetaType::InputObject(..)
                | MetaType::List(..)
                | MetaType::Nullable(..)
                | MetaType::Placeholder(..)
                | MetaType::Scalar(..)
                | MetaType::Union(..) => None,
            },
            Self::List(..) | Self::NonNull(..) => None,
        }
    }

    fn possible_types<'s>(&self, context: &'s SchemaType<S>) -> Option<Vec<TypeType<'s, S>>> {
        match self {
            Self::Concrete(t) => match t {
                MetaType::Interface(InterfaceMeta {
                    name: iface_name, ..
                }) => {
                    let mut type_names = context
                        .types
                        .values()
                        .filter_map(|ct| {
                            if let MetaType::Object(ObjectMeta {
                                name,
                                interface_names,
                                ..
                            }) = ct
                            {
                                interface_names
                                    .iter()
                                    .any(|iname| iname == iface_name)
                                    .then_some(name)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    type_names.sort();
                    Some(
                        type_names
                            .into_iter()
                            .filter_map(|n| context.type_by_name(n))
                            .collect(),
                    )
                }
                MetaType::Union(UnionMeta { of_type_names, .. }) => Some(
                    of_type_names
                        .iter()
                        .filter_map(|tn| context.type_by_name(tn))
                        .collect(),
                ),
                MetaType::Enum(..)
                | MetaType::InputObject(..)
                | MetaType::List(..)
                | MetaType::Nullable(..)
                | MetaType::Object(..)
                | MetaType::Placeholder(..)
                | MetaType::Scalar(..) => None,
            },
            Self::List(..) | Self::NonNull(..) => None,
        }
    }

    fn enum_values(&self, #[graphql(default)] include_deprecated: bool) -> Option<Vec<&EnumValue>> {
        match self {
            Self::Concrete(t) => match t {
                MetaType::Enum(EnumMeta { values, .. }) => Some(
                    values
                        .iter()
                        .filter(|f| include_deprecated || !f.deprecation_status.is_deprecated())
                        .collect(),
                ),
                MetaType::InputObject(..)
                | MetaType::Interface(..)
                | MetaType::List(..)
                | MetaType::Nullable(..)
                | MetaType::Object(..)
                | MetaType::Placeholder(..)
                | MetaType::Scalar(..)
                | MetaType::Union(..) => None,
            },
            Self::List(..) | Self::NonNull(..) => None,
        }
    }

    fn input_fields(
        &self,
        #[graphql(default)] include_deprecated: bool,
    ) -> Option<Vec<&Argument<S>>> {
        match self {
            Self::Concrete(t) => match t {
                MetaType::InputObject(InputObjectMeta { input_fields, .. }) => Some(
                    input_fields
                        .iter()
                        .filter(|f| include_deprecated || !f.deprecation_status.is_deprecated())
                        .collect(),
                ),
                MetaType::Enum(..)
                | MetaType::Interface(..)
                | MetaType::List(..)
                | MetaType::Nullable(..)
                | MetaType::Object(..)
                | MetaType::Placeholder(..)
                | MetaType::Scalar(..)
                | MetaType::Union(..) => None,
            },
            Self::List(..) | Self::NonNull(..) => None,
        }
    }

    fn of_type(&self) -> Option<&Self> {
        match self {
            Self::Concrete(..) => None,
            Self::List(l, _) | Self::NonNull(l) => Some(&**l),
        }
    }

    fn is_one_of(&self) -> Option<bool> {
        match self {
            Self::Concrete(t) => match t {
                MetaType::InputObject(InputObjectMeta { is_one_of, .. }) => Some(*is_one_of),
                MetaType::Enum(..)
                | MetaType::Interface(..)
                | MetaType::List(..)
                | MetaType::Nullable(..)
                | MetaType::Object(..)
                | MetaType::Placeholder(..)
                | MetaType::Scalar(..)
                | MetaType::Union(..) => None,
            },
            Self::List(..) | Self::NonNull(..) => None,
        }
    }
}

#[graphql_object]
#[graphql(
    name = "__Field",
    context = SchemaType<S>,
    scalar = S,
    internal,
)]
impl<S: ScalarValue> Field<S> {
    fn name(&self) -> &ArcStr {
        &self.name
    }

    #[graphql(name = "description")]
    fn description_(&self) -> Option<&ArcStr> {
        self.description.as_ref()
    }

    fn args(&self, #[graphql(default)] include_deprecated: bool) -> Vec<&Argument<S>> {
        self.arguments.as_ref().map_or_else(Vec::new, |args| {
            args.iter()
                .filter(|a| include_deprecated || !a.deprecation_status.is_deprecated())
                .collect()
        })
    }

    #[graphql(name = "type")]
    fn type_<'s>(&self, context: &'s SchemaType<S>) -> TypeType<'s, S> {
        context.make_type(&self.field_type)
    }

    fn is_deprecated(&self) -> bool {
        self.deprecation_status.is_deprecated()
    }

    fn deprecation_reason(&self) -> Option<&ArcStr> {
        self.deprecation_status.reason()
    }
}

#[graphql_object]
#[graphql(
    name = "__InputValue",
    context = SchemaType<S>,
    scalar = S,
    internal,
)]
impl<S: ScalarValue> Argument<S> {
    fn name(&self) -> &ArcStr {
        &self.name
    }

    #[graphql(name = "description")]
    fn description_(&self) -> Option<&ArcStr> {
        self.description.as_ref()
    }

    #[graphql(name = "type")]
    fn type_<'s>(&self, context: &'s SchemaType<S>) -> TypeType<'s, S> {
        context.make_type(&self.arg_type)
    }

    #[graphql(name = "defaultValue")]
    fn default_value_(&self) -> Option<String> {
        self.default_value.as_ref().map(ToString::to_string)
    }

    fn is_deprecated(&self) -> bool {
        self.deprecation_status.is_deprecated()
    }

    fn deprecation_reason(&self) -> Option<&ArcStr> {
        self.deprecation_status.reason()
    }
}

#[graphql_object]
#[graphql(name = "__EnumValue", internal)]
impl EnumValue {
    fn name(&self) -> &ArcStr {
        &self.name
    }

    #[graphql(name = "description")]
    fn description_(&self) -> Option<&ArcStr> {
        self.description.as_ref()
    }

    fn is_deprecated(&self) -> bool {
        self.deprecation_status.is_deprecated()
    }

    fn deprecation_reason(&self) -> Option<&ArcStr> {
        self.deprecation_status.reason()
    }
}

#[graphql_object]
#[graphql(
    name = "__Directive",
    context = SchemaType<S>,
    scalar = S,
    internal,
)]
impl<S: ScalarValue> DirectiveType<S> {
    fn name(&self) -> &ArcStr {
        &self.name
    }

    #[graphql(name = "description")]
    fn description_(&self) -> Option<&ArcStr> {
        self.description.as_ref()
    }

    fn is_repeatable(&self) -> bool {
        self.is_repeatable
    }

    fn locations(&self) -> &[DirectiveLocation] {
        &self.locations
    }

    fn args(&self, #[graphql(default)] include_deprecated: bool) -> Vec<&Argument<S>> {
        self.arguments
            .iter()
            .filter(|a| include_deprecated || !a.deprecation_status.is_deprecated())
            .collect()
    }

    // Included for compatibility with the introspection query in GraphQL.js.
    #[graphql(deprecated = "Use `__Directive.locations` instead.")]
    fn on_operation(&self) -> bool {
        self.locations.contains(&DirectiveLocation::Query)
    }

    // Included for compatibility with the introspection query in GraphQL.js.
    #[graphql(deprecated = "Use `__Directive.locations` instead.")]
    fn on_fragment(&self) -> bool {
        self.locations
            .contains(&DirectiveLocation::FragmentDefinition)
            || self.locations.contains(&DirectiveLocation::InlineFragment)
            || self.locations.contains(&DirectiveLocation::FragmentSpread)
    }

    // Included for compatibility with the introspection query in GraphQL.js.
    #[graphql(deprecated = "Use `__Directive.locations` instead.")]
    fn on_field(&self) -> bool {
        self.locations.contains(&DirectiveLocation::Field)
    }
}
