use crate::{
    ast::{FromInputValue, InputValue},
    executor::Registry,
    parser::parse_document_source,
    schema::{
        meta::{EnumValue, MetaType},
        model::{DirectiveLocation, DirectiveType, RootNode},
    },
    types::{
        base::{GraphQLType, GraphQLValue},
        scalars::ID,
    },
    validation::{visit, MultiVisitorNil, RuleError, ValidatorContext, Visitor},
    GraphQLInputObject,
};

struct Being;
struct Pet;
struct Canine;

struct Dog;
struct Cat;

struct Intelligent;
struct Human;
struct Alien;

struct DogOrHuman;
struct CatOrDog;
struct HumanOrAlien;

struct ComplicatedArgs;

pub(crate) struct QueryRoot;

#[derive(Debug, GraphQLInputObject)]
struct TestInput {
    id: i32,
    name: String,
}

pub(crate) struct MutationRoot;

pub(crate) struct SubscriptionRoot;

#[derive(Debug)]
enum DogCommand {
    Sit,
    Heel,
    Down,
}

#[derive(Debug)]
enum FurColor {
    Brown,
    Black,
    Tan,
    Spotted,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ComplexInput {
    required_field: bool,
    int_field: Option<i32>,
    string_field: Option<String>,
    boolean_field: Option<bool>,
    string_list_field: Option<Vec<Option<String>>>,
}

impl GraphQLType for Being {
    fn name(_: &()) -> Option<&'static str> {
        Some("Being")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[registry
            .field::<Option<String>>("name", i)
            .argument(registry.arg::<Option<bool>>("surname", i))];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl GraphQLValue for Being {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for Pet {
    fn name(_: &()) -> Option<&'static str> {
        Some("Pet")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[registry
            .field::<Option<String>>("name", i)
            .argument(registry.arg::<Option<bool>>("surname", i))];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl GraphQLValue for Pet {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for Canine {
    fn name(_: &()) -> Option<&'static str> {
        Some("Canine")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[registry
            .field::<Option<String>>("name", i)
            .argument(registry.arg::<Option<bool>>("surname", i))];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl GraphQLValue for Canine {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for DogCommand {
    fn name(_: &()) -> Option<&'static str> {
        Some("DogCommand")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        registry
            .build_enum_type::<Self>(
                i,
                &[
                    EnumValue::new("SIT"),
                    EnumValue::new("HEEL"),
                    EnumValue::new("DOWN"),
                ],
            )
            .into_meta()
    }
}

impl GraphQLValue for DogCommand {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl FromInputValue for DogCommand {
    fn from_input_value<'a>(v: &InputValue) -> Option<DogCommand> {
        match v.as_enum_value() {
            Some("SIT") => Some(DogCommand::Sit),
            Some("HEEL") => Some(DogCommand::Heel),
            Some("DOWN") => Some(DogCommand::Down),
            _ => None,
        }
    }
}

impl GraphQLType for Dog {
    fn name(_: &()) -> Option<&'static str> {
        Some("Dog")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name", i)
                .argument(registry.arg::<Option<bool>>("surname", i)),
            registry.field::<Option<String>>("nickname", i),
            registry.field::<Option<i32>>("barkVolume", i),
            registry.field::<Option<bool>>("barks", i),
            registry
                .field::<Option<bool>>("doesKnowCommand", i)
                .argument(registry.arg::<Option<DogCommand>>("dogCommand", i)),
            registry
                .field::<Option<bool>>("isHousetrained", i)
                .argument(registry.arg_with_default("atOtherHomes", &true, i)),
            registry
                .field::<Option<bool>>("isAtLocation", i)
                .argument(registry.arg::<Option<i32>>("x", i))
                .argument(registry.arg::<Option<i32>>("y", i)),
        ];

        registry
            .build_object_type::<Self>(i, fields)
            .interfaces(&[
                registry.get_type::<Being>(i),
                registry.get_type::<Pet>(i),
                registry.get_type::<Canine>(i),
            ])
            .into_meta()
    }
}

impl GraphQLValue for Dog {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for FurColor {
    fn name(_: &()) -> Option<&'static str> {
        Some("FurColor")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        registry
            .build_enum_type::<Self>(
                i,
                &[
                    EnumValue::new("BROWN"),
                    EnumValue::new("BLACK"),
                    EnumValue::new("TAN"),
                    EnumValue::new("SPOTTED"),
                ],
            )
            .into_meta()
    }
}

impl GraphQLValue for FurColor {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl FromInputValue for FurColor {
    fn from_input_value<'a>(v: &InputValue) -> Option<FurColor> {
        match v.as_enum_value() {
            Some("BROWN") => Some(FurColor::Brown),
            Some("BLACK") => Some(FurColor::Black),
            Some("TAN") => Some(FurColor::Tan),
            Some("SPOTTED") => Some(FurColor::Spotted),
            _ => None,
        }
    }
}

impl GraphQLType for Cat {
    fn name(_: &()) -> Option<&'static str> {
        Some("Cat")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name", i)
                .argument(registry.arg::<Option<bool>>("surname", i)),
            registry.field::<Option<String>>("nickname", i),
            registry.field::<Option<bool>>("meows", i),
            registry.field::<Option<i32>>("meowVolume", i),
            registry.field::<Option<FurColor>>("furColor", i),
        ];

        registry
            .build_object_type::<Self>(i, fields)
            .interfaces(&[registry.get_type::<Being>(i), registry.get_type::<Pet>(i)])
            .into_meta()
    }
}

impl GraphQLValue for Cat {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for CatOrDog {
    fn name(_: &()) -> Option<&'static str> {
        Some("CatOrDog")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let types = &[registry.get_type::<Cat>(i), registry.get_type::<Dog>(i)];

