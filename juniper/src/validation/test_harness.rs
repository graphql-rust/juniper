use parser::parse_document_source;
use ast::{FromInputValue, InputValue};
use types::base::GraphQLType;
use executor::Registry;
use types::scalars::{EmptyMutation, ID};
use schema::model::{DirectiveLocation, DirectiveType, RootNode};
use schema::meta::{EnumValue, MetaType};
use validation::{visit, MultiVisitor, MultiVisitorNil, RuleError, ValidatorContext, Visitor};

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

struct QueryRoot;

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
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("Being")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name")
                .argument(registry.arg::<Option<bool>>("surname")),
        ];

        registry.build_interface_type::<Self>(fields).into_meta()
    }
}

impl GraphQLType for Pet {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("Pet")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name")
                .argument(registry.arg::<Option<bool>>("surname")),
        ];

        registry.build_interface_type::<Self>(fields).into_meta()
    }
}

impl GraphQLType for Canine {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("Canine")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name")
                .argument(registry.arg::<Option<bool>>("surname")),
        ];

        registry.build_interface_type::<Self>(fields).into_meta()
    }
}

impl GraphQLType for DogCommand {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("DogCommand")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        registry
            .build_enum_type::<Self>(&[
                EnumValue::new("SIT"),
                EnumValue::new("HEEL"),
                EnumValue::new("DOWN"),
            ])
            .into_meta()
    }
}

impl FromInputValue for DogCommand {
    fn from(v: &InputValue) -> Option<DogCommand> {
        match v.as_enum_value() {
            Some("SIT") => Some(DogCommand::Sit),
            Some("HEEL") => Some(DogCommand::Heel),
            Some("DOWN") => Some(DogCommand::Down),
            _ => None,
        }
    }
}

impl GraphQLType for Dog {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("Dog")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name")
                .argument(registry.arg::<Option<bool>>("surname")),
            registry.field::<Option<String>>("nickname"),
            registry.field::<Option<i32>>("barkVolume"),
            registry.field::<Option<bool>>("barks"),
            registry
                .field::<Option<bool>>("doesKnowCommand")
                .argument(registry.arg::<Option<DogCommand>>("dogCommand")),
            registry
                .field::<Option<bool>>("isHousetrained")
                .argument(registry.arg_with_default("atOtherHomes", &true)),
            registry
                .field::<Option<bool>>("isAtLocation")
                .argument(registry.arg::<Option<i32>>("x"))
                .argument(registry.arg::<Option<i32>>("y")),
        ];

        registry
            .build_object_type::<Self>(fields)
            .interfaces(&[
                registry.get_type::<Being>(),
                registry.get_type::<Pet>(),
                registry.get_type::<Canine>(),
            ])
            .into_meta()
    }
}

impl GraphQLType for FurColor {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("FurColor")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        registry
            .build_enum_type::<Self>(&[
                EnumValue::new("BROWN"),
                EnumValue::new("BLACK"),
                EnumValue::new("TAN"),
                EnumValue::new("SPOTTED"),
            ])
            .into_meta()
    }
}

impl FromInputValue for FurColor {
    fn from(v: &InputValue) -> Option<FurColor> {
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
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("Cat")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name")
                .argument(registry.arg::<Option<bool>>("surname")),
            registry.field::<Option<String>>("nickname"),
            registry.field::<Option<bool>>("meows"),
            registry.field::<Option<i32>>("meowVolume"),
            registry.field::<Option<FurColor>>("furColor"),
        ];

        registry
            .build_object_type::<Self>(fields)
            .interfaces(&[registry.get_type::<Being>(), registry.get_type::<Pet>()])
            .into_meta()
    }
}

impl GraphQLType for CatOrDog {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("CatOrDog")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let types = &[registry.get_type::<Cat>(), registry.get_type::<Dog>()];

        registry.build_union_type::<Self>(types).into_meta()
    }
}

impl GraphQLType for Intelligent {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("Intelligent")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[registry.field::<Option<i32>>("iq")];

        registry.build_interface_type::<Self>(fields).into_meta()
    }
}

impl GraphQLType for Human {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("Human")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name")
                .argument(registry.arg::<Option<bool>>("surname")),
            registry.field::<Option<Vec<Option<Pet>>>>("pets"),
            registry.field::<Option<Vec<Human>>>("relatives"),
            registry.field::<Option<i32>>("iq"),
        ];
        registry
            .build_object_type::<Self>(fields)
            .interfaces(&[
                registry.get_type::<Being>(),
                registry.get_type::<Intelligent>(),
            ])
            .into_meta()
    }
}

impl GraphQLType for Alien {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("Alien")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("name")
                .argument(registry.arg::<Option<bool>>("surname")),
            registry.field::<Option<i32>>("iq"),
            registry.field::<Option<i32>>("numEyes"),
        ];

        registry
            .build_object_type::<Self>(fields)
            .interfaces(&[
                registry.get_type::<Being>(),
                registry.get_type::<Intelligent>(),
            ])
            .into_meta()
    }
}

