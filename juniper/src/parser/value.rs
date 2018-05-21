use ast::InputValue;

use parser::{ParseError, ParseResult, Parser, Spanning, Token};

pub fn parse_value_literal<'a>(
    parser: &mut Parser<'a>,
    is_const: bool,
) -> ParseResult<'a, InputValue> {
    match *parser.peek() {
        Spanning {
            item: Token::BracketOpen,
            ..
        } => parse_list_literal(parser, is_const),
        Spanning {
            item: Token::CurlyOpen,
            ..
        } => parse_object_literal(parser, is_const),
        Spanning {
            item: Token::Dollar,
            ..
        } if !is_const =>
        {
            parse_variable_literal(parser)
        }
        Spanning {
            item: Token::Int(i),
            ..
        } => Ok(parser.next()?.map(|_| InputValue::int(i))),
        Spanning {
            item: Token::Float(f),
            ..
        } => Ok(parser.next()?.map(|_| InputValue::float(f))),
        Spanning {
            item: Token::String(_),
            ..
        } => Ok(parser.next()?.map(|t| {
            if let Token::String(s) = t {
                InputValue::string(s)
            } else {
                panic!("Internal parser error");
            }
        })),
        Spanning {
            item: Token::Name("true"),
            ..
        } => Ok(parser.next()?.map(|_| InputValue::boolean(true))),
        Spanning {
            item: Token::Name("false"),
            ..
        } => Ok(parser.next()?.map(|_| InputValue::boolean(false))),
        Spanning {
            item: Token::Name("null"),
            ..
        } => Ok(parser.next()?.map(|_| InputValue::null())),
        Spanning {
            item: Token::Name(name),
            ..
        } => Ok(parser
            .next()?
            .map(|_| InputValue::enum_value(name.to_owned()))),
        _ => Err(parser.next()?.map(ParseError::UnexpectedToken)),
    }
}

fn parse_list_literal<'a>(parser: &mut Parser<'a>, is_const: bool) -> ParseResult<'a, InputValue> {
    Ok(parser
        .delimited_list(
            &Token::BracketOpen,
            |p| parse_value_literal(p, is_const),
            &Token::BracketClose,
        )?
        .map(InputValue::parsed_list))
}

fn parse_object_literal<'a>(
    parser: &mut Parser<'a>,
    is_const: bool,
) -> ParseResult<'a, InputValue> {
    Ok(parser
        .delimited_list(
            &Token::CurlyOpen,
            |p| parse_object_field(p, is_const),
            &Token::CurlyClose,
        )?
        .map(|items| InputValue::parsed_object(items.into_iter().map(|s| s.item).collect())))
}

fn parse_object_field<'a>(
    parser: &mut Parser<'a>,
    is_const: bool,
) -> ParseResult<'a, (Spanning<String>, Spanning<InputValue>)> {
    let key = parser.expect_name()?;

    parser.expect(&Token::Colon)?;

    let value = parse_value_literal(parser, is_const)?;

    Ok(Spanning::start_end(
        &key.start.clone(),
        &value.end.clone(),
        (key.map(|s| s.to_owned()), value),
    ))
}

fn parse_variable_literal<'a>(parser: &mut Parser<'a>) -> ParseResult<'a, InputValue> {
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
