use crate::{
    ast::{FromInputValue, InputValue, Type},
    graphql_input_value,
    parser::{value::parse_value_literal, Lexer, Parser, SourcePosition, Spanning},
    schema::{
        meta::{Argument, EnumMeta, EnumValue, InputObjectMeta, MetaType, ScalarMeta},
        model::SchemaType,
    },
    types::scalars::{EmptyMutation, EmptySubscription},
    value::{DefaultScalarValue, ParseScalarValue, ScalarValue},
    GraphQLEnum, GraphQLInputObject, IntoFieldError,
};

#[derive(GraphQLEnum)]
enum Enum {
    EnumValue,
}

#[derive(GraphQLInputObject)]
struct Bar {
    foo: String,
}

#[derive(GraphQLInputObject)]
struct Foo {
    key: i32,
    other: Bar,
}

struct Query;

#[crate::graphql_object]
impl Query {
    fn int_field() -> i32 {
        42
    }

    fn float_field() -> f64 {
        3.12
    }

    fn string_field() -> String {
        "".into()
    }

    fn bool_field() -> bool {
        true
    }

    fn enum_field(_foo: Foo) -> Enum {
        Enum::EnumValue
    }
}

fn scalar_meta<T>(name: &'static str) -> MetaType
where
    T: FromInputValue<DefaultScalarValue> + ParseScalarValue<DefaultScalarValue>,
    T::Error: IntoFieldError,
{
    MetaType::Scalar(ScalarMeta::new::<T>(name.into()))
}

fn parse_value<S>(s: &str, meta: &MetaType<S>) -> Spanning<InputValue<S>>
where
    S: ScalarValue,
{
    let mut lexer = Lexer::new(s);
    let mut parser =
        Parser::new(&mut lexer).unwrap_or_else(|_| panic!("Lexer error on input {s:#?}"));
    let schema = SchemaType::new::<Query, EmptyMutation<()>, EmptySubscription<()>>(&(), &(), &());

    parse_value_literal(&mut parser, false, &schema, Some(meta))
        .unwrap_or_else(|_| panic!("Parse error on input {s:#?}"))
}

#[test]
fn input_value_literals() {
    assert_eq!(
        parse_value::<DefaultScalarValue>("123", &scalar_meta::<i32>("Int")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(3, 0, 3),
            graphql_input_value!(123),
        ),
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("123.45", &scalar_meta::<f64>("Float")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(6, 0, 6),
            graphql_input_value!(123.45),
        ),
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("true", &scalar_meta::<bool>("Bool")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(4, 0, 4),
            graphql_input_value!(true),
        ),
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("false", &scalar_meta::<bool>("Bool")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(5, 0, 5),
            graphql_input_value!(false),
        ),
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>(r#""test""#, &scalar_meta::<String>("String")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(6, 0, 6),
            graphql_input_value!("test"),
        ),
    );
    let values = &[EnumValue::new("enum_value")];
    let e: EnumMeta<DefaultScalarValue> = EnumMeta::new::<Enum>("TestEnum".into(), values);

    assert_eq!(
        parse_value::<DefaultScalarValue>("enum_value", &MetaType::Enum(e)),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(10, 0, 10),
            graphql_input_value!(enum_value),
        ),
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("$variable", &scalar_meta::<i32>("Int")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(9, 0, 9),
            graphql_input_value!(@variable),
        ),
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("[]", &scalar_meta::<i32>("Int")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(2, 0, 2),
            graphql_input_value!([]),
        ),
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("[1, [2, 3]]", &scalar_meta::<i32>("Int")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(11, 0, 11),
            InputValue::parsed_list(vec![
                Spanning::start_end(
                    &SourcePosition::new(1, 0, 1),
                    &SourcePosition::new(2, 0, 2),
                    graphql_input_value!(1),
                ),
                Spanning::start_end(
                    &SourcePosition::new(4, 0, 4),
                    &SourcePosition::new(10, 0, 10),
                    InputValue::parsed_list(vec![
                        Spanning::start_end(
                            &SourcePosition::new(5, 0, 5),
                            &SourcePosition::new(6, 0, 6),
                            graphql_input_value!(2),
                        ),
                        Spanning::start_end(
                            &SourcePosition::new(8, 0, 8),
                            &SourcePosition::new(9, 0, 9),
                            graphql_input_value!(3),
                        ),
                    ]),
                ),
            ]),
        ),
    );
    let fields = [
        Argument::new("key", Type::NonNullNamed("Int".into())),
        Argument::new("other", Type::NonNullNamed("Bar".into())),
    ];
    let meta = &MetaType::InputObject(InputObjectMeta::new::<Foo>("foo".into(), &fields));
    assert_eq!(
        parse_value::<DefaultScalarValue>("{}", meta),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(2, 0, 2),
            graphql_input_value!({}),
        ),
    );

    assert_eq!(
        parse_value::<DefaultScalarValue>(r#"{key: 123, other: {foo: "bar"}}"#, meta),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(31, 0, 31),
            InputValue::parsed_object(vec![
                (
                    Spanning::start_end(
                        &SourcePosition::new(1, 0, 1),
                        &SourcePosition::new(4, 0, 4),
                        "key".to_owned(),
                    ),
                    Spanning::start_end(
                        &SourcePosition::new(6, 0, 6),
                        &SourcePosition::new(9, 0, 9),
                        graphql_input_value!(123),
                    ),
                ),
                (
                    Spanning::start_end(
                        &SourcePosition::new(11, 0, 11),
                        &SourcePosition::new(16, 0, 16),
                        "other".to_owned(),
                    ),
                    Spanning::start_end(
                        &SourcePosition::new(18, 0, 18),
                        &SourcePosition::new(30, 0, 30),
                        InputValue::parsed_object(vec![(
                            Spanning::start_end(
                                &SourcePosition::new(19, 0, 19),
                                &SourcePosition::new(22, 0, 22),
                                "foo".to_owned(),
                            ),
                            Spanning::start_end(
                                &SourcePosition::new(24, 0, 24),
                                &SourcePosition::new(29, 0, 29),
                                graphql_input_value!("bar"),
                            ),
                        )]),
                    ),
                ),
            ]),
        ),
    );
}