impl GraphQLType for DogOrHuman {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("DogOrHuman")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let types = &[registry.get_type::<Dog>(), registry.get_type::<Human>()];

        registry.build_union_type::<Self>(types).into_meta()
    }
}

impl GraphQLType for HumanOrAlien {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("HumanOrAlien")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let types = &[registry.get_type::<Human>(), registry.get_type::<Alien>()];

        registry.build_union_type::<Self>(types).into_meta()
    }
}

impl GraphQLType for ComplexInput {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("ComplexInput")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry.arg::<bool>("requiredField"),
            registry.arg::<Option<i32>>("intField"),
            registry.arg::<Option<String>>("stringField"),
            registry.arg::<Option<bool>>("booleanField"),
            registry.arg::<Option<Vec<Option<String>>>>("stringListField"),
        ];

        registry.build_input_object_type::<Self>(fields).into_meta()
    }
}

impl FromInputValue for ComplexInput {
    fn from(v: &InputValue) -> Option<ComplexInput> {
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
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("ComplicatedArgs")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<String>>("intArgField")
                .argument(registry.arg::<Option<i32>>("intArg")),
            registry
                .field::<Option<String>>("nonNullIntArgField")
                .argument(registry.arg::<i32>("nonNullIntArg")),
            registry
                .field::<Option<String>>("stringArgField")
                .argument(registry.arg::<Option<String>>("stringArg")),
            registry
                .field::<Option<String>>("booleanArgField")
                .argument(registry.arg::<Option<bool>>("booleanArg")),
            registry
                .field::<Option<String>>("enumArgField")
                .argument(registry.arg::<Option<FurColor>>("enumArg")),
            registry
                .field::<Option<String>>("floatArgField")
                .argument(registry.arg::<Option<f64>>("floatArg")),
            registry
                .field::<Option<String>>("idArgField")
                .argument(registry.arg::<Option<ID>>("idArg")),
            registry
                .field::<Option<String>>("stringListArgField")
                .argument(registry.arg::<Option<Vec<Option<String>>>>("stringListArg")),
            registry
                .field::<Option<String>>("complexArgField")
                .argument(registry.arg::<Option<ComplexInput>>("complexArg")),
            registry
                .field::<Option<String>>("multipleReqs")
                .argument(registry.arg::<i32>("req1"))
                .argument(registry.arg::<i32>("req2")),
            registry
                .field::<Option<String>>("multipleOpts")
                .argument(registry.arg_with_default("opt1", &0i32))
                .argument(registry.arg_with_default("opt2", &0i32)),
            registry
                .field::<Option<String>>("multipleOptAndReq")
                .argument(registry.arg::<i32>("req1"))
                .argument(registry.arg::<i32>("req2"))
                .argument(registry.arg_with_default("opt1", &0i32))
                .argument(registry.arg_with_default("opt2", &0i32)),
        ];

        registry.build_object_type::<Self>(fields).into_meta()
    }
}

impl GraphQLType for QueryRoot {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("QueryRoot")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = &[
            registry
                .field::<Option<Human>>("human")
                .argument(registry.arg::<Option<ID>>("id")),
            registry.field::<Option<Alien>>("alien"),
            registry.field::<Option<Dog>>("dog"),
            registry.field::<Option<Cat>>("cat"),
            registry.field::<Option<Pet>>("pet"),
            registry.field::<Option<CatOrDog>>("catOrDog"),
            registry.field::<Option<DogOrHuman>>("dorOrHuman"),
            registry.field::<Option<HumanOrAlien>>("humanOrAlien"),
            registry.field::<Option<ComplicatedArgs>>("complicatedArgs"),
        ];

        registry.build_object_type::<Self>(fields).into_meta()
    }
}

pub fn validate<'a, R, V, F>(r: R, q: &'a str, factory: F) -> Vec<RuleError>
where
    R: GraphQLType,
    V: Visitor<'a> + 'a,
    F: Fn() -> V,
{
    let mut root = RootNode::new(r, EmptyMutation::<()>::new());

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

    let doc = parse_document_source(q).expect(&format!("Parse error on input {:#?}", q));
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
    expect_passes_rule_with_schema(QueryRoot, factory, q);
}

pub fn expect_passes_rule_with_schema<'a, R, V, F>(r: R, factory: F, q: &'a str)
where
    R: GraphQLType,
    V: Visitor<'a> + 'a,
    F: Fn() -> V,
{
    let errs = validate(r, q, factory);

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
    expect_fails_rule_with_schema(QueryRoot, factory, q, expected_errors);
}

pub fn expect_fails_rule_with_schema<'a, R, V, F>(
    r: R,
    factory: F,
    q: &'a str,
    expected_errors: &[RuleError],
) where
    R: GraphQLType,
    V: Visitor<'a> + 'a,
    F: Fn() -> V,
{
    let errs = validate(r, q, factory);

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
