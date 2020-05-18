use crate::{
    ast::Selection,
    executor::{ExecutionResult, Executor, Registry},
    schema::{
        meta::{
            Argument, EnumMeta, EnumValue, Field, InputObjectMeta, InterfaceMeta, MetaType,
            ObjectMeta, UnionMeta,
        },
        model::{DirectiveLocation, DirectiveType, RootNode, SchemaType, TypeType},
    },
    types::base::{Arguments, GraphQLType, TypeKind},
    value::ScalarValue,
    BoxFuture,
};

impl<'a, CtxT, S, QueryT, MutationT, SubscriptionT> GraphQLType<S>
    for RootNode<'a, QueryT, MutationT, SubscriptionT, S>
where
    S: ScalarValue,
    QueryT: crate::GraphQLType<S, Context = CtxT> + Send + Sync,
    QueryT::TypeInfo: Send + Sync,
    MutationT: crate::GraphQLType<S, Context = CtxT> + Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    SubscriptionT: GraphQLType<S, Context = CtxT> + Send + Sync,
    SubscriptionT::TypeInfo: Send + Sync,
    CtxT: Send + Sync + 'a,
{
    type Context = CtxT;
    type TypeInfo = QueryT::TypeInfo;

    fn name(info: &QueryT::TypeInfo) -> Option<&str> {
        QueryT::name(info)
    }

    fn meta<'r>(info: &QueryT::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        QueryT::meta(info, registry)
    }

    fn resolve_field<'me, 'ty, 'field, 'args, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        field_name: &'field str,
        args: &'args Arguments<'args, S>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'args: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
        'field: 'fut,
        S: 'fut,
    {
        let f = async move {
            match field_name {
                "__schema" => {
                    executor
                        .replaced_context(&self.schema)
                        .resolve(&(), &self.schema)
                        .await
                }
                "__type" => {
                    let type_name: String = args.get("name").unwrap();
                    executor
                        .replaced_context(&self.schema)
                        .resolve(&(), &self.schema.type_by_name(&type_name))
                        .await
                }
                _ => {
                    self.query_type
                        .resolve_field(info, field_name, args, executor)
                        .await
                }
            }
        };
        futures::future::FutureExt::boxed(f)
    }

    fn resolve<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        let f = async move {
            use crate::types::base::resolve_selection_set_into;
            if let Some(selection_set) = selection_set {
                Ok(resolve_selection_set_into(self, info, selection_set, executor).await)
            } else {
                // TODO: this panic seems useless, investigate why it is here.
                panic!("resolve() must be implemented by non-object output types");
            }
        };
        futures::future::FutureExt::boxed(f)
    }
}

#[crate::graphql_object_internal(
    name = "__Schema"
    Context = SchemaType<'a, S>,
    Scalar = S,
)]
impl<'a, S> SchemaType<'a, S>
where
    S: crate::ScalarValue + 'a,
{
    async fn types(&self) -> Vec<TypeType<'_, S>> {
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

    async fn query_type(&self) -> TypeType<'_, S> {
        self.query_type()
    }

    async fn mutation_type(&self) -> Option<TypeType<'_, S>> {
        self.mutation_type()
    }

    async fn subscription_type(&self) -> Option<TypeType<'_, S>> {
        self.subscription_type()
    }

    async fn directives(&self) -> Vec<&DirectiveType<'_, S>> {
        self.directive_list()
    }
}

