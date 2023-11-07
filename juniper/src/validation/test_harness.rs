use std::mem;

use crate::{
    ast::{Document, FromInputValue, InputValue},
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
    value::ScalarValue,
    FieldError, GraphQLInputObject, IntoFieldError,
};

struct Being;
struct Pet;
struct Canine;
struct Unpopulated;

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

impl<S> GraphQLType<S> for Being
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("Being")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let fields = &[registry
            .field::<Option<String>>("name", i)
            .argument(registry.arg::<Option<bool>>("surname", i))];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for Being
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Pet
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("Pet")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let fields = &[registry
            .field::<Option<String>>("name", i)
            .argument(registry.arg::<Option<bool>>("surname", i))];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for Pet
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Canine
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("Canine")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let fields = &[registry
            .field::<Option<String>>("name", i)
            .argument(registry.arg::<Option<bool>>("surname", i))];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for Canine
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Unpopulated
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("Unpopulated")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let fields = &[registry
            .field::<Option<String>>("name", i)
            .argument(registry.arg::<Option<bool>>("surname", i))];

        registry
            .build_interface_type::<Self>(i, fields)
            .interfaces(&[registry.get_type::<Being>(i)])
            .into_meta()
    }
}

impl<S> GraphQLValue<S> for Unpopulated
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for DogCommand
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("DogCommand")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
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

impl<S> GraphQLValue<S> for DogCommand
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> FromInputValue<S> for DogCommand
where
    S: ScalarValue,
{
    type Error = &'static str;

    fn from_input_value<'a>(v: &InputValue<S>) -> Result<DogCommand, Self::Error> {
        match v.as_enum_value() {
            Some("SIT") => Ok(DogCommand::Sit),
            Some("HEEL") => Ok(DogCommand::Heel),
            Some("DOWN") => Ok(DogCommand::Down),
            _ => Err("Unknown DogCommand"),
        }
    }
}

impl<S> GraphQLType<S> for Dog
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("Dog")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
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

impl<S> GraphQLValue<S> for Dog
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for FurColor
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("FurColor")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
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

impl<S> GraphQLValue<S> for FurColor
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> FromInputValue<S> for FurColor
where
    S: ScalarValue,
{
    type Error = &'static str;

    fn from_input_value<'a>(v: &InputValue<S>) -> Result<FurColor, Self::Error> {
        match v.as_enum_value() {
            Some("BROWN") => Ok(FurColor::Brown),
            Some("BLACK") => Ok(FurColor::Black),
            Some("TAN") => Ok(FurColor::Tan),
            Some("SPOTTED") => Ok(FurColor::Spotted),
            _ => Err("Unknown FurColor"),
        }
    }
}

impl<S> GraphQLType<S> for Cat
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("Cat")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
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

impl<S> GraphQLValue<S> for Cat
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for CatOrDog
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("CatOrDog")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let types = &[registry.get_type::<Cat>(i), registry.get_type::<Dog>(i)];

        registry.build_union_type::<Self>(i, types).into_meta()
    }
}

impl<S> GraphQLValue<S> for CatOrDog
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Intelligent
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("Intelligent")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let fields = &[registry.field::<Option<i32>>("iq", i)];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for Intelligent
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Human
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("Human")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
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

impl<S> GraphQLValue<S> for Human
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Alien
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("Alien")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
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

impl<S> GraphQLValue<S> for Alien
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for DogOrHuman
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("DogOrHuman")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let types = &[registry.get_type::<Dog>(i), registry.get_type::<Human>(i)];

        registry.build_union_type::<Self>(i, types).into_meta()
    }
}

impl<S> GraphQLValue<S> for DogOrHuman
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for HumanOrAlien
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("HumanOrAlien")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let types = &[registry.get_type::<Human>(i), registry.get_type::<Alien>(i)];

        registry.build_union_type::<Self>(i, types).into_meta()
    }
}

impl<S> GraphQLValue<S> for HumanOrAlien
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for ComplexInput
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("ComplexInput")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
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

