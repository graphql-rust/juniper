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

impl<'a, S, QueryT, MutationT, SubscriptionT> GraphQLType<S>
    for RootNode<'a, QueryT, MutationT, SubscriptionT, S>
where
    S: ScalarValue,
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S, Context = QueryT::Context>,
    SubscriptionT: GraphQLType<S, Context = QueryT::Context>,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        QueryT::name(info)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        QueryT::meta(info, registry)
    }
}

impl<'a, S, QueryT, MutationT, SubscriptionT> GraphQLValue<S>
    for RootNode<'a, QueryT, MutationT, SubscriptionT, S>
where
    S: ScalarValue,
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S, Context = QueryT::Context>,
    SubscriptionT: GraphQLType<S, Context = QueryT::Context>,
{
    type Context = QueryT::Context;
    type TypeInfo = QueryT::TypeInfo;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
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

impl<'a, S, QueryT, MutationT, SubscriptionT> GraphQLValueAsync<S>
    for RootNode<'a, QueryT, MutationT, SubscriptionT, S>
where
    QueryT: GraphQLTypeAsync<S>,
    QueryT::TypeInfo: Sync,
    QueryT::Context: Sync + 'a,
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
        use futures::future::ready;
        match field_name {
            "__schema" | "__type" => {
                let v = self.resolve_field(info, field_name, arguments, executor);
                Box::pin(ready(v))
            }
            _ => self
                .query_type
                .resolve_field_async(info, field_name, arguments, executor),
        }
    }
}

#[graphql_object(
    name = "__Schema"
    context = SchemaType<'a, S>,
    scalar = S,
    internal,
)]
impl<'a, S: ScalarValue + 'a> SchemaType<'a, S> {
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn types(&self) -> Vec<TypeType<S>> {
        self.type_list()
            .into_iter()
            .filter(|t| {
                t.to_concrete()
                    .map(|t| {
                        !(t.name() == Some("_EmptyMutation")
                            || t.name() == Some("_EmptySubscription"))
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    #[graphql(name = "queryType")]
    fn query_type_(&self) -> TypeType<S> {
        self.query_type()
    }

    #[graphql(name = "mutationType")]
    fn mutation_type_(&self) -> Option<TypeType<S>> {
        self.mutation_type()
    }

    #[graphql(name = "subscriptionType")]
    fn subscription_type_(&self) -> Option<TypeType<S>> {
        self.subscription_type()
    }

    fn directives(&self) -> Vec<&DirectiveType<S>> {
        self.directive_list()
    }
}

#[graphql_object(
    name = "__Type"
    context = SchemaType<'a, S>,
    scalar = S,
    internal,
)]
impl<'a, S: ScalarValue + 'a> TypeType<'a, S> {
    fn name(&self) -> Option<&str> {
        match self {
            TypeType::Concrete(t) => t.name(),
            _ => None,
        }
    }

    fn description(&self) -> Option<&str> {
        match self {
            TypeType::Concrete(t) => t.description(),
            _ => None,
        }
    }

    fn specified_by_url(&self) -> Option<&str> {
        match self {
            Self::Concrete(t) => t.specified_by_url(),
            Self::NonNull(_) | Self::List(..) => None,
        }
    }

    fn kind(&self) -> TypeKind {
        match self {
            TypeType::Concrete(t) => t.type_kind(),
            TypeType::List(..) => TypeKind::List,
            TypeType::NonNull(_) => TypeKind::NonNull,
        }
    }

    fn fields(
        &self,
        #[graphql(default = false)] include_deprecated: Option<bool>,
    ) -> Option<Vec<&Field<S>>> {
        match self {
            TypeType::Concrete(&MetaType::Interface(InterfaceMeta { ref fields, .. }))
            | TypeType::Concrete(&MetaType::Object(ObjectMeta { ref fields, .. })) => Some(
                fields
                    .iter()
                    .filter(|f| {
                        include_deprecated.unwrap_or_default()
                            || !f.deprecation_status.is_deprecated()
                    })
                    .filter(|f| !f.name.starts_with("__"))
                    .collect(),
            ),
            _ => None,
        }
    }

    fn of_type(&self) -> Option<&TypeType<S>> {
        match self {
            TypeType::Concrete(_) => None,
            TypeType::List(l, _) | TypeType::NonNull(l) => Some(&**l),
        }
    }

    fn input_fields(&self) -> Option<&[Argument<S>]> {
        match self {
            TypeType::Concrete(&MetaType::InputObject(InputObjectMeta {
                ref input_fields,
                ..
            })) => Some(input_fields.as_slice()),
            _ => None,
        }
    }

    fn interfaces<'s>(&self, context: &'s SchemaType<'a, S>) -> Option<Vec<TypeType<'s, S>>> {
        match self {
            TypeType::Concrete(
                &MetaType::Object(ObjectMeta {
                    ref interface_names,
                    ..
                })
                | &MetaType::Interface(InterfaceMeta {
                    ref interface_names,
                    ..
                }),
            ) => Some(
                interface_names
                    .iter()
                    .filter_map(|n| context.type_by_name(n))
                    .collect(),
            ),
            _ => None,
        }
    }

    fn possible_types<'s>(&self, context: &'s SchemaType<'a, S>) -> Option<Vec<TypeType<'s, S>>> {
        match self {
            TypeType::Concrete(&MetaType::Union(UnionMeta {
                ref of_type_names, ..
            })) => Some(
                of_type_names
                    .iter()
                    .filter_map(|tn| context.type_by_name(tn))
                    .collect(),
            ),
            TypeType::Concrete(&MetaType::Interface(InterfaceMeta {
                name: ref iface_name,
                ..
            })) => {
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
                                .then(|| name.as_ref())
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
            _ => None,
        }
    }

    fn enum_values(
        &self,
        #[graphql(default = false)] include_deprecated: Option<bool>,
    ) -> Option<Vec<&EnumValue>> {
        match self {
            TypeType::Concrete(&MetaType::Enum(EnumMeta { ref values, .. })) => Some(
                values
                    .iter()
                    .filter(|f| {
                        include_deprecated.unwrap_or_default()
                            || !f.deprecation_status.is_deprecated()
                    })
                    .collect(),
            ),
            _ => None,
        }
    }
}

#[graphql_object(
    name = "__Field",
    context = SchemaType<'a, S>,
    scalar = S,
    internal,
)]
impl<'a, S: ScalarValue + 'a> Field<'a, S> {
    fn name(&self) -> String {
        self.name.clone().into()
    }

    #[graphql(name = "description")]
    fn description_(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn args(&self) -> Vec<&Argument<S>> {
        self.arguments
            .as_ref()
            .map_or_else(Vec::new, |v| v.iter().collect())
    }

    #[graphql(name = "type")]
    fn type_<'s>(&self, context: &'s SchemaType<'a, S>) -> TypeType<'s, S> {
        context.make_type(&self.field_type)
    }

    fn is_deprecated(&self) -> bool {
        self.deprecation_status.is_deprecated()
    }

    fn deprecation_reason(&self) -> Option<&str> {
        self.deprecation_status.reason()
    }
}