#[crate::graphql_object_internal(
    name = "__Type"
    Context = SchemaType<'a, S>,
    Scalar = S,
)]
impl<'a, S> TypeType<'a, S>
where
    S: crate::ScalarValue + 'a,
{
    async fn name(&self) -> Option<&str> {
        match *self {
            TypeType::Concrete(t) => t.name(),
            _ => None,
        }
    }

    async fn description(&self) -> Option<&String> {
        match *self {
            TypeType::Concrete(t) => t.description(),
            _ => None,
        }
    }

    async fn kind(&self) -> TypeKind {
        match *self {
            TypeType::Concrete(t) => t.type_kind(),
            TypeType::List(_) => TypeKind::List,
            TypeType::NonNull(_) => TypeKind::NonNull,
        }
    }

    #[graphql(arguments(include_deprecated(default = false)))]
    async fn fields(&self, include_deprecated: bool) -> Option<Vec<&Field<'a, S>>> {
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

    async fn of_type(&self) -> Option<&Box<TypeType<S>>> {
        match *self {
            TypeType::Concrete(_) => None,
            TypeType::List(ref l) | TypeType::NonNull(ref l) => Some(l),
        }
    }

    async fn input_fields(&self) -> Option<&Vec<Argument<'a, S>>> {
        match *self {
            TypeType::Concrete(&MetaType::InputObject(InputObjectMeta {
                ref input_fields,
                ..
            })) => Some(input_fields),
            _ => None,
        }
    }

    async fn interfaces(&self, schema: &SchemaType<'a, S>) -> Option<Vec<TypeType<S>>> {
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

    async fn possible_types(&self, schema: &SchemaType<'a, S>) -> Option<Vec<TypeType<S>>> {
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
    async fn enum_values(&self, include_deprecated: bool) -> Option<Vec<&EnumValue>> {
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

#[crate::graphql_object_internal(
    name = "__Field",
    Context = SchemaType<'a, S>,
    Scalar = S,
)]
impl<'a, S> Field<'a, S>
where
    S: crate::ScalarValue + 'a,
{
    async fn name(&self) -> &String {
        &self.name
    }

    async fn description(&self) -> &Option<String> {
        &self.description
    }

    async fn args(&self) -> Vec<&Argument<S>> {
        self.arguments
            .as_ref()
            .map_or_else(Vec::new, |v| v.iter().collect())
    }

    #[graphql(name = "type")]
    async fn _type(&self, context: &SchemaType<'a, S>) -> TypeType<S> {
        context.make_type(&self.field_type)
    }

    async fn is_deprecated(&self) -> bool {
        self.deprecation_status.is_deprecated()
    }

    async fn deprecation_reason(&self) -> Option<&String> {
        self.deprecation_status.reason()
    }
}

#[crate::graphql_object_internal(
    name = "__InputValue",
    Context = SchemaType<'a, S>,
    Scalar = S,
)]
impl<'a, S> Argument<'a, S>
where
    S: crate::ScalarValue + 'a,
{
    async fn name(&self) -> &String {
        &self.name
    }

    async fn description(&self) -> &Option<String> {
        &self.description
    }

    #[graphql(name = "type")]
    async fn _type(&self, context: &SchemaType<'a, S>) -> TypeType<S> {
        context.make_type(&self.arg_type)
    }

    async fn default_value(&self) -> Option<String> {
        self.default_value.as_ref().map(|v| format!("{}", v))
    }
}

#[crate::graphql_object_internal(
    name = "__EnumValue",
    Scalar = S,
)]
impl<'a, S> EnumValue
where
    S: crate::ScalarValue + 'a,
{
    async fn name(&self) -> &String {
        &self.name
    }

    async fn description(&self) -> &Option<String> {
        &self.description
    }

    async fn is_deprecated(&self) -> bool {
        self.deprecation_status.is_deprecated()
    }

    async fn deprecation_reason(&self) -> Option<&String> {
        self.deprecation_status.reason()
    }
}

#[crate::graphql_object_internal(
    name = "__Directive",
    Context = SchemaType<'a, S>,
    Scalar = S,
)]
impl<'a, S> DirectiveType<'a, S>
where
    S: crate::ScalarValue + 'a,
{
    async fn name(&self) -> &String {
        &self.name
    }

    async fn description(&self) -> &Option<String> {
        &self.description
    }

    async fn locations(&self) -> &Vec<DirectiveLocation> {
        &self.locations
    }

    async fn args(&self) -> &Vec<Argument<'_, S>> {
        &self.arguments
    }

    // Included for compatibility with the introspection query in GraphQL.js
    #[graphql(deprecated = "Use the locations array instead")]
    async fn on_operation(&self) -> bool {
        self.locations.contains(&DirectiveLocation::Query)
    }

    // Included for compatibility with the introspection query in GraphQL.js
    #[graphql(deprecated = "Use the locations array instead")]
    async fn on_fragment(&self) -> bool {
        self.locations
            .contains(&DirectiveLocation::FragmentDefinition)
            || self.locations.contains(&DirectiveLocation::InlineFragment)
            || self.locations.contains(&DirectiveLocation::FragmentSpread)
    }

    // Included for compatibility with the introspection query in GraphQL.js
    #[graphql(deprecated = "Use the locations array instead")]
    async fn on_field(&self) -> bool {
        self.locations.contains(&DirectiveLocation::Field)
    }
}