impl<S> GraphQLValue<S> for ComplexInput
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> FromInputValue<S> for ComplexInput
where
    S: ScalarValue,
{
    type Error = FieldError<S>;

    fn from_input_value<'a>(v: &InputValue<S>) -> Result<ComplexInput, Self::Error> {
        let obj = v.to_object_value().ok_or("Expected object")?;

        Ok(ComplexInput {
            required_field: obj
                .get("requiredField")
                .map(|v| v.convert())
                .transpose()?
                .ok_or("Expected requiredField")?,
            int_field: obj
                .get("intField")
                .map(|v| v.convert())
                .transpose()?
                .ok_or("Expected intField")?,
            string_field: obj
                .get("stringField")
                .map(|v| v.convert())
                .transpose()?
                .ok_or("Expected stringField")?,
            boolean_field: obj
                .get("booleanField")
                .map(|v| v.convert())
                .transpose()?
                .ok_or("Expected booleanField")?,
            string_list_field: obj
                .get("stringListField")
                .map(|v| v.convert().map_err(IntoFieldError::into_field_error))
                .transpose()?
                .ok_or("Expected stringListField")?,
        })
    }
}

impl<S> GraphQLType<S> for ComplicatedArgs
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("ComplicatedArgs")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
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
                .field::<Option<String>>("nonNullStringListArgField", i)
                .argument(registry.arg::<Vec<String>>("nonNullStringListArg", i)),
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

impl<S> GraphQLValue<S> for ComplicatedArgs
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for QueryRoot
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&'static str> {
        Some("QueryRoot")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
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

impl<S> GraphQLValue<S> for QueryRoot
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for MutationRoot
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&str> {
        Some("MutationRoot")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let _ = registry.get_type::<Unpopulated>(i);

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

impl<S> GraphQLValue<S> for MutationRoot
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for SubscriptionRoot
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<&str> {
        Some("SubscriptionRoot")
    }

    fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let fields = [];

        registry.build_object_type::<Self>(i, &fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for SubscriptionRoot
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType>::name(info)
    }
}

pub fn validate<'a, Q, M, Sub, F, S>(r: Q, m: M, s: Sub, q: &'a str, visit_fn: F) -> Vec<RuleError>
where
    S: ScalarValue + 'a,
    Q: GraphQLType<S, TypeInfo = ()>,
    M: GraphQLType<S, TypeInfo = ()>,
    Sub: GraphQLType<S, TypeInfo = ()>,
    F: FnOnce(&mut ValidatorContext<'a, S>, &'a Document<S>),
{
    let mut root = RootNode::new_with_scalar_value(r, m, s);

    root.schema.add_directive(DirectiveType::new(
        "onQuery",
        &[DirectiveLocation::Query],
        &[],
        false,
    ));
    root.schema.add_directive(DirectiveType::new(
        "onMutation",
        &[DirectiveLocation::Mutation],
        &[],
        false,
    ));
    root.schema.add_directive(DirectiveType::new(
        "onField",
        &[DirectiveLocation::Field],
        &[],
        false,
    ));
    root.schema.add_directive(DirectiveType::new(
        "onFragmentDefinition",
        &[DirectiveLocation::FragmentDefinition],
        &[],
        false,
    ));
    root.schema.add_directive(DirectiveType::new(
        "onFragmentSpread",
        &[DirectiveLocation::FragmentSpread],
        &[],
        false,
    ));
    root.schema.add_directive(DirectiveType::new(
        "onInlineFragment",
        &[DirectiveLocation::InlineFragment],
        &[],
        false,
    ));

    let doc = parse_document_source(q, &root.schema)
        .unwrap_or_else(|_| panic!("Parse error on input {q:#?}"));
    let mut ctx = ValidatorContext::new(unsafe { mem::transmute(&root.schema) }, &doc);

    visit_fn(&mut ctx, unsafe { mem::transmute(doc.as_slice()) });

    ctx.into_errors()
}

pub fn expect_passes_rule<'a, V, F, S>(factory: F, q: &'a str)
where
    S: ScalarValue + 'a,
    V: Visitor<'a, S> + 'a,
    F: Fn() -> V,
{
    expect_passes_rule_with_schema(QueryRoot, MutationRoot, SubscriptionRoot, factory, q);
}

