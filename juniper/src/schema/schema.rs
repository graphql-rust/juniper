use ast::Selection;
use executor::{ExecutionResult, Executor, Registry};
use types::base::{Arguments, GraphQLType, TypeKind};
use value::{ScalarRefValue, ScalarValue, Value};

use schema::meta::{
    Argument, EnumMeta, EnumValue, Field, InputObjectMeta, InterfaceMeta, MetaType, ObjectMeta,
    UnionMeta,
};
use schema::model::{DirectiveLocation, DirectiveType, RootNode, SchemaType, TypeType};

impl<'a, CtxT, S, QueryT, MutationT> GraphQLType<S> for RootNode<'a, QueryT, MutationT, S>
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
        executor: &Executor<CtxT, S>,
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
        executor: &Executor<Self::Context, S>,
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

graphql_object!(<'a> SchemaType<'a, S>: SchemaType<'a, S> as "__Schema"
    where Scalar = <S: 'a> |&self|
{
    field types() -> Vec<TypeType<S>> {
        self.type_list()
            .into_iter()
            .filter(|t| t.to_concrete().map(|t| t.name() != Some("_EmptyMutation")).unwrap_or(false))
            .collect()
    }

    field query_type() -> TypeType<S> {
        self.query_type()
    }

    field mutation_type() -> Option<TypeType<S>> {
        self.mutation_type()
    }

    // Included for compatibility with the introspection query in GraphQL.js
    field subscription_type() -> Option<TypeType<S>> {
        None
    }

    field directives() -> Vec<&DirectiveType<S>> {
        self.directive_list()
    }
});

graphql_object!(<'a> TypeType<'a, S>: SchemaType<'a, S> as "__Type"
    where Scalar = <S: 'a> |&self|
{
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

    field fields(include_deprecated = false: bool) -> Option<Vec<&Field<S>>> {
        match *self {
            TypeType::Concrete(&MetaType::Interface(InterfaceMeta { ref fields, .. })) |
            TypeType::Concrete(&MetaType::Object(ObjectMeta { ref fields, .. })) =>
                Some(fields
                    .iter()
                    .filter(|f| include_deprecated || !f.deprecation_status.is_deprecated())
                    .filter(|f| !f.name.starts_with("__"))
                    .collect()),
            _ => None,
        }
    }

    field of_type() -> Option<&Box<TypeType<S>>> {
        match *self {
            TypeType::Concrete(_) => None,
            TypeType::List(ref l) | TypeType::NonNull(ref l) => Some(l),
        }
    }

    field input_fields() -> Option<&Vec<Argument<S>>> {
        match *self {
            TypeType::Concrete(&MetaType::InputObject(InputObjectMeta { ref input_fields, .. })) =>
                Some(input_fields),
            _ => None,
        }
    }

    field interfaces(&executor) -> Option<Vec<TypeType<S>>> {
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

    field possible_types(&executor) -> Option<Vec<TypeType<S>>> {
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
                    .filter(|f| include_deprecated || !f.deprecation_status.is_deprecated())
                    .collect()),
            _ => None,
        }
    }
});

graphql_object!(<'a> Field<'a, S>: SchemaType<'a, S> as "__Field"
    where Scalar = <S: 'a> |&self|
{
    field name() -> &String {
        &self.name
    }

    field description() -> &Option<String> {
        &self.description
    }

    field args() -> Vec<&Argument<S>> {
        self.arguments.as_ref().map_or_else(Vec::new, |v| v.iter().collect())
    }

    field type(&executor) -> TypeType<S> {
        executor.context().make_type(&self.field_type)
    }

    field is_deprecated() -> bool {
        self.deprecation_status.is_deprecated()
    }

    field deprecation_reason() -> Option<&String> {
        self.deprecation_status.reason()
    }
});

graphql_object!(<'a> Argument<'a, S>: SchemaType<'a, S> as "__InputValue"
    where Scalar = <S: 'a> |&self|
{
    field name() -> &String {
        &self.name
    }

    field description() -> &Option<String> {
        &self.description
    }

    field type(&executor) -> TypeType<S> {
        executor.context().make_type(&self.arg_type)
    }

    field default_value() -> Option<String> {
        self.default_value.as_ref().map(|v| format!("{}", v))
    }
});

graphql_object!(EnumValue: () as "__EnumValue" where Scalar = <S> |&self| {
    field name() -> &String {
        &self.name
    }

    field description() -> &Option<String> {
        &self.description
    }

    field is_deprecated() -> bool {
        self.deprecation_status.is_deprecated()
    }

    field deprecation_reason() -> Option<&String> {
        self.deprecation_status.reason()
    }
});

graphql_object!(<'a> DirectiveType<'a, S>: SchemaType<'a, S> as "__Directive"
    where Scalar = <S: 'a> |&self|
{
   field name() -> &String {
        &self.name
    }

    field description() -> &Option<String> {
        &self.description
    }

    field locations() -> &Vec<DirectiveLocation> {
        &self.locations
    }

    field args() -> &Vec<Argument<S>> {
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