#[graphql_object(
    name = "__InputValue",
    context = SchemaType<'a, S>,
    scalar = S,
    internal,
)]
impl<'a, S: ScalarValue + 'a> Argument<'a, S> {
    fn name(&self) -> &str {
        &self.name
    }

    #[graphql(name = "description")]
    fn description_(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[graphql(name = "type")]
    fn type_<'s>(&self, context: &'s SchemaType<'a, S>) -> TypeType<'s, S> {
        context.make_type(&self.arg_type)
    }

    #[graphql(name = "defaultValue")]
    fn default_value_(&self) -> Option<String> {
        self.default_value.as_ref().map(ToString::to_string)
    }
}

#[graphql_object(name = "__EnumValue", internal)]
impl EnumValue {
    fn name(&self) -> &str {
        &self.name
    }

    #[graphql(name = "description")]
    fn description_(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn is_deprecated(&self) -> bool {
        self.deprecation_status.is_deprecated()
    }

    fn deprecation_reason(&self) -> Option<&str> {
        self.deprecation_status.reason()
    }
}

#[graphql_object(
    name = "__Directive",
    context = SchemaType<'a, S>,
    scalar = S,
    internal,
)]
impl<'a, S: ScalarValue + 'a> DirectiveType<'a, S> {
    fn name(&self) -> &str {
        &self.name
    }

    #[graphql(name = "description")]
    fn description_(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn locations(&self) -> &[DirectiveLocation] {
        &self.locations
    }

    fn is_repeatable(&self) -> bool {
        self.is_repeatable
    }

    fn args(&self) -> &[Argument<S>] {
        &self.arguments
    }

    // Included for compatibility with the introspection query in GraphQL.js
    #[graphql(deprecated = "Use the locations array instead")]
    fn on_operation(&self) -> bool {
        self.locations.contains(&DirectiveLocation::Query)
    }

    // Included for compatibility with the introspection query in GraphQL.js
    #[graphql(deprecated = "Use the locations array instead")]
    fn on_fragment(&self) -> bool {
        self.locations
            .contains(&DirectiveLocation::FragmentDefinition)
            || self.locations.contains(&DirectiveLocation::InlineFragment)
            || self.locations.contains(&DirectiveLocation::FragmentSpread)
    }

    // Included for compatibility with the introspection query in GraphQL.js
    #[graphql(deprecated = "Use the locations array instead")]
    fn on_field(&self) -> bool {
        self.locations.contains(&DirectiveLocation::Field)
    }
}