pub fn expect_passes_fn<'a, F, S>(visit_fn: F, q: &'a str)
where
    S: ScalarValue + 'a,
    F: FnOnce(&mut ValidatorContext<'a, S>, &'a Document<S>),
{
    expect_passes_fn_with_schema(QueryRoot, MutationRoot, SubscriptionRoot, visit_fn, q);
}

pub fn expect_passes_rule_with_schema<'a, Q, M, Sub, V, F, S>(
    r: Q,
    m: M,
    s: Sub,
    factory: F,
    q: &'a str,
) where
    S: ScalarValue + 'a,
    Q: GraphQLType<S, TypeInfo = ()>,
    M: GraphQLType<S, TypeInfo = ()>,
    Sub: GraphQLType<S, TypeInfo = ()>,
    V: Visitor<'a, S> + 'a,
    F: FnOnce() -> V,
{
    let errs = validate(r, m, s, q, move |ctx, doc| {
        let mut mv = MultiVisitorNil.with(factory());
        visit(&mut mv, ctx, unsafe { mem::transmute(doc) });
    });

    if !errs.is_empty() {
        print_errors(&errs);
        panic!("Expected rule to pass, but errors found");
    }
}

pub fn expect_passes_fn_with_schema<'a, Q, M, Sub, F, S>(
    r: Q,
    m: M,
    s: Sub,
    visit_fn: F,
    q: &'a str,
) where
    S: ScalarValue + 'a,
    Q: GraphQLType<S, TypeInfo = ()>,
    M: GraphQLType<S, TypeInfo = ()>,
    Sub: GraphQLType<S, TypeInfo = ()>,
    F: FnOnce(&mut ValidatorContext<'a, S>, &'a Document<S>),
{
    let errs = validate(r, m, s, q, visit_fn);

    if !errs.is_empty() {
        print_errors(&errs);
        panic!("Expected `visit_fn` to pass, but errors found");
    }
}

pub fn expect_fails_rule<'a, V, F, S>(factory: F, q: &'a str, expected_errors: &[RuleError])
where
    S: ScalarValue + 'a,
    V: Visitor<'a, S> + 'a,
    F: Fn() -> V,
{
    expect_fails_rule_with_schema(QueryRoot, MutationRoot, factory, q, expected_errors);
}

pub fn expect_fails_fn<'a, F, S>(visit_fn: F, q: &'a str, expected_errors: &[RuleError])
where
    S: ScalarValue + 'a,
    F: FnOnce(&mut ValidatorContext<'a, S>, &'a Document<S>),
{
    expect_fails_fn_with_schema(QueryRoot, MutationRoot, visit_fn, q, expected_errors);
}

pub fn expect_fails_rule_with_schema<'a, Q, M, V, F, S>(
    r: Q,
    m: M,
    factory: F,
    q: &'a str,
    expected_errors: &[RuleError],
) where
    S: ScalarValue + 'a,
    Q: GraphQLType<S, TypeInfo = ()>,
    M: GraphQLType<S, TypeInfo = ()>,
    V: Visitor<'a, S> + 'a,
    F: FnOnce() -> V,
{
    let errs = validate(
        r,
        m,
        crate::EmptySubscription::<S>::new(),
        q,
        move |ctx, doc| {
            let mut mv = MultiVisitorNil.with(factory());
            visit(&mut mv, ctx, unsafe { mem::transmute(doc) });
        },
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

pub fn expect_fails_fn_with_schema<'a, Q, M, F, S>(
    r: Q,
    m: M,
    visit_fn: F,
    q: &'a str,
    expected_errors: &[RuleError],
) where
    S: ScalarValue + 'a,
    Q: GraphQLType<S, TypeInfo = ()>,
    M: GraphQLType<S, TypeInfo = ()>,
    F: FnOnce(&mut ValidatorContext<'a, S>, &'a Document<S>),
{
    let errs = validate(r, m, crate::EmptySubscription::<S>::new(), q, visit_fn);

    if errs.is_empty() {
        panic!("Expected `visit_fn` to fail, but no errors were found");
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