        registry.build_union_type::<Self>(i, types).into_meta()
    }
}

impl GraphQLValue for CatOrDog {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for Intelligent {
    fn name(_: &()) -> Option<&'static str> {
        Some("Intelligent")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[registry.field::<Option<i32>>("iq", i)];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl GraphQLValue for Intelligent {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for Human {
    fn name(_: &()) -> Option<&'static str> {
        Some("Human")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name", i)
                .argument(registry.arg::<Option<bool>>("surname", i)),
            registry.field::<Option<Vec<Option<Pet>>>>("pets", i),
            registry.field::<Option<Vec<Human>>>("relatives", i),
            registry.field::<Option<i32>>("iq", i),
        ];
        registry
            .build_object_type::<Self>(i, fields)
            .interfaces(&[
                registry.get_type::<Being>(i),
                registry.get_type::<Intelligent>(i),
            ])
            .into_meta()
    }
}

impl GraphQLValue for Human {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for Alien {
    fn name(_: &()) -> Option<&'static str> {
        Some("Alien")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name", i)
                .argument(registry.arg::<Option<bool>>("surname", i)),
            registry.field::<Option<i32>>("iq", i),
            registry.field::<Option<i32>>("numEyes", i),
        ];

        registry
            .build_object_type::<Self>(i, fields)
            .interfaces(&[
                registry.get_type::<Being>(i),
                registry.get_type::<Intelligent>(i),
            ])
            .into_meta()
    }
}

impl GraphQLValue for Alien {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for DogOrHuman {
    fn name(_: &()) -> Option<&'static str> {
        Some("DogOrHuman")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let types = &[registry.get_type::<Dog>(i), registry.get_type::<Human>(i)];

        registry.build_union_type::<Self>(i, types).into_meta()
    }
}

impl GraphQLValue for DogOrHuman {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for HumanOrAlien {
    fn name(_: &()) -> Option<&'static str> {
        Some("HumanOrAlien")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let types = &[registry.get_type::<Human>(i), registry.get_type::<Alien>(i)];

        registry.build_union_type::<Self>(i, types).into_meta()
    }
}

impl GraphQLValue for HumanOrAlien {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for ComplexInput {
    fn name(_: &()) -> Option<&'static str> {
        Some("ComplexInput")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry.arg::<bool>("requiredField", i),
            registry.arg::<Option<i32>>("intField", i),
            registry.arg::<Option<String>>("stringField", i),
            registry.arg::<Option<bool>>("booleanField", i),
            registry.arg::<Option<Vec<Option<String>>>>("stringListField", i),
        ];

        registry
            .build_input_object_type::<Self>(i, fields)
            .into_meta()
    }
}

impl GraphQLValue for ComplexInput {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl FromInputValue for ComplexInput {
    fn from_input_value<'a>(v: &InputValue) -> Option<ComplexInput> {
        let obj = match v.to_object_value() {
            Some(o) => o,
            None => return None,
        };

