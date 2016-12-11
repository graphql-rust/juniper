use rustc_serialize::json::ToJson;

use types::base::{GraphQLType, Arguments, TypeKind};
use executor::{Executor, Registry, ExecutionResult};

use schema::meta::{MetaType, ObjectMeta, EnumMeta, InputObjectMeta, UnionMeta, InterfaceMeta,
                   Field, Argument, EnumValue};
use schema::model::{RootNode, SchemaType, TypeType, DirectiveType, DirectiveLocation};

impl<CtxT, QueryT, MutationT> GraphQLType<CtxT> for RootNode<CtxT, QueryT, MutationT>
    where QueryT: GraphQLType<CtxT>,
          MutationT: GraphQLType<CtxT>
{
    fn name() -> Option<&'static str> {
        QueryT::name()
    }

    fn meta(registry: &mut Registry<CtxT>) -> MetaType {
        QueryT::meta(registry)
    }

    fn resolve_field(&self, field: &str, args: &Arguments, executor: &Executor<CtxT>) -> ExecutionResult {
        match field {
            "__schema" => executor.replaced_context(&self.schema).resolve(&self.schema),
            "__type" => {
                let type_name: String = args.get("name").unwrap();
                executor.replaced_context(&self.schema).resolve(&self.schema.type_by_name(&type_name))
            },
            _=> self.query_type.resolve_field(field, args, executor),
        }
    }
}

graphql_object!(SchemaType: SchemaType as "__Schema" |&self| {
    field types() -> Vec<TypeType> {
        self.type_list()
    }

    field query_type() -> TypeType {
        self.query_type()
    }

    field mutation_type() -> Option<TypeType> {
        self.mutation_type()
    }

    field directives() -> Vec<&DirectiveType> {
        self.directive_list()
    }
});

graphql_object!(<'a> TypeType<'a>: SchemaType as "__Type" |&self| {
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
            TypeType::Concrete(&MetaType::Interface(InterfaceMeta { name: ref iface_name, .. })) => {
                Some(schema.concrete_type_list()
                    .iter()
                    .filter_map(|&ct|
                        if let &MetaType::Object(ObjectMeta { ref name, ref interface_names, .. }) = ct {
                            if interface_names.contains(iface_name) {
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

graphql_object!(Field: SchemaType as "__Field" |&self| {
    field name() -> &String {
        &self.name
    }

    field description() -> &Option<String> {
        &self.description
    }

    field args() -> Vec<&Argument> {
        self.arguments.as_ref().map_or_else(|| Vec::new(), |v| v.iter().collect())
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

graphql_object!(Argument: SchemaType as "__InputValue" |&self| {
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
        self.default_value.as_ref().map(|v| v.to_json().to_string())
    }
});

graphql_object!(EnumValue: SchemaType as "__EnumValue" |&self| {
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

graphql_enum!(TypeKind as "__TypeKind" {
    TypeKind::Scalar => "SCALAR",
    TypeKind::Object => "OBJECT",
    TypeKind::Interface => "INTERFACE",
    TypeKind::Union => "UNION",
    TypeKind::Enum => "ENUM",
    TypeKind::InputObject => "INPUT_OBJECT",
    TypeKind::List => "LIST",
    TypeKind::NonNull => "NON_NULL",
});


graphql_object!(DirectiveType: SchemaType as "__Directive" |&self| {
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
});

graphql_enum!(DirectiveLocation as "__DirectiveLocation" {
    DirectiveLocation::Query => "QUERY",
    DirectiveLocation::Mutation => "MUTATION",
    DirectiveLocation::Field => "FIELD",
    DirectiveLocation::FragmentDefinition => "FRAGMENT_DEFINITION",
    DirectiveLocation::FragmentSpread => "FRAGMENT_SPREAD",
    DirectiveLocation::InlineFragment => "INLINE_FRAGMENT",
});
