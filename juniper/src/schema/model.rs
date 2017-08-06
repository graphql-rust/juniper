use std::collections::HashMap;
use std::fmt;

use types::base::GraphQLType;
use executor::{Context, Registry};
use ast::Type;
use schema::meta::{Argument, InterfaceMeta, MetaType, ObjectMeta, PlaceholderMeta, UnionMeta};

/// Root query node of a schema
///
/// This brings the mutation and query types together, and provides the
/// predefined metadata fields.
pub struct RootNode<'a, QueryT, MutationT> {
    #[doc(hidden)]
    pub query_type: QueryT,
    #[doc(hidden)]
    pub mutation_type: MutationT,
    #[doc(hidden)]
    pub schema: SchemaType<'a>,
}

/// Metadata for a schema
pub struct SchemaType<'a> {
    types: HashMap<String, MetaType<'a>>,
    query_type_name: String,
    mutation_type_name: Option<String>,
    directives: HashMap<String, DirectiveType<'a>>,
}

impl<'a> Context for SchemaType<'a> {}

pub enum TypeType<'a> {
    Concrete(&'a MetaType<'a>),
    NonNull(Box<TypeType<'a>>),
    List(Box<TypeType<'a>>),
}

pub struct DirectiveType<'a> {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<DirectiveLocation>,
    pub arguments: Vec<Argument<'a>>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DirectiveLocation {
    Query,
    Mutation,
    Field,
    FragmentDefinition,
    FragmentSpread,
    InlineFragment,
}

impl<'a, QueryT, MutationT> RootNode<'a, QueryT, MutationT>
where
    QueryT: GraphQLType,
    MutationT: GraphQLType,
{
    /// Construct a new root node from query and mutation nodes
    ///
    /// If the schema should not support mutations, use the
    /// `new` constructor instead.
    pub fn new(query_obj: QueryT, mutation_obj: MutationT) -> RootNode<'a, QueryT, MutationT> {
        RootNode {
            query_type: query_obj,
            mutation_type: mutation_obj,
            schema: SchemaType::new::<QueryT, MutationT>(),
        }
    }
}

impl<'a> SchemaType<'a> {
    pub fn new<QueryT, MutationT>() -> SchemaType<'a>
    where
        QueryT: GraphQLType,
        MutationT: GraphQLType,
    {
        let mut directives = HashMap::new();
        let query_type_name: String;
        let mutation_type_name: String;

        let mut registry = Registry::new(HashMap::new());
        query_type_name = registry.get_type::<QueryT>().innermost_name().to_owned();
        mutation_type_name = registry.get_type::<MutationT>().innermost_name().to_owned();

        registry.get_type::<SchemaType>();
        directives.insert("skip".to_owned(), DirectiveType::new_skip(&mut registry));
        directives.insert(
            "include".to_owned(),
            DirectiveType::new_include(&mut registry),
        );

        let mut meta_fields = vec![
            registry.field::<SchemaType>("__schema"),
            registry
                .field::<TypeType>("__type")
                .argument(registry.arg::<String>("name")),
        ];

        if let Some(root_type) = registry.types.get_mut(&query_type_name) {
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
                panic!("Type {:?} is still a placeholder type", of_type);
            }
        }

