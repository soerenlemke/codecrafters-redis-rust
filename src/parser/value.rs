use nom::character::complete::{i64, one_of};
use nom::{error_position, IResult};
use nom::error::{Error, ErrorKind};
use nom::bytes::complete::{tag, take_while};
use nom::combinator::{complete, eof, value};
use nom::sequence::terminated;

#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    SimpleString(&'a str),
    Error(&'a str),
    Integer(i64),
    BulkString(&'a str),
    Array(Vec<Value<'a>>),
}

pub fn parse_value(input: &str) -> IResult<&str, Value> {
    let (input, type_char) = one_of("+-:$*")(input)?;

    let parser = match type_char {
        '+' => parse_simple_string,
        '-' => parse_error,
        ':' => parse_integer,
        '$' => todo!(),
        _ => unreachable!("Invalid type char"),
    };

    terminated(parser, eof)(input)
}

fn parse_simple_string(input: &str) -> IResult<&str, Value> {
    let (input, value) = terminated(take_while(|c| c != '\r' && c != '\n'), crlf)(input)?;
    Ok((input, Value::SimpleString(value)))
}

fn parse_simple_string_raw(input: &str) -> IResult<&str, &str> {
    terminated(take_while(|c| c != '\r' && c != '\n'), crlf)(input)
}

fn parse_error(input: &str) -> IResult<&str, Value> {
    let (input, value) = parse_simple_string_raw(input)?;
    Ok((input, Value::Error(value)))
}

fn parse_integer(input: &str) -> IResult<&str, Value> {
    let (input, value) = terminated(i64, crlf)(input)?;
    Ok((input, Value::Integer(value)))
}

fn crlf(input: &str) -> IResult<&str, &str, Error<&str>> {
    tag("\r\n")(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_string() {
        assert_eq!(parse_value("+OK\r\n"), Ok(("", Value::SimpleString("OK"))));
        assert!(parse_value("+O\nK\r\n").is_err());
        assert!(parse_value("+O\nK\r\nTHIS_SHOULD_NOT_BE_HERE").is_err());
    }

    #[test]
    fn test_parse_error() {
        assert_eq!(parse_value("-Error message\r\n"), Ok(("", Value::Error("Error message"))));
        assert_eq!(parse_value("-ERR unknown command 'foobar'\r\n"), Ok(("", Value::Error("ERR unknown command 'foobar'"))));
        assert_eq!(parse_value("-WRONGTYPE Operation against a key holding the wrong kind of value\r\n"), Ok(("", Value::Error("WRONGTYPE Operation against a key holding the wrong kind of value"))));
        assert!(parse_value("-Error\nmessage\r\n").is_err());
        assert!(parse_value("-Error message\r\nTHIS_SHOULD_NOT_BE_HERE").is_err());
    }

    #[test]
    fn test_parse_integer() {
        assert_eq!(parse_value(":0\r\n"), Ok(("", Value::Integer(0))));
        assert_eq!(parse_value(":1000\r\n"), Ok(("", Value::Integer(1000))));
        assert_eq!(parse_value(":-1000\r\n"), Ok(("", Value::Integer(-1000))));
        assert!(parse_value(":-1000\n").is_err());
        assert!(parse_value(":-1000\r\nTHIS_SHOULD_NOT_BE_HERE").is_err());
    }
}