        Some(ComplexInput {
            required_field: match obj.get("requiredField").and_then(|v| v.convert()) {
                Some(f) => f,
                None => return None,
            },
            int_field: obj.get("intField").and_then(|v| v.convert()),
            string_field: obj.get("stringField").and_then(|v| v.convert()),
            boolean_field: obj.get("booleanField").and_then(|v| v.convert()),
            string_list_field: obj.get("stringListField").and_then(|v| v.convert()),
        })
    }
}

impl GraphQLType for ComplicatedArgs {
    fn name(_: &()) -> Option<&'static str> {
        Some("ComplicatedArgs")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("intArgField", i)
                .argument(registry.arg::<Option<i32>>("intArg", i)),
            registry
                .field::<Option<String>>("nonNullIntArgField", i)
                .argument(registry.arg::<i32>("nonNullIntArg", i)),
            registry
                .field::<Option<String>>("stringArgField", i)
                .argument(registry.arg::<Option<String>>("stringArg", i)),
            registry
                .field::<Option<String>>("booleanArgField", i)
                .argument(registry.arg::<Option<bool>>("booleanArg", i)),
            registry
                .field::<Option<String>>("enumArgField", i)
                .argument(registry.arg::<Option<FurColor>>("enumArg", i)),
            registry
                .field::<Option<String>>("floatArgField", i)
                .argument(registry.arg::<Option<f64>>("floatArg", i)),
            registry
                .field::<Option<String>>("idArgField", i)
                .argument(registry.arg::<Option<ID>>("idArg", i)),
            registry
                .field::<Option<String>>("stringListArgField", i)
                .argument(registry.arg::<Option<Vec<Option<String>>>>("stringListArg", i)),
            registry
                .field::<Option<String>>("complexArgField", i)
                .argument(registry.arg::<Option<ComplexInput>>("complexArg", i)),
            registry
                .field::<Option<String>>("multipleReqs", i)
                .argument(registry.arg::<i32>("req1", i))
                .argument(registry.arg::<i32>("req2", i)),
            registry
                .field::<Option<String>>("multipleOpts", i)
                .argument(registry.arg_with_default("opt1", &0i32, i))
                .argument(registry.arg_with_default("opt2", &0i32, i)),
            registry
                .field::<Option<String>>("multipleOptAndReq", i)
                .argument(registry.arg::<i32>("req1", i))
                .argument(registry.arg::<i32>("req2", i))
                .argument(registry.arg_with_default("opt1", &0i32, i))
                .argument(registry.arg_with_default("opt2", &0i32, i)),
        ];

        registry.build_object_type::<Self>(i, fields).into_meta()
    }
}

impl GraphQLValue for ComplicatedArgs {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for QueryRoot {
    fn name(_: &()) -> Option<&'static str> {
        Some("QueryRoot")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<Human>>("human", i)
                .argument(registry.arg::<Option<ID>>("id", i)),
            registry.field::<Option<Alien>>("alien", i),
            registry.field::<Option<Dog>>("dog", i),
            registry.field::<Option<Cat>>("cat", i),
            registry.field::<Option<Pet>>("pet", i),
            registry.field::<Option<CatOrDog>>("catOrDog", i),
            registry.field::<Option<DogOrHuman>>("dorOrHuman", i),
            registry.field::<Option<HumanOrAlien>>("humanOrAlien", i),
            registry.field::<Option<ComplicatedArgs>>("complicatedArgs", i),
        ];

        registry.build_object_type::<Self>(i, fields).into_meta()
    }
}

