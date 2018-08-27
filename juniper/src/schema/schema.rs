use ast::Selection;
use executor::{ExecutionResult, Executor, Registry};
use types::base::{Arguments, GraphQLType, TypeKind};
use value::{ScalarRefValue, ScalarValue, Value};

use schema::meta::{
    Argument, EnumMeta, EnumValue, Field, InputObjectMeta, InterfaceMeta, MetaType, ObjectMeta,
    UnionMeta,
};
use schema::model::{DirectiveLocation, DirectiveType, RootNode, SchemaType, TypeType};

impl<'a, CtxT, S, QueryT, MutationT> GraphQLType<S> for RootNode<'a, S, QueryT, MutationT>
where
    S: ScalarValue,
    QueryT: GraphQLType<S, Context = CtxT>,
    MutationT: GraphQLType<S, Context = CtxT>,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = CtxT;
    type TypeInfo = QueryT::TypeInfo;

    fn name(info: &QueryT::TypeInfo) -> Option<&str> {
        QueryT::name(info)
    }

    fn meta<'r>(info: &QueryT::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        QueryT::meta(info, registry)
    }

    fn resolve_field(
        &self,
        info: &QueryT::TypeInfo,
        field: &str,
        args: &Arguments<S>,
        executor: &Executor<S, CtxT>,
    ) -> ExecutionResult<S> {
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
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<S, Self::Context>,
    ) -> Value<S> {
        use types::base::resolve_selection_set_into;
        use value::Object;
        if let Some(selection_set) = selection_set {
            let mut result = Object::with_capacity(selection_set.len());
            if resolve_selection_set_into(self, info, selection_set, executor, &mut result) {
                Value::Object(result)
            } else {
                Value::null()
            }
        } else {
            panic!("resolve() must be implemented by non-object output types");
        }
    }
}

impl<'a, S> GraphQLType<S> for SchemaType<'a, S>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = Self;
    type TypeInfo = ();

    fn name((): &Self::TypeInfo) -> Option<&str> {
        Some("__Schema")
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        let types = registry.field::<Vec<TypeType<S>>>("types", info);
        let query_type = registry.field::<TypeType<S>>("queryType", info);
        let mutation_type = registry.field::<Option<TypeType<S>>>("mutationType", info);
        let subscription_type = registry.field::<Option<TypeType<S>>>("subscriptionType", info);
        let directives = registry.field::<Vec<&DirectiveType<S>>>("directives", info);

        let obj = registry.build_object_type::<Self>(
            info,
            &[
                types,
                query_type,
                mutation_type,
                subscription_type,
                directives,
            ],
        );
        obj.into_meta()
    }

    fn concrete_type_name(&self, _: &Self::Context, (): &Self::TypeInfo) -> String {
        String::from("__Schema")
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        _args: &Arguments<S>,
        executor: &Executor<S, Self::Context>,
    ) -> ExecutionResult<S> {
        match field {
            "types" => {
                let r = self
                    .type_list()
                    .into_iter()
                    .filter(|t| {
                        t.to_concrete()
                            .map(|t| t.name() != Some("_EmptyMutation"))
                            .unwrap_or(false)
                    }).collect::<Vec<_>>();
                executor.resolve(info, &r)
            }
            "queryType" => executor.resolve(info, &self.query_type()),
            "mutationType" => executor.resolve(info, &self.mutation_type()),
            "subscriptionType" => executor.resolve::<Option<TypeType<S>>>(info, &None),
            "directives" => executor.resolve(info, &self.directive_list()),
            e => panic!("Field {} not found on type __Schema", e),
        }
    }
}

