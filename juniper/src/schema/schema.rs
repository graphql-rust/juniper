use crate::{
    ast::Selection,
    executor::{ExecutionResult, Executor, Registry},
    types::{
        async_await::{GraphQLTypeAsync, GraphQLValueAsync},
        base::{Arguments, GraphQLType, GraphQLValue, TypeKind},
    },
    value::Value,
};

use crate::schema::{
    meta::{
        Argument, EnumMeta, EnumValue, Field, InputObjectMeta, InterfaceMeta, MetaType, ObjectMeta,
        UnionMeta,
    },
    model::{DirectiveLocation, DirectiveType, RootNode, SchemaType, TypeType},
};

impl<'a, QueryT, MutationT, SubscriptionT> GraphQLType
    for RootNode<'a, QueryT, MutationT, SubscriptionT>
where
    QueryT: GraphQLType,
    MutationT: GraphQLType<Context = QueryT::Context>,
    SubscriptionT: GraphQLType<Context = QueryT::Context>,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        QueryT::name(info)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r> {
        QueryT::meta(info, registry)
    }
}

impl<'a, QueryT, MutationT, SubscriptionT> GraphQLValue
    for RootNode<'a, QueryT, MutationT, SubscriptionT>
where
    QueryT: GraphQLType,
    MutationT: GraphQLType<Context = QueryT::Context>,
    SubscriptionT: GraphQLType<Context = QueryT::Context>,
{
    type Context = QueryT::Context;
    type TypeInfo = QueryT::TypeInfo;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        QueryT::name(info)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        args: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        match field {
            "__schema" => executor
                .replaced_context(&self.schema)
                .resolve(&(), &self.schema),
            "__type" => {
                let type_name: String = args.get("name").unwrap();
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
        selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
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

impl<'a, QueryT, MutationT, SubscriptionT> GraphQLValueAsync
    for RootNode<'a, QueryT, MutationT, SubscriptionT>
where
    QueryT: GraphQLTypeAsync,
    QueryT::TypeInfo: Sync,
    QueryT::Context: Sync + 'a,
    MutationT: GraphQLTypeAsync<Context = QueryT::Context>,
    MutationT::TypeInfo: Sync,
    SubscriptionT: GraphQLType<Context = QueryT::Context> + Sync,
    SubscriptionT::TypeInfo: Sync,
{
    fn resolve_field_async<'b>(
        &'b self,
        info: &'b Self::TypeInfo,
        field_name: &'b str,
        arguments: &'b Arguments,
        executor: &'b Executor<Self::Context>,
    ) -> crate::BoxFuture<'b, ExecutionResult> {
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

#[crate::graphql_object(
    name = "__Schema"
    Context = SchemaType<'a, >,
    internal,
    // FIXME: make this redundant.
    noasync,
)]
impl<'a> SchemaType<'a> {
    fn types(&self) -> Vec<TypeType> {
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
            .collect::<Vec<_>>()
    }

    fn query_type(&self) -> TypeType {
        self.query_type()
    }

    fn mutation_type(&self) -> Option<TypeType> {
        self.mutation_type()
    }

    fn subscription_type(&self) -> Option<TypeType> {
        self.subscription_type()
    }

    fn directives(&self) -> Vec<&DirectiveType> {
        self.directive_list()
    }
}

#[crate::graphql_object(
    name = "__Type"
    Context = SchemaType<'a>,
    internal,
    // FIXME: make this redundant.
    noasync,
)]
impl<'a> TypeType<'a> {
    fn name(&self) -> Option<&str> {
        match *self {
            TypeType::Concrete(t) => t.name(),
            _ => None,
        }
    }

    fn description(&self) -> Option<&String> {
        match *self {
            TypeType::Concrete(t) => t.description(),
            _ => None,
        }
    }

    fn kind(&self) -> TypeKind {
        match *self {
            TypeType::Concrete(t) => t.type_kind(),
            TypeType::List(_) => TypeKind::List,
            TypeType::NonNull(_) => TypeKind::NonNull,
        }
    }

    #[graphql(arguments(include_deprecated(default = false)))]
    fn fields(&self, include_deprecated: bool) -> Option<Vec<&Field>> {
        match *self {
            TypeType::Concrete(&MetaType::Interface(InterfaceMeta { ref fields, .. }))
            | TypeType::Concrete(&MetaType::Object(ObjectMeta { ref fields, .. })) => Some(
                fields
                    .iter()
                    .filter(|f| include_deprecated || !f.deprecation_status.is_deprecated())
                    .filter(|f| !f.name.starts_with("__"))
                    .collect(),
            ),
            _ => None,
        }
    }