impl GraphQLValue for QueryRoot {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for MutationRoot {
    fn name(_: &()) -> Option<&str> {
        Some("MutationRoot")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = [registry.field::<i32>("testInput", i).argument(
            registry.arg_with_default::<TestInput>(
                "input",
                &TestInput {
                    id: 423,
                    name: String::from("foo"),
                },
                i,
            ),
        )];

        registry.build_object_type::<Self>(i, &fields).into_meta()
    }
}

impl GraphQLValue for MutationRoot {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl GraphQLType for SubscriptionRoot {
    fn name(_: &()) -> Option<&str> {
        Some("SubscriptionRoot")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = [];

        registry.build_object_type::<Self>(i, &fields).into_meta()
    }
}

impl GraphQLValue for SubscriptionRoot {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

pub fn validate<'a, Q, M, Sub, V, F>(r: Q, m: M, s: Sub, q: &'a str, factory: F) -> Vec<RuleError>
where
    Q: GraphQLType<TypeInfo = ()>,
    M: GraphQLType<TypeInfo = ()>,
    Sub: GraphQLType<TypeInfo = ()>,
    V: Visitor<'a> + 'a,
    F: Fn() -> V,
{
    let mut root = RootNode::new(r, m, s);

    root.schema.add_directive(DirectiveType::new(
        "onQuery",
        &[DirectiveLocation::Query],
        &[],
    ));
    root.schema.add_directive(DirectiveType::new(
        "onMutation",
        &[DirectiveLocation::Mutation],
        &[],
    ));
    root.schema.add_directive(DirectiveType::new(
        "onField",
        &[DirectiveLocation::Field],
        &[],
    ));
    root.schema.add_directive(DirectiveType::new(
        "onFragmentDefinition",
        &[DirectiveLocation::FragmentDefinition],
        &[],
    ));
    root.schema.add_directive(DirectiveType::new(
        "onFragmentSpread",
        &[DirectiveLocation::FragmentSpread],
        &[],
    ));
    root.schema.add_directive(DirectiveType::new(
        "onInlineFragment",
        &[DirectiveLocation::InlineFragment],
        &[],
    ));

    let doc =
        parse_document_source(q, &root.schema).expect(&format!("Parse error on input {:#?}", q));
    let mut ctx = ValidatorContext::new(unsafe { ::std::mem::transmute(&root.schema) }, &doc);

    let mut mv = MultiVisitorNil.with(factory());
    visit(&mut mv, &mut ctx, unsafe { ::std::mem::transmute(&doc) });

    ctx.into_errors()
}

pub fn expect_passes_rule<'a, V, F>(factory: F, q: &'a str)
where
    V: Visitor<'a> + 'a,
    F: Fn() -> V,
{
    expect_passes_rule_with_schema(QueryRoot, MutationRoot, SubscriptionRoot, factory, q);
}

pub fn expect_passes_rule_with_schema<'a, Q, M, Sub, V, F>(
    r: Q,
    m: M,
    s: Sub,
    factory: F,
    q: &'a str,
) where
    Q: GraphQLType<TypeInfo = ()>,
    M: GraphQLType<TypeInfo = ()>,
    Sub: GraphQLType<TypeInfo = ()>,
    V: Visitor<'a> + 'a,
    F: Fn() -> V,
{
    let errs = validate(r, m, s, q, factory);

    if !errs.is_empty() {
        print_errors(&errs);
        panic!("Expected rule to pass, but errors found");
    }
}

pub fn expect_fails_rule<'a, V, F>(factory: F, q: &'a str, expected_errors: &[RuleError])
where
    V: Visitor<'a> + 'a,
    F: Fn() -> V,
{
    expect_fails_rule_with_schema(QueryRoot, MutationRoot, factory, q, expected_errors);
}

pub fn expect_fails_rule_with_schema<'a, Q, M, V, F>(
    r: Q,
    m: M,
    factory: F,
    q: &'a str,
    expected_errors: &[RuleError],
) where
    Q: GraphQLType<TypeInfo = ()>,
    M: GraphQLType<TypeInfo = ()>,
    V: Visitor<'a> + 'a,
    F: Fn() -> V,
{
    let errs = validate::<Q, M, crate::EmptySubscription, V, F>(
        r,
        m,
        crate::EmptySubscription::new(),
        q,
        factory,
    );

    if errs.is_empty() {
        panic!("Expected rule to fail, but no errors were found");
    } else if errs != expected_errors {
        println!("==> Expected errors:");
        print_errors(expected_errors);

        println!("\n==> Actual errors:");
        print_errors(&errs);

        panic!("Unexpected set of errors found");
    }
}

fn print_errors(errs: &[RuleError]) {
    for err in errs {
        for p in err.locations() {
            print!("[{:>3},{:>3},{:>3}]  ", p.index(), p.line(), p.column());
        }
        println!("{}", err.message());
    }
}