        SchemaType {
            types: registry.types,
            query_type_name: query_type_name,
            mutation_type_name: if &mutation_type_name != "_EmptyMutation" {
                Some(mutation_type_name)
            } else {
                None
            },
            directives: directives,
        }
    }

    pub fn add_directive(&mut self, directive: DirectiveType<'a>) {
        self.directives.insert(directive.name.clone(), directive);
    }

    pub fn type_by_name(&self, name: &str) -> Option<TypeType> {
        self.types.get(name).map(|t| TypeType::Concrete(t))
    }

    pub fn concrete_type_by_name(&self, name: &str) -> Option<&MetaType> {
        self.types.get(name)
    }

    pub fn query_type(&self) -> TypeType {
        TypeType::Concrete(
            self.types
                .get(&self.query_type_name)
                .expect("Query type does not exist in schema"),
        )
    }

    pub fn concrete_query_type(&self) -> &MetaType {
        self.types
            .get(&self.query_type_name)
            .expect("Query type does not exist in schema")
    }

    pub fn mutation_type(&self) -> Option<TypeType> {
        if let Some(ref mutation_type_name) = self.mutation_type_name {
            Some(
                self.type_by_name(mutation_type_name)
                    .expect("Mutation type does not exist in schema"),
            )
        } else {
            None
        }
    }

    pub fn concrete_mutation_type(&self) -> Option<&MetaType> {
        self.mutation_type_name.as_ref().map(|name| {
            self.concrete_type_by_name(name)
                .expect("Mutation type does not exist in schema")
        })
    }

    pub fn type_list(&self) -> Vec<TypeType> {
        self.types.values().map(|t| TypeType::Concrete(t)).collect()
    }

    pub fn concrete_type_list(&self) -> Vec<&MetaType> {
        self.types.values().collect()
    }

    pub fn make_type(&self, t: &Type) -> TypeType {
        match *t {
            Type::NonNullNamed(n) => TypeType::NonNull(Box::new(
                self.type_by_name(n).expect("Type not found in schema"),
            )),
            Type::NonNullList(ref inner) => {
                TypeType::NonNull(Box::new(TypeType::List(Box::new(self.make_type(inner)))))
            }
            Type::Named(n) => self.type_by_name(n).expect("Type not found in schema"),
            Type::List(ref inner) => TypeType::List(Box::new(self.make_type(inner))),
        }
    }

    pub fn directive_list(&self) -> Vec<&DirectiveType> {
        self.directives.values().collect()
    }

    pub fn directive_by_name(&self, name: &str) -> Option<&DirectiveType> {
        self.directives.get(name)
    }

    pub fn type_overlap(&self, t1: &MetaType, t2: &MetaType) -> bool {
        if (t1 as *const MetaType) == (t2 as *const MetaType) {
            return true;
        }

        match (t1.is_abstract(), t2.is_abstract()) {
            (true, true) => self.possible_types(t1)
                .iter()
                .any(|t| self.is_possible_type(t2, t)),
            (true, false) => self.is_possible_type(t1, t2),
            (false, true) => self.is_possible_type(t2, t1),
            (false, false) => false,
        }
    }

    pub fn possible_types(&self, t: &MetaType) -> Vec<&MetaType> {
        match *t {
            MetaType::Union(UnionMeta {
                ref of_type_names, ..
            }) => of_type_names
                .iter()
                .flat_map(|t| self.concrete_type_by_name(t))
                .collect(),
            MetaType::Interface(InterfaceMeta { name, .. }) => self.concrete_type_list()
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

    pub fn is_possible_type(&self, abstract_type: &MetaType, possible_type: &MetaType) -> bool {
        self.possible_types(abstract_type).into_iter().any(|t| {
            (t as *const MetaType) == (possible_type as *const MetaType)
        })
    }

    pub fn is_subtype<'b>(&self, sub_type: &Type<'b>, super_type: &Type<'b>) -> bool {
        use ast::Type::*;

        if super_type == sub_type {
            return true;
        }

        match (super_type, sub_type) {
            (&NonNullNamed(super_name), &NonNullNamed(sub_name)) |
            (&Named(super_name), &Named(sub_name)) |
            (&Named(super_name), &NonNullNamed(sub_name)) => {
                self.is_named_subtype(sub_name, super_name)
            }
            (&NonNullList(ref super_inner), &NonNullList(ref sub_inner)) |
            (&List(ref super_inner), &List(ref sub_inner)) |
            (&List(ref super_inner), &NonNullList(ref sub_inner)) => {
                self.is_subtype(sub_inner, super_inner)
            }
            _ => false,
        }
    }

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

impl<'a> TypeType<'a> {
    pub fn to_concrete(&self) -> Option<&'a MetaType> {
        match *self {
            TypeType::Concrete(t) => Some(t),
            _ => None,
        }
    }
}

impl<'a> DirectiveType<'a> {
    pub fn new(
        name: &str,
        locations: &[DirectiveLocation],
        arguments: &[Argument<'a>],
    ) -> DirectiveType<'a> {
        DirectiveType {
            name: name.to_owned(),
            description: None,
            locations: locations.to_vec(),
            arguments: arguments.to_vec(),
        }
    }

    fn new_skip(registry: &mut Registry<'a>) -> DirectiveType<'a> {
        Self::new(
            "skip",
            &[
                DirectiveLocation::Field,
                DirectiveLocation::FragmentSpread,
                DirectiveLocation::InlineFragment,
            ],
            &[registry.arg::<bool>("if")],
        )
    }

    fn new_include(registry: &mut Registry<'a>) -> DirectiveType<'a> {
        Self::new(
            "include",
            &[
                DirectiveLocation::Field,
                DirectiveLocation::FragmentSpread,
                DirectiveLocation::InlineFragment,
            ],
            &[registry.arg::<bool>("if")],
        )
    }

    pub fn description(mut self, description: &str) -> DirectiveType<'a> {
        self.description = Some(description.to_owned());
        self
    }
}

impl fmt::Display for DirectiveLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            DirectiveLocation::Query => "query",
            DirectiveLocation::Mutation => "mutation",
            DirectiveLocation::Field => "field",
            DirectiveLocation::FragmentDefinition => "fragment definition",
            DirectiveLocation::FragmentSpread => "fragment spread",
            DirectiveLocation::InlineFragment => "inline fragment",
        })
    }
}

impl<'a> fmt::Display for TypeType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TypeType::Concrete(t) => f.write_str(t.name().unwrap()),
            TypeType::List(ref i) => write!(f, "[{}]", i),
            TypeType::NonNull(ref i) => write!(f, "{}!", i),
        }
    }
}