    fn of_type(&self) -> Option<&Box<TypeType>> {
        match *self {
            TypeType::Concrete(_) => None,
            TypeType::List(ref l) | TypeType::NonNull(ref l) => Some(l),
        }
    }

    fn input_fields(&self) -> Option<&Vec<Argument>> {
        match *self {
            TypeType::Concrete(&MetaType::InputObject(InputObjectMeta {
                ref input_fields,
                ..
            })) => Some(input_fields),
            _ => None,
        }
    }

    fn interfaces(&self, schema: &SchemaType<'a>) -> Option<Vec<TypeType>> {
        match *self {
            TypeType::Concrete(&MetaType::Object(ObjectMeta {
                ref interface_names,
                ..
            })) => Some(
                interface_names
                    .iter()
                    .filter_map(|n| schema.type_by_name(n))
                    .collect(),
            ),
            _ => None,
        }
    }

    fn possible_types(&self, schema: &SchemaType<'a>) -> Option<Vec<TypeType>> {
        match *self {
            TypeType::Concrete(&MetaType::Union(UnionMeta {
                ref of_type_names, ..
            })) => Some(
                of_type_names
                    .iter()
                    .filter_map(|tn| schema.type_by_name(tn))
                    .collect(),
            ),
            TypeType::Concrete(&MetaType::Interface(InterfaceMeta {
                name: ref iface_name,
                ..
            })) => Some(
                schema
                    .concrete_type_list()
                    .iter()
                    .filter_map(|&ct| {
                        if let MetaType::Object(ObjectMeta {
                            ref name,
                            ref interface_names,
                            ..
                        }) = *ct
                        {
                            if interface_names.contains(&iface_name.to_string()) {
                                schema.type_by_name(name)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect(),
            ),
            _ => None,
        }
    }

    #[graphql(arguments(include_deprecated(default = false)))]
    fn enum_values(&self, include_deprecated: bool) -> Option<Vec<&EnumValue>> {
        match *self {
            TypeType::Concrete(&MetaType::Enum(EnumMeta { ref values, .. })) => Some(
                values
                    .iter()
                    .filter(|f| include_deprecated || !f.deprecation_status.is_deprecated())
                    .collect(),
            ),
            _ => None,
        }
    }
}

#[crate::graphql_object(
    name = "__Field",
    Context = SchemaType<'a>,
    internal,
    // FIXME: make this redundant.
    noasync,
)]
impl<'a> Field<'a> {
    fn name(&self) -> &String {
        &self.name
    }

    fn description(&self) -> &Option<String> {
        &self.description
    }

    fn args(&self) -> Vec<&Argument> {
        self.arguments
            .as_ref()
            .map_or_else(Vec::new, |v| v.iter().collect())
    }

    #[graphql(name = "type")]
    fn _type(&self, context: &SchemaType<'a>) -> TypeType {
        context.make_type(&self.field_type)
    }

    fn is_deprecated(&self) -> bool {
        self.deprecation_status.is_deprecated()
    }

    fn deprecation_reason(&self) -> Option<&String> {
        self.deprecation_status.reason()
    }
}

#[crate::graphql_object(
    name = "__InputValue",
    Context = SchemaType<'a>,
    internal,
    // FIXME: make this redundant.
    noasync,
)]
impl<'a> Argument<'a> {
    fn name(&self) -> &String {
        &self.name
    }

    fn description(&self) -> &Option<String> {
        &self.description
    }

    #[graphql(name = "type")]
    fn _type(&self, context: &SchemaType<'a>) -> TypeType {
        context.make_type(&self.arg_type)
    }

    fn default_value(&self) -> Option<String> {
        self.default_value.as_ref().map(|v| format!("{}", v))
    }
}

#[crate::graphql_object(
    name = "__EnumValue",
    internal,
    // FIXME: make this redundant.
    noasync,
)]
impl<'a> EnumValue {
    fn name(&self) -> &String {
        &self.name
    }

    fn description(&self) -> &Option<String> {
        &self.description
    }

    fn is_deprecated(&self) -> bool {
        self.deprecation_status.is_deprecated()
    }

    fn deprecation_reason(&self) -> Option<&String> {
        self.deprecation_status.reason()
    }
}

#[crate::graphql_object(
    name = "__Directive",
    Context = SchemaType<'a, >,
    internal,
    // FIXME: make this redundant.
    noasync,
)]
impl<'a> DirectiveType<'a> {
    fn name(&self) -> &String {
        &self.name
    }

    fn description(&self) -> &Option<String> {
        &self.description
    }

    fn locations(&self) -> &Vec<DirectiveLocation> {
        &self.locations
    }

    fn args(&self) -> &Vec<Argument> {
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
