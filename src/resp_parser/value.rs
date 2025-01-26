use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_while};
use nom::character::complete::{i32, i64, one_of};
use nom::combinator::{complete, eof, value, verify};
use nom::error::{Error, ErrorKind};
use nom::multi::count;
use nom::sequence::terminated;
use nom::{error_position, IResult};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    SimpleString(&'a str),
    Error(&'a str),
    Integer(i64),
    BulkString(&'a str),
    Array(Vec<Value<'a>>),
    Null,
    Map(HashMap<&'a str, Value<'a>>),
    Set(BTreeSet<Value<'a>>),
    Boolean(bool),
}

pub fn parse_message(input: &str) -> IResult<&str, Value> {
    let (input, value) = terminated(parse_value, eof)(input)?;
    Ok((input, value))
}

pub fn parse_value(input: &str) -> IResult<&str, Value> {
    alt((
        parse_simple_string,
        parse_error,
        parse_integer,
        parse_bulk_string,
        parse_array,
        parse_null,
        parse_bool,
    ))(input)
}

fn parse_simple_string_raw(input: &str) -> IResult<&str, &str> {
    terminated(take_while(|c| c != '\r' && c != '\n'), crlf)(input)
}

fn parse_simple_string(input: &str) -> IResult<&str, Value> {
    let (input, _) = tag("+")(input)?;
    let (input, value) = terminated(take_while(|c| c != '\r' && c != '\n'), crlf)(input)?;
    Ok((input, Value::SimpleString(value)))
}

fn parse_error(input: &str) -> IResult<&str, Value> {
    let (input, _) = tag("-")(input)?;
    let (input, value) = parse_simple_string_raw(input)?;
    Ok((input, Value::Error(value)))
}

fn parse_integer(input: &str) -> IResult<&str, Value> {
    let (input, _) = tag(":")(input)?;
    let (input, value) = terminated(i64, crlf)(input)?;
    Ok((input, Value::Integer(value)))
}

fn parse_bulk_string(input: &str) -> IResult<&str, Value> {
    let (input, _) = tag("$")(input)?;
    let (input, length) = terminated(i32, crlf)(input)?;
    if length == -1 {
        return Ok((input, Value::Null));
    }
    if length < -1 {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify)));
    }

    let (input, value) = terminated(take(length as usize), crlf)(input)?;
    Ok((input, Value::BulkString(value)))
}

fn parse_array(input: &str) -> IResult<&str, Value> {
    let (input, _) = tag("*")(input)?;
    let (input, length) = terminated(i32, crlf)(input)?;
    if length == -1 {
        return Ok((input, Value::Null));
    }
    if length < -1 {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify)));
    }

    let (input, values) = count(parse_value, length as usize)(input)?;
    Ok((input, Value::Array(values)))
}

fn parse_null(input: &str) -> IResult<&str, Value> {
    let (input, _) = tag("_\r\n")(input)?;
    Ok((input, Value::Null))
}

fn parse_bool(input: &str) -> IResult<&str, Value> {
    let (input, _) = tag("#")(input)?;
    let (input, ch) = terminated(one_of("tf"), crlf)(input)?;
    let value = match ch {
        't' => true,
        'f' => false,
        _ => unreachable!(),
    };
    Ok((input, Value::Boolean(value)))
}

fn crlf(input: &str) -> IResult<&str, &str> {
    tag("\r\n")(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_string() {
        assert_eq!(
            parse_message("+OK\r\n"),
            Ok(("", Value::SimpleString("OK")))
        );
        assert!(parse_message("+O\nK\r\n").is_err());
        assert!(parse_message("+O\nK\r\nTHIS_SHOULD_NOT_BE_HERE").is_err());
    }

    #[test]
    fn test_parse_error() {
        assert_eq!(
            parse_message("-Error message\r\n"),
            Ok(("", Value::Error("Error message")))
        );
        assert_eq!(
            parse_message("-ERR unknown command 'foobar'\r\n"),
            Ok(("", Value::Error("ERR unknown command 'foobar'")))
        );
        assert_eq!(
            parse_message("-WRONGTYPE Operation against a key holding the wrong kind of value\r\n"),
            Ok((
                "",
                Value::Error("WRONGTYPE Operation against a key holding the wrong kind of value")
            ))
        );
        assert!(parse_message("-Error\nmessage\r\n").is_err());
        assert!(parse_message("-Error message\r\nTHIS_SHOULD_NOT_BE_HERE").is_err());
    }

    #[test]
    fn test_parse_integer() {
        assert_eq!(parse_message(":0\r\n"), Ok(("", Value::Integer(0))));
        assert_eq!(parse_message(":1000\r\n"), Ok(("", Value::Integer(1000))));
        assert_eq!(parse_message(":-1000\r\n"), Ok(("", Value::Integer(-1000))));
        assert!(parse_message(":-1000\n").is_err());
        assert!(parse_message(":-1000\r\nTHIS_SHOULD_NOT_BE_HERE").is_err());
    }

    #[test]
    fn test_parse_bulk_string() {
        assert_eq!(
            parse_message("$6\r\nfoobar\r\n"),
            Ok(("", Value::BulkString("foobar")))
        );
        assert_eq!(parse_message("$0\r\n\r\n"), Ok(("", Value::BulkString(""))));
        assert_eq!(parse_message("$-1\r\n"), Ok(("", Value::Null)));
        assert!(parse_message("$-2\r\n").is_err());
        assert!(parse_message("$10\r\n123456789\r\n").is_err());
    }

    #[test]
    fn test_parse_array() {
        assert_eq!(parse_message("*-1\r\n"), Ok(("", Value::Null)));
        assert_eq!(parse_message("*0\r\n"), Ok(("", Value::Array(vec![]))));
        assert_eq!(
            parse_message("*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"),
            Ok((
                "",
                Value::Array(vec![Value::BulkString("foo"), Value::BulkString("bar")])
            ))
        );
        assert_eq!(
            parse_message("*3\r\n:1\r\n:2\r\n:3\r\n"),
            Ok((
                "",
                Value::Array(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3)
                ])
            ))
        );
        assert_eq!(
            parse_message("*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$6\r\nfoobar\r\n"),
            Ok((
                "",
                Value::Array(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::BulkString("foobar")
                ])
            ))
        );
        assert_eq!(
            parse_message("*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Foo\r\n-Bar\r\n"),
            Ok((
                "",
                Value::Array(vec![
                    Value::Array(vec![
                        Value::Integer(1),
                        Value::Integer(2),
                        Value::Integer(3),
                    ]),
                    Value::Array(vec![Value::SimpleString("Foo"), Value::Error("Bar"),])
                ])
            ))
        );
        assert_eq!(
            parse_message("*3\r\n$3\r\nfoo\r\n$-1\r\n$3\r\nbar\r\n"),
            Ok((
                "",
                Value::Array(vec![
                    Value::BulkString("foo"),
                    Value::Null,
                    Value::BulkString("bar"),
                ])
            ))
        )
    }

    #[test]
    fn test_parse_null() {
        assert_eq!(parse_message("_\r\n"), Ok(("", Value::Null)));
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_message("#t\r\n"), Ok(("", Value::Boolean(true))));
        assert_eq!(parse_message("#f\r\n"), Ok(("", Value::Boolean(false))));
        assert!(parse_message("#x\r\n").is_err());
    }
}
