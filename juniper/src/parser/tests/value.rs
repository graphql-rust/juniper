use indexmap::IndexMap;

use ast::{FromInputValue, InputValue, Type};
use parser::value::parse_value_literal;
use parser::{Lexer, Parser, SourcePosition, Spanning};
use value::{DefaultScalarValue, ParseScalarValue, ScalarRefValue, ScalarValue};

use schema::meta::{MetaType, ScalarMeta, EnumMeta, EnumValue, InputObjectMeta, Argument};
use schema::model::SchemaType;
use types::scalars::EmptyMutation;

#[derive(GraphQLEnum)]
enum Enum {
    EnumValue
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

graphql_object!(Query: () where Scalar = <S> |&self| {
    field int_field() -> i32 {
        42
    }

    field float_field() -> f64 {
        3.14
    }

    field string_field() -> String {
        "".into()
    }

    field bool_field() -> bool {
        true
    }

    field enum_field(_foo: Foo) -> Enum {
        Enum::EnumValue
    }
});

fn scalar_meta<T>(name: &'static str) -> MetaType<DefaultScalarValue>
where
    T: FromInputValue<DefaultScalarValue> + ParseScalarValue<DefaultScalarValue> + 'static,
{
    MetaType::Scalar(ScalarMeta::new::<T>(name.into()))
}

fn parse_value<S>(s: &str, meta: &MetaType<S>) -> Spanning<InputValue<S>>
where
    S: ScalarValue,
    for<'a> &'a S: ScalarRefValue<'a>,
{
    let mut lexer = Lexer::new(s);
    let mut parser = Parser::new(&mut lexer).expect(&format!("Lexer error on input {:#?}", s));
    let schema = SchemaType::new::<Query, EmptyMutation<()>>(&(), &());

    parse_value_literal(&mut parser, false, &schema, Some(meta))
        .expect(&format!("Parse error on input {:#?}", s))
}

#[test]
fn input_value_literals() {
    assert_eq!(
        parse_value::<DefaultScalarValue>("123", &scalar_meta::<i32>("Int")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(3, 0, 3),
            InputValue::scalar(123)
        )
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("123.45", &scalar_meta::<f64>("Float")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(6, 0, 6),
            InputValue::scalar(123.45)
        )
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("true", &scalar_meta::<bool>("Bool")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(4, 0, 4),
            InputValue::scalar(true)
        )
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("false", &scalar_meta::<bool>("Bool")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(5, 0, 5),
            InputValue::scalar(false)
        )
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>(r#""test""#, &scalar_meta::<String>("String")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(6, 0, 6),
            InputValue::scalar("test")
        )
    );
    let values = &[EnumValue::new("enum_value")];
    let e: EnumMeta<DefaultScalarValue> = EnumMeta::new::<Enum>("TestEnum".into(), values);

    assert_eq!(
        parse_value::<DefaultScalarValue>("enum_value", &MetaType::Enum(e)),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(10, 0, 10),
            InputValue::enum_value("enum_value")
        )
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("$variable", &scalar_meta::<i32>("Int")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(9, 0, 9),
            InputValue::variable("variable")
        )
    );
    assert_eq!(
        parse_value::<DefaultScalarValue>("[]", &scalar_meta::<i32>("Int")),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(2, 0, 2),
            InputValue::list(vec![])
        )
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
                    InputValue::scalar(1),
                ),
                Spanning::start_end(
                    &SourcePosition::new(4, 0, 4),
                    &SourcePosition::new(10, 0, 10),
                    InputValue::parsed_list(vec![
                        Spanning::start_end(
                            &SourcePosition::new(5, 0, 5),
                            &SourcePosition::new(6, 0, 6),
                            InputValue::scalar(2),
                        ),
                        Spanning::start_end(
                            &SourcePosition::new(8, 0, 8),
                            &SourcePosition::new(9, 0, 9),
                            InputValue::scalar(3),
                        ),
                    ]),
                ),
            ])
        )
    );
    let fields = [ Argument::new("key", Type::NonNullNamed("Int".into())),
                   Argument::new("other", Type::NonNullNamed("Bar".into()))];
    let meta = &MetaType::InputObject(InputObjectMeta::new::<Foo>("foo".into(), &fields));
    assert_eq!(
        parse_value::<DefaultScalarValue>("{}", meta),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(2, 0, 2),
            InputValue::object(IndexMap::<String, InputValue<DefaultScalarValue>>::new())
        )
    );

    assert_eq!(
        parse_value::<DefaultScalarValue>(
            r#"{key: 123, other: {foo: "bar"}}"#,
            meta
        ),
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
                        InputValue::scalar(123),
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
                                InputValue::scalar("bar"),
                            ),
                        )]),
                    ),
                ),
            ])
        )
    );
}