impl<'a, S> GraphQLType<S> for TypeType<'a, S>
where
    S: ScalarValue + 'a,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = SchemaType<'a, S>;
    type TypeInfo = ();

    fn name((): &Self::TypeInfo) -> Option<&str> {
        Some("__Type")
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        let name = registry.field::<Option<&str>>("name", info);
        let description = registry.field::<Option<&String>>("description", info);
        let kind = registry.field::<TypeKind>("kind", info);
        let fields = registry
            .field::<Option<Vec<&Field<S>>>>("fields", info)
            .argument(registry.arg_with_default("includeDeprecated", &false, info));
        let of_type = registry.field::<Option<&Box<TypeType<S>>>>("ofType", info);
        let input_fields = registry.field::<Option<&Vec<Argument<S>>>>("inputFields", info);
        let interfaces = registry.field::<Option<Vec<TypeType<S>>>>("interfaces", info);
        let possible_types = registry.field::<Option<Vec<TypeType<S>>>>("possibleTypes", info);
        let enum_values = registry
            .field::<Option<Vec<&EnumValue>>>("enumValues", info)
            .argument(registry.arg_with_default("includeDeprecated", &false, info));

        let obj = registry.build_object_type::<Self>(
            info,
            &[
                name,
                description,
                kind,
                fields,
                of_type,
                input_fields,
                interfaces,
                possible_types,
                enum_values,
            ],
        );
        obj.into_meta()
    }

    fn concrete_type_name(&self, _: &Self::Context, (): &Self::TypeInfo) -> String {
        String::from("__Type")
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        args: &Arguments<S>,
        executor: &Executor<S, Self::Context>,
    ) -> ExecutionResult<S> {
        match field {
            "name" => {
                let r = match *self {
                    TypeType::Concrete(t) => t.name(),
                    _ => None,
                };
                executor.replaced_context(&()).resolve(info, &r)
            }
            "description" => {
                let r = match *self {
                    TypeType::Concrete(t) => t.description(),
                    _ => None,
                };
                executor.replaced_context(&()).resolve(info, &r)
            }
            "kind" => {
                let r = match *self {
                    TypeType::Concrete(t) => t.type_kind(),
                    TypeType::List(_) => TypeKind::List,
                    TypeType::NonNull(_) => TypeKind::NonNull,
                };
                executor.replaced_context(&()).resolve(info, &r)
            }
            "fields" => {
                let include_deprecated = args.get("includeDeprecated").unwrap_or(false);
                let r: Option<Vec<_>> = match *self {
                    TypeType::Concrete(&MetaType::Interface(InterfaceMeta {
                        ref fields, ..
                    }))
                    | TypeType::Concrete(&MetaType::Object(ObjectMeta { ref fields, .. })) => Some(
                        fields
                            .iter()
                            .filter(|f| include_deprecated || f.deprecation_reason.is_none())
                            .filter(|f| !f.name.starts_with("__"))
                            .collect(),
                    ),
                    _ => None,
                };
                executor.resolve(info, &r)
            }
            "ofType" => {
                let r = match *self {
                    TypeType::Concrete(_) => None,
                    TypeType::List(ref l) | TypeType::NonNull(ref l) => Some(l),
                };
                executor.resolve(info, &r)
            }
            "inputFields" => {
                let r = match *self {
                    TypeType::Concrete(&MetaType::InputObject(InputObjectMeta {
                        ref input_fields,
                        ..
                    })) => Some(input_fields),
                    _ => None,
                };
                executor.resolve(info, &r)
            }
            "interfaces" => {
                let r: Option<Vec<_>> = match *self {
                    TypeType::Concrete(&MetaType::Object(ObjectMeta {
                        ref interface_names,
                        ..
                    })) => {
                        let schema = executor.context();
                        Some(
                            interface_names
                                .iter()
                                .filter_map(|n| schema.type_by_name(n))
                                .collect(),
                        )
                    }
                    _ => None,
                };
                executor.resolve(info, &r)
            }
            "possibleTypes" => {
                let schema = executor.context();
                let r: Option<Vec<_>> = match *self {
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
                            }).collect(),
                    ),
                    _ => None,
                };
                executor.resolve(info, &r)
            }
            "enumValues" => {
                let include_deprecated = args.get("includeDeprecated").unwrap_or(false);
                let r: Option<Vec<_>> = match *self {
                    TypeType::Concrete(&MetaType::Enum(EnumMeta { ref values, .. })) => Some(
                        values
                            .iter()
                            .filter(|f| include_deprecated || f.deprecation_reason.is_none())
                            .collect(),
                    ),
                    _ => None,
                };
                executor.replaced_context(&()).resolve(info, &r)
            }
            e => panic!("Field {} not found on type __Type", e),
        }
    }
}

impl<'a, S> GraphQLType<S> for Field<'a, S>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = SchemaType<'a, S>;
    type TypeInfo = ();

    fn name((): &Self::TypeInfo) -> Option<&str> {
        Some("__Field")
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        let name = registry.field::<Option<&str>>("name", info);
        let description = registry.field::<Option<&String>>("description", info);
        let args = registry.field::<Vec<&Argument<S>>>("args", info);
        let tpe = registry.field::<TypeType<S>>("type", info);
        let is_deprecated = registry.field::<bool>("isDeprecated", info);
        let deprecation_reason = registry.field::<Option<&String>>("deprecationReason", info);

        let obj = registry.build_object_type::<Self>(
            info,
            &[
                name,
                description,
                args,
                tpe,
                is_deprecated,
                deprecation_reason,
            ],
        );
        obj.into_meta()
    }

    fn concrete_type_name(&self, _: &Self::Context, (): &Self::TypeInfo) -> String {
        String::from("__Field")
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        _args: &Arguments<S>,
        executor: &Executor<S, Self::Context>,
    ) -> ExecutionResult<S> {
        match field {
            "name" => executor.replaced_context(&()).resolve(info, &self.name),
            "description" => executor
                .replaced_context(&())
                .resolve(&(), &self.description),
            "args" => executor.resolve(
                info,
                &self
                    .arguments
                    .as_ref()
                    .map_or_else(Vec::new, |v| v.iter().collect()),
            ),
            "type" => executor.resolve(info, &executor.context().make_type(&self.field_type)),
            "isDeprecated" => executor
                .replaced_context(&())
                .resolve(info, &self.deprecation_reason.is_some()),
            "deprecationReason" => executor
                .replaced_context(&())
                .resolve(info, &&self.deprecation_reason),
            e => panic!("Field {} not found on type __Type", e),
        }
    }
}

