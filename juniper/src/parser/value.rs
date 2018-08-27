use ast::{InputValue, Type};

use parser::lexer::LexerError;
use parser::{ParseError, ParseResult, Parser, Spanning, Token};
use schema::meta::{MetaType, ObjectMeta};
use schema::model::SchemaType;
use value::ScalarValue;

pub fn parse_value_literal<'a, 'b, S>(
    parser: &mut Parser<'a>,
    is_const: bool,
    schema: &'b SchemaType<'b, S>,
    tpe: &MetaType<'b, S>,
) -> ParseResult<'a, InputValue<S>>
where
    S: ScalarValue,
{
    match (parser.peek(), tpe) {
        (
            &Spanning {
                item: Token::BracketOpen,
                ..
            },
            _,
        ) => parse_list_literal(parser, is_const, schema, tpe),
        (
            &Spanning {
                item: Token::CurlyOpen,
                ..
            },
            MetaType::Object(ref o),
        ) => parse_object_literal(parser, is_const, schema, o),
        (
            &Spanning {
                item: Token::Dollar,
                ..
            },
            _,
        )
            if !is_const =>
        {
            parse_variable_literal(parser)
        }
        (
            &Spanning {
                item: Token::Scalar(_),
                ..
            },
            MetaType::Scalar(ref s),
        ) => {
            if let Spanning {
                item: Token::Scalar(scalar),
                start,
                end,
            } = parser.next()?
            {
                println!("{:?}", scalar);
                (s.parse_fn)(scalar)
                    .map(|s| Spanning::start_end(&start, &end, InputValue::Scalar(s)))
                    .map_err(|e| Spanning::start_end(&start, &end, e))
            } else {
                unreachable!()
            }
        }
        (
            &Spanning {
                item: Token::Scalar(_),
                ..
            },
            MetaType::Enum(_),
        ) => {
            if let Spanning {
                item: Token::Scalar(scalar),
                start,
                end,
            } = parser.next()?
            {
                if let Some(MetaType::Scalar(s)) = schema.concrete_type_by_name("String") {
                    (s.parse_fn)(scalar)
                        .map(|s| Spanning::start_end(&start, &end, InputValue::Scalar(s)))
                        .map_err(|e| Spanning::start_end(&start, &end, e))
                } else {
                    panic!("There needs to be a String type")
                }
            } else {
                unreachable!()
            }
        }
        (
            &Spanning {
                item: Token::Name("true"),
                ..
            },
            MetaType::Scalar(ref _s),
        ) => Ok(parser.next()?.map(|_| InputValue::boolean(true))),
        (
            &Spanning {
                item: Token::Name("false"),
                ..
            },
            &MetaType::Scalar(ref _s),
        ) => Ok(parser.next()?.map(|_| InputValue::boolean(false))),
        (
            &Spanning {
                item: Token::Name("null"),
                ..
            },
            _,
        ) => Ok(parser.next()?.map(|_| InputValue::null())),
        (
            &Spanning {
                item: Token::Name(name),
                ..
            },
            MetaType::Enum(_),
        ) => Ok(parser
            .next()?
            .map(|_| InputValue::enum_value(name.to_owned()))),
        _ => Err(parser.next()?.map(ParseError::UnexpectedToken)),
    }
}

fn parse_list_literal<'a, 'b, S>(
    parser: &mut Parser<'a>,
    is_const: bool,
    schema: &'b SchemaType<'b, S>,
    tpe: &MetaType<'b, S>,
) -> ParseResult<'a, InputValue<S>>
where
    S: ScalarValue,
{
    Ok(parser
        .delimited_list(
            &Token::BracketOpen,
            |p| parse_value_literal(p, is_const, schema, tpe),
            &Token::BracketClose,
        )?.map(InputValue::parsed_list))
}

fn parse_object_literal<'a, 'b, S>(
    parser: &mut Parser<'a>,
    is_const: bool,
    schema: &'b SchemaType<'b, S>,
    object_tpe: &ObjectMeta<'b, S>,
) -> ParseResult<'a, InputValue<S>>
where
    S: ScalarValue,
{
    Ok(parser
        .delimited_list(
            &Token::CurlyOpen,
            |p| parse_object_field(p, is_const, schema, object_tpe),
            &Token::CurlyClose,
        )?.map(|items| InputValue::parsed_object(items.into_iter().map(|s| s.item).collect())))
}

fn parse_object_field<'a, 'b, S>(
    parser: &mut Parser<'a>,
    is_const: bool,
    schema: &'b SchemaType<'b, S>,
    object_meta: &ObjectMeta<'b, S>,
) -> ParseResult<'a, (Spanning<String>, Spanning<InputValue<S>>)>
where
    S: ScalarValue,
{
    let key = parser.expect_name()?;

    let field = object_meta
        .fields
        .iter()
        .find(|f| f.name == key.item)
        .ok_or_else(|| unimplemented!())?;

    let tpe = schema
        .lookup_type(&field.field_type)
        .ok_or_else(|| unimplemented!())?;

    parser.expect(&Token::Colon)?;

    let value = parse_value_literal(parser, is_const, schema, &tpe)?;

    Ok(Spanning::start_end(
        &key.start.clone(),
        &value.end.clone(),
        (key.map(|s| s.to_owned()), value),
    ))
}

fn parse_variable_literal<'a, S>(parser: &mut Parser<'a>) -> ParseResult<'a, InputValue<S>>
where
    S: ScalarValue,
{
    let Spanning {
        start: start_pos, ..
    } = parser.expect(&Token::Dollar)?;
    let Spanning {
        item: name,
        end: end_pos,
        ..
    } = parser.expect_name()?;

    Ok(Spanning::start_end(
        &start_pos,
        &end_pos,
        InputValue::variable(name),
    ))
}
