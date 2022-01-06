use crate::ast::InputValue;

use crate::{
    parser::{ParseError, ParseResult, Parser, ScalarToken, SourcePosition, Spanning, Token},
    schema::{
        meta::{InputObjectMeta, MetaType},
        model::SchemaType,
    },
    value::ScalarValue,
};

pub fn parse_value_literal<'a, 'b, S>(
    parser: &mut Parser<'a>,
    is_const: bool,
    schema: &'b SchemaType<'b, S>,
    tpe: Option<&MetaType<'b, S>>,
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
            None,
        ) => parse_object_literal(parser, is_const, schema, None),
        (
            &Spanning {
                item: Token::CurlyOpen,
                ..
            },
            Some(&MetaType::InputObject(ref o)),
        ) => parse_object_literal(parser, is_const, schema, Some(o)),
        (
            &Spanning {
                item: Token::Dollar,
                ..
            },
            _,
        ) if !is_const => parse_variable_literal(parser),
        (
            &Spanning {
                item: Token::Scalar(_),
                ..
            },
            Some(&MetaType::Scalar(ref s)),
        ) => {
            if let Spanning {
                item: Token::Scalar(scalar),
                start,
                end,
            } = parser.next_token()?
            {
                (s.parse_fn)(scalar)
                    .map(|s| Spanning::start_end(&start, &end, InputValue::Scalar(s)))
                    .or_else(|_| parse_scalar_literal_by_infered_type(scalar, &start, &end, schema))
            } else {
                unreachable!()
            }
        }
        (
            &Spanning {
                item: Token::Scalar(_),
                ..
            },
            _,
        ) => {
            if let Spanning {
                item: Token::Scalar(token),
                start,
                end,
            } = parser.next_token()?
            {
                parse_scalar_literal_by_infered_type(token, &start, &end, schema)
            } else {
                unreachable!()
            }
        }
        (
            &Spanning {
                item: Token::Name("true"),
                ..
            },
            _,
        ) => Ok(parser.next_token()?.map(|_| InputValue::scalar(true))),
        (
            &Spanning {
                item: Token::Name("false"),
                ..
            },
            _,
        ) => Ok(parser.next_token()?.map(|_| InputValue::scalar(false))),
        (
            &Spanning {
                item: Token::Name("null"),
                ..
            },
            _,
        ) => Ok(parser.next_token()?.map(|_| InputValue::null())),
        (
            &Spanning {
                item: Token::Name(name),
                ..
            },
            _,
        ) => Ok(parser.next_token()?.map(|_| InputValue::enum_value(name))),
        _ => Err(parser.next_token()?.map(ParseError::UnexpectedToken)),
    }
}

fn parse_list_literal<'a, 'b, S>(
    parser: &mut Parser<'a>,
    is_const: bool,
    schema: &'b SchemaType<'b, S>,
    tpe: Option<&MetaType<'b, S>>,
) -> ParseResult<'a, InputValue<S>>
where
    S: ScalarValue,
{
    Ok(parser
        .delimited_list(
            &Token::BracketOpen,
            |p| parse_value_literal(p, is_const, schema, tpe),
            &Token::BracketClose,
        )?
        .map(InputValue::parsed_list))
}

fn parse_object_literal<'a, 'b, S>(
    parser: &mut Parser<'a>,
    is_const: bool,
    schema: &'b SchemaType<'b, S>,
    object_tpe: Option<&InputObjectMeta<'b, S>>,
) -> ParseResult<'a, InputValue<S>>
where
    S: ScalarValue,
{
    Ok(parser
        .delimited_list(
            &Token::CurlyOpen,
            |p| parse_object_field(p, is_const, schema, object_tpe),
            &Token::CurlyClose,
        )?
        .map(|items| InputValue::parsed_object(items.into_iter().map(|s| s.item).collect())))
}

fn parse_object_field<'a, 'b, S>(
    parser: &mut Parser<'a>,
    is_const: bool,
    schema: &'b SchemaType<'b, S>,
    object_meta: Option<&InputObjectMeta<'b, S>>,
) -> ParseResult<'a, (Spanning<String>, Spanning<InputValue<S>>)>
where
    S: ScalarValue,
{
    let key = parser.expect_name()?;

    let tpe = object_meta
        .and_then(|o| o.input_fields.iter().find(|f| f.name == key.item))
        .and_then(|f| schema.lookup_type(&f.arg_type));

    parser.expect(&Token::Colon)?;

    let value = parse_value_literal(parser, is_const, schema, tpe)?;

    Ok(Spanning::start_end(
        &key.start,
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

fn parse_scalar_literal_by_infered_type<'a, 'b, S>(
    token: ScalarToken<'a>,
    start: &SourcePosition,
    end: &SourcePosition,
    schema: &'b SchemaType<'b, S>,
) -> ParseResult<'a, InputValue<S>>
where
    S: ScalarValue,
{
    let result = match token {
        ScalarToken::String(_) => {
            if let Some(&MetaType::Scalar(ref s)) = schema.concrete_type_by_name("String") {
                (s.parse_fn)(token).map(InputValue::Scalar)
            } else {
                Err(ParseError::ExpectedScalarError(
                    "There needs to be a String type",
                ))
            }
        }
        ScalarToken::Int(_) => {
            if let Some(&MetaType::Scalar(ref s)) = schema.concrete_type_by_name("Int") {
                (s.parse_fn)(token).map(InputValue::Scalar)
            } else {
                Err(ParseError::ExpectedScalarError(
                    "There needs to be an Int type",
                ))
            }
        }
        ScalarToken::Float(_) => {
            if let Some(&MetaType::Scalar(ref s)) = schema.concrete_type_by_name("Float") {
                (s.parse_fn)(token).map(InputValue::Scalar)
            } else {
                Err(ParseError::ExpectedScalarError(
                    "There needs to be a Float type",
                ))
            }
        }
    };
    result
        .map(|s| Spanning::start_end(start, end, s))
        .map_err(|e| Spanning::start_end(start, end, e))
}
