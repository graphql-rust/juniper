use std::mem;

use arcstr::ArcStr;

use crate::{
    FieldError, GraphQLInputObject, IntoFieldError,
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
    validation::{MultiVisitorNil, RuleError, ValidatorContext, Visitor, visit},
    value::ScalarValue,
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

#[expect(dead_code, reason = "GraphQL schema testing")]
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
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("Being"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[registry
            .field::<Option<String>>(arcstr::literal!("name"), i)
            .argument(registry.arg::<Option<bool>>(arcstr::literal!("surname"), i))];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for Being
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Pet
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("Pet"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[registry
            .field::<Option<String>>(arcstr::literal!("name"), i)
            .argument(registry.arg::<Option<bool>>(arcstr::literal!("surname"), i))];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for Pet
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Canine
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("Canine"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[registry
            .field::<Option<String>>(arcstr::literal!("name"), i)
            .argument(registry.arg::<Option<bool>>(arcstr::literal!("surname"), i))];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for Canine
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Unpopulated
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("Unpopulated"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[registry
            .field::<Option<String>>(arcstr::literal!("name"), i)
            .argument(registry.arg::<Option<bool>>(arcstr::literal!("surname"), i))];

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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for DogCommand
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("DogCommand"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> FromInputValue<S> for DogCommand
where
    S: ScalarValue,
{
    type Error = &'static str;

    fn from_input_value(v: &InputValue<S>) -> Result<DogCommand, Self::Error> {
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
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("Dog"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[
            registry
                .field::<Option<String>>(arcstr::literal!("name"), i)
                .argument(registry.arg::<Option<bool>>(arcstr::literal!("surname"), i)),
            registry.field::<Option<String>>(arcstr::literal!("nickname"), i),
            registry.field::<Option<i32>>(arcstr::literal!("barkVolume"), i),
            registry.field::<Option<bool>>(arcstr::literal!("barks"), i),
            registry
                .field::<Option<bool>>(arcstr::literal!("doesKnowCommand"), i)
                .argument(registry.arg::<Option<DogCommand>>(arcstr::literal!("dogCommand"), i)),
            registry
                .field::<Option<bool>>(arcstr::literal!("isHousetrained"), i)
                .argument(registry.arg_with_default(arcstr::literal!("atOtherHomes"), &true, i)),
            registry
                .field::<Option<bool>>(arcstr::literal!("isAtLocation"), i)
                .argument(registry.arg::<Option<i32>>(arcstr::literal!("x"), i))
                .argument(registry.arg::<Option<i32>>(arcstr::literal!("y"), i)),
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for FurColor
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("FurColor"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> FromInputValue<S> for FurColor
where
    S: ScalarValue,
{
    type Error = &'static str;

    fn from_input_value(v: &InputValue<S>) -> Result<FurColor, Self::Error> {
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
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("Cat"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[
            registry
                .field::<Option<String>>(arcstr::literal!("name"), i)
                .argument(registry.arg::<Option<bool>>(arcstr::literal!("surname"), i)),
            registry.field::<Option<String>>(arcstr::literal!("nickname"), i),
            registry.field::<Option<bool>>(arcstr::literal!("meows"), i),
            registry.field::<Option<i32>>(arcstr::literal!("meowVolume"), i),
            registry.field::<Option<FurColor>>(arcstr::literal!("furColor"), i),
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for CatOrDog
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("CatOrDog"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Intelligent
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("Intelligent"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[registry.field::<Option<i32>>(arcstr::literal!("iq"), i)];

        registry.build_interface_type::<Self>(i, fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for Intelligent
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Human
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("Human"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[
            registry
                .field::<Option<String>>(arcstr::literal!("name"), i)
                .argument(registry.arg::<Option<bool>>(arcstr::literal!("surname"), i)),
            registry.field::<Option<Vec<Option<Pet>>>>(arcstr::literal!("pets"), i),
            registry.field::<Option<Vec<Human>>>(arcstr::literal!("relatives"), i),
            registry.field::<Option<i32>>(arcstr::literal!("iq"), i),
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for Alien
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("Alien"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[
            registry
                .field::<Option<String>>(arcstr::literal!("name"), i)
                .argument(registry.arg::<Option<bool>>(arcstr::literal!("surname"), i)),
            registry.field::<Option<i32>>(arcstr::literal!("iq"), i),
            registry.field::<Option<i32>>(arcstr::literal!("numEyes"), i),
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for DogOrHuman
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("DogOrHuman"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for HumanOrAlien
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("HumanOrAlien"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for ComplexInput
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("ComplexInput"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[
            registry.arg::<bool>(arcstr::literal!("requiredField"), i),
            registry.arg::<Option<i32>>(arcstr::literal!("intField"), i),
            registry.arg::<Option<String>>(arcstr::literal!("stringField"), i),
            registry.arg::<Option<bool>>(arcstr::literal!("booleanField"), i),
            registry.arg::<Option<Vec<Option<String>>>>(arcstr::literal!("stringListField"), i),
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> FromInputValue<S> for ComplexInput
where
    S: ScalarValue,
{
    type Error = FieldError<S>;

    fn from_input_value(v: &InputValue<S>) -> Result<ComplexInput, Self::Error> {
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
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("ComplicatedArgs"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[
            registry
                .field::<Option<String>>(arcstr::literal!("intArgField"), i)
                .argument(registry.arg::<Option<i32>>(arcstr::literal!("intArg"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("nonNullIntArgField"), i)
                .argument(registry.arg::<i32>(arcstr::literal!("nonNullIntArg"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("nonNullIntArgFieldWithDefault"), i)
                .argument(registry.arg_with_default::<i32>(
                    arcstr::literal!("nonNullIntArg"),
                    &3,
                    i,
                )),
            registry
                .field::<Option<String>>(arcstr::literal!("stringArgField"), i)
                .argument(registry.arg::<Option<String>>(arcstr::literal!("stringArg"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("booleanArgField"), i)
                .argument(registry.arg::<Option<bool>>(arcstr::literal!("booleanArg"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("enumArgField"), i)
                .argument(registry.arg::<Option<FurColor>>(arcstr::literal!("enumArg"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("floatArgField"), i)
                .argument(registry.arg::<Option<f64>>(arcstr::literal!("floatArg"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("idArgField"), i)
                .argument(registry.arg::<Option<ID>>(arcstr::literal!("idArg"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("stringListArgField"), i)
                .argument(
                    registry
                        .arg::<Option<Vec<Option<String>>>>(arcstr::literal!("stringListArg"), i),
                ),
            registry
                .field::<Option<String>>(arcstr::literal!("nonNullStringListArgField"), i)
                .argument(registry.arg::<Vec<String>>(arcstr::literal!("nonNullStringListArg"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("complexArgField"), i)
                .argument(registry.arg::<Option<ComplexInput>>(arcstr::literal!("complexArg"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("multipleReqs"), i)
                .argument(registry.arg::<i32>(arcstr::literal!("req1"), i))
                .argument(registry.arg::<i32>(arcstr::literal!("req2"), i)),
            registry
                .field::<Option<String>>(arcstr::literal!("multipleOpts"), i)
                .argument(registry.arg_with_default(arcstr::literal!("opt1"), &0i32, i))
                .argument(registry.arg_with_default(arcstr::literal!("opt2"), &0i32, i)),
            registry
                .field::<Option<String>>(arcstr::literal!("multipleOptAndReq"), i)
                .argument(registry.arg::<i32>(arcstr::literal!("req1"), i))
                .argument(registry.arg::<i32>(arcstr::literal!("req2"), i))
                .argument(registry.arg_with_default(arcstr::literal!("opt1"), &0i32, i))
                .argument(registry.arg_with_default(arcstr::literal!("opt2"), &0i32, i)),
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for QueryRoot
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("QueryRoot"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let fields = &[
            registry
                .field::<Option<Human>>(arcstr::literal!("human"), i)
                .argument(registry.arg::<Option<ID>>(arcstr::literal!("id"), i)),
            registry.field::<Option<Alien>>(arcstr::literal!("alien"), i),
            registry.field::<Option<Dog>>(arcstr::literal!("dog"), i),
            registry.field::<Option<Cat>>(arcstr::literal!("cat"), i),
            registry.field::<Option<Pet>>(arcstr::literal!("pet"), i),
            registry.field::<Option<CatOrDog>>(arcstr::literal!("catOrDog"), i),
            registry.field::<Option<DogOrHuman>>(arcstr::literal!("dorOrHuman"), i),
            registry.field::<Option<HumanOrAlien>>(arcstr::literal!("humanOrAlien"), i),
            registry.field::<Option<ComplicatedArgs>>(arcstr::literal!("complicatedArgs"), i),
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for MutationRoot
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("MutationRoot"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
        let _ = registry.get_type::<Unpopulated>(i);

        let fields = [registry
            .field::<i32>(arcstr::literal!("testInput"), i)
            .argument(registry.arg_with_default::<TestInput>(
                arcstr::literal!("input"),
                &TestInput {
                    id: 423,
                    name: String::from("foo"),
                },
                i,
            ))];

        registry.build_object_type::<Self>(i, &fields).into_meta()
    }
}

impl<S> GraphQLValue<S> for MutationRoot
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

impl<S> GraphQLType<S> for SubscriptionRoot
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("SubscriptionRoot"))
    }

    fn meta(i: &(), registry: &mut Registry<S>) -> MetaType<S> {
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

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }
}

pub(crate) fn validate<'a, Q, M, Sub, F, S>(
    r: Q,
    m: M,
    s: Sub,
    q: &'a str,
    visit_fn: F,
) -> Vec<RuleError>
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

pub(crate) fn expect_passes_rule<'a, V, F, S>(factory: F, q: &'a str)
where
    S: ScalarValue + 'a,
    V: Visitor<'a, S> + 'a,
    F: Fn() -> V,
{
    expect_passes_rule_with_schema(QueryRoot, MutationRoot, SubscriptionRoot, factory, q);
}

pub(crate) fn expect_passes_rule_with_schema<'a, Q, M, Sub, V, F, S>(
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

pub(crate) fn expect_fails_rule<'a, V, F, S>(factory: F, q: &'a str, expected_errors: &[RuleError])
where
    S: ScalarValue + 'a,
    V: Visitor<'a, S> + 'a,
    F: Fn() -> V,
{
    expect_fails_rule_with_schema(QueryRoot, MutationRoot, factory, q, expected_errors);
}

pub(crate) fn expect_fails_fn<'a, F, S>(visit_fn: F, q: &'a str, expected_errors: &[RuleError])
where
    S: ScalarValue + 'a,
    F: FnOnce(&mut ValidatorContext<'a, S>, &'a Document<S>),
{
    expect_fails_fn_with_schema(QueryRoot, MutationRoot, visit_fn, q, expected_errors);
}

pub(crate) fn expect_fails_rule_with_schema<'a, Q, M, V, F, S>(
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

pub(crate) fn expect_fails_fn_with_schema<'a, Q, M, F, S>(
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