impl<'a, S> GraphQLType<S> for Argument<'a, S>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = SchemaType<'a, S>;
    type TypeInfo = ();

    fn name((): &Self::TypeInfo) -> Option<&str> {
        Some("__InputValue")
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        let name = registry.field::<Option<&str>>("name", info);
        let description = registry.field::<Option<&String>>("description", info);
        let tpe = registry.field::<TypeType<S>>("type", info);
        let default_value = registry.field::<Option<String>>("defaultValue", info);

        let obj =
            registry.build_object_type::<Self>(info, &[name, description, tpe, default_value]);
        obj.into_meta()
    }

    fn concrete_type_name(&self, _: &Self::Context, (): &Self::TypeInfo) -> String {
        String::from("__InputValue")
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        _args: &Arguments<S>,
        executor: &Executor<S, Self::Context>,
    ) -> ExecutionResult<S> {
        match field {
            "name" => executor.replaced_context(&()).resolve(info, &self.name),
            "description" => executor
                .replaced_context(&())
                .resolve(info, &self.description),
            "type" => executor.resolve(info, &executor.context().make_type(&self.arg_type)),
            "defaultValue" => executor
                .replaced_context(&())
                .resolve(info, &self.default_value.as_ref().map(|v| format!("{}", v))),
            e => panic!("Field {} not found on type __Type", e),
        }
    }
}

graphql_object!(EnumValue: () as "__EnumValue" |&self| {
    field name() -> &String {
        &self.name
    }

    field description() -> &Option<String> {
        &self.description
    }

    field is_deprecated() -> bool {
        self.deprecation_reason.is_some()
    }

    field deprecation_reason() -> &Option<String> {
        &self.deprecation_reason
    }
});

impl<'a, S> GraphQLType<S> for DirectiveType<'a, S>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = SchemaType<'a, S>;
    type TypeInfo = ();

    fn name((): &Self::TypeInfo) -> Option<&str> {
        Some("__Directive")
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let name = registry.field::<Option<&str>>("name", info);
        let description = registry.field::<Option<&String>>("description", info);
        let locations = registry.field::<&Vec<DirectiveLocation>>("locations", info);
        let args = registry.field::<Vec<&Argument<S>>>("args", info);

        let on_operation = registry
            .field::<bool>("onOperation", info)
            .deprecated("Use the locations array instead");
        let on_fragment = registry
            .field::<bool>("onFragment", info)
            .deprecated("Use the locations array instead");
        let on_field = registry
            .field::<bool>("onField", info)
            .deprecated("Use the locations array instead");

        let obj = registry.build_object_type::<Self>(
            info,
            &[
                name,
                description,
                locations,
                args,
                on_operation,
                on_fragment,
                on_field,
            ],
        );
        obj.into_meta()
    }

    fn concrete_type_name(&self, _: &Self::Context, (): &Self::TypeInfo) -> String {
        String::from("__Directive")
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field: &str,
        _args: &Arguments<S>,
        executor: &Executor<S, Self::Context>,
    ) -> ExecutionResult<S> {
        match field {
            "name" => executor.replaced_context(&()).resolve(info, &self.name),
            "description" => executor
                .replaced_context(&())
                .resolve(info, &self.description),
            "locations" => executor
                .replaced_context(&())
                .resolve(info, &self.locations),
            "args" => executor.resolve(info, &self.arguments),
            "onOperation" => executor
                .replaced_context(&())
                .resolve(info, &self.locations.contains(&DirectiveLocation::Query)),
            "onFragment" => executor.replaced_context(&()).resolve(
                info,
                &(self
                    .locations
                    .contains(&DirectiveLocation::FragmentDefinition)
                    || self.locations.contains(&DirectiveLocation::InlineFragment)
                    || self.locations.contains(&DirectiveLocation::FragmentSpread)),
            ),
            "onField" => executor
                .replaced_context(&())
                .resolve(info, &self.locations.contains(&DirectiveLocation::Field)),
            e => panic!("Field {} not found on type __Type", e),
        }
    }
}
