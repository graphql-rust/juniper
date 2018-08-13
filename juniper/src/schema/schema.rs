use executor::{ExecutionResult, Executor, Registry};
use types::base::{Arguments, GraphQLType, TypeKind};
use value::Value;
use ast::Selection;

use schema::meta::{
    Argument, EnumMeta, EnumValue, Field, InputObjectMeta, InterfaceMeta, MetaType, ObjectMeta,
    UnionMeta,
};
use schema::model::{DirectiveLocation, DirectiveType, RootNode, SchemaType, TypeType};

impl<'a, CtxT, QueryT, MutationT> GraphQLType for RootNode<'a, QueryT, MutationT>
where
    QueryT: GraphQLType<Context = CtxT>,
    MutationT: GraphQLType<Context = CtxT>,
{
    type Context = CtxT;
    type TypeInfo = QueryT::TypeInfo;

    fn name(info: &QueryT::TypeInfo) -> Option<&str> {
        QueryT::name(info)
    }

    fn meta<'r>(info: &QueryT::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r> {
        QueryT::meta(info, registry)
    }

    fn resolve_field(
        &self,
        info: &QueryT::TypeInfo,
        field: &str,
        args: &Arguments,
        executor: &Executor<CtxT>,
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
    ) -> Value {
        use value::Object;
        use types::base::resolve_selection_set_into;
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

graphql_object!(<'a> SchemaType<'a>: SchemaType<'a> as "__Schema" |&self| {
    field types() -> Vec<TypeType> {
        self.type_list()
            .into_iter()
            .filter(|t| t.to_concrete().map(|t| t.name() != Some("_EmptyMutation")).unwrap_or(false))
            .collect()
    }

    field query_type() -> TypeType {
        self.query_type()
    }

    field mutation_type() -> Option<TypeType> {
        self.mutation_type()
    }

    // Included for compatibility with the introspection query in GraphQL.js
    field subscription_type() -> Option<TypeType> {
        None
    }

    field directives() -> Vec<&DirectiveType> {
        self.directive_list()
    }
});

graphql_object!(<'a> TypeType<'a>: SchemaType<'a> as "__Type" |&self| {
    field name() -> Option<&str> {
        match *self {
            TypeType::Concrete(t) => t.name(),
            _ => None,
        }
    }

    field description() -> Option<&String> {
        match *self {
            TypeType::Concrete(t) => t.description(),
            _ => None,
        }
    }

    field kind() -> TypeKind {
        match *self {
            TypeType::Concrete(t) => t.type_kind(),
            TypeType::List(_) => TypeKind::List,
            TypeType::NonNull(_) => TypeKind::NonNull,
        }
    }

    field fields(include_deprecated = false: bool) -> Option<Vec<&Field>> {
        match *self {
            TypeType::Concrete(&MetaType::Interface(InterfaceMeta { ref fields, .. })) |
            TypeType::Concrete(&MetaType::Object(ObjectMeta { ref fields, .. })) =>
                Some(fields
                    .iter()
                    .filter(|f| include_deprecated || f.deprecation_reason.is_none())
                    .filter(|f| !f.name.starts_with("__"))
                    .collect()),
            _ => None,
        }
    }

    field of_type() -> Option<&Box<TypeType>> {
        match *self {
            TypeType::Concrete(_) => None,
            TypeType::List(ref l) | TypeType::NonNull(ref l) => Some(l),
        }
    }

    field input_fields() -> Option<&Vec<Argument>> {
        match *self {
            TypeType::Concrete(&MetaType::InputObject(InputObjectMeta { ref input_fields, .. })) =>
                Some(input_fields),
            _ => None,
        }
    }

    field interfaces(&executor) -> Option<Vec<TypeType>> {
        match *self {
            TypeType::Concrete(&MetaType::Object(ObjectMeta { ref interface_names, .. })) => {
                let schema = executor.context();
                Some(interface_names
                    .iter()
                    .filter_map(|n| schema.type_by_name(n))
                    .collect())
            }
            _ => None,
        }
    }

    field possible_types(&executor) -> Option<Vec<TypeType>> {
        let schema = executor.context();
        match *self {
            TypeType::Concrete(&MetaType::Union(UnionMeta { ref of_type_names, .. })) => {
                Some(of_type_names
                    .iter()
                    .filter_map(|tn| schema.type_by_name(tn))
                    .collect())
            }
            TypeType::Concrete(&MetaType::Interface(InterfaceMeta{name: ref iface_name, .. })) => {
                Some(schema.concrete_type_list()
                    .iter()
                    .filter_map(|&ct|
                        if let MetaType::Object(ObjectMeta{
                            ref name,
                            ref interface_names,
                            ..
                        }) = *ct {
                            if interface_names.contains(&iface_name.to_string()) {
                                schema.type_by_name(name)
                            } else { None }
                        } else { None }
                    )
                    .collect())
            }
            _ => None,
        }
    }

    field enum_values(include_deprecated = false: bool) -> Option<Vec<&EnumValue>> {
        match *self {
            TypeType::Concrete(&MetaType::Enum(EnumMeta { ref values, .. })) =>
                Some(values
                    .iter()
                    .filter(|f| include_deprecated || f.deprecation_reason.is_none())
                    .collect()),
            _ => None,
        }
    }
});

graphql_object!(<'a> Field<'a>: SchemaType<'a> as "__Field" |&self| {
    field name() -> &String {
        &self.name
    }

    field description() -> &Option<String> {
        &self.description
    }

    field args() -> Vec<&Argument> {
        self.arguments.as_ref().map_or_else(Vec::new, |v| v.iter().collect())
    }

    field type(&executor) -> TypeType {
        executor.context().make_type(&self.field_type)
    }

    field is_deprecated() -> bool {
        self.deprecation_reason.is_some()
    }

    field deprecation_reason() -> &Option<String> {
        &self.deprecation_reason
    }
});

graphql_object!(<'a> Argument<'a>: SchemaType<'a> as "__InputValue" |&self| {
    field name() -> &String {
        &self.name
    }

    field description() -> &Option<String> {
        &self.description
    }

    field type(&executor) -> TypeType {
        executor.context().make_type(&self.arg_type)
    }

    field default_value() -> Option<String> {
        self.default_value.as_ref().map(|v| format!("{}", v))
    }
});

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

graphql_object!(<'a> DirectiveType<'a>: SchemaType<'a> as "__Directive" |&self| {
    field name() -> &String {
        &self.name
    }

    field description() -> &Option<String> {
        &self.description
    }

    field locations() -> &Vec<DirectiveLocation> {
        &self.locations
    }

    field args() -> &Vec<Argument> {
        &self.arguments
    }

    // Included for compatibility with the introspection query in GraphQL.js
    field deprecated "Use the locations array instead"
    on_operation() -> bool {
        self.locations.contains(&DirectiveLocation::Query)
    }

    // Included for compatibility with the introspection query in GraphQL.js
    field deprecated "Use the locations array instead"
    on_fragment() -> bool {
        self.locations.contains(&DirectiveLocation::FragmentDefinition) ||
            self.locations.contains(&DirectiveLocation::InlineFragment) ||
            self.locations.contains(&DirectiveLocation::FragmentSpread)
    }

    // Included for compatibility with the introspection query in GraphQL.js
    field deprecated "Use the locations array instead"
    on_field() -> bool {
        self.locations.contains(&DirectiveLocation::Field)
    }
});
