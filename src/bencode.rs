use nom::bytes::complete as bytes;
use nom::IResult;
use nom::{character, combinator, multi, sequence};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BencodeValue<'a> {
    Bytes(&'a [u8]),
    Integer(i128),
    List(Vec<BencodeValue<'a>>),
    Dict(HashMap<&'a str, BencodeValue<'a>>),
}

fn parse_bytes<'a>(b: &'a [u8]) -> IResult<&'a [u8], &'a [u8]> {
    combinator::complete(multi::length_data(sequence::terminated(
        character::complete::u64,
        bytes::tag(":".as_bytes()),
    )))(b)
}

fn parse_integer<'a>(b: &'a [u8]) -> IResult<&'a [u8], i128> {
    sequence::delimited(
        bytes::tag("i".as_bytes()),
        sequence::preceded(
            combinator::cut(combinator::peek(combinator::not(bytes::tag(
                "-0".as_bytes(),
            )))),
            combinator::cut(character::complete::i128),
        ),
        combinator::cut(bytes::tag("e".as_bytes())),
    )(b)
}

#[cfg(test)]
mod test {
    use super::*;
    use nom::error;

    #[test]
    fn parse_bytes_test_success() {
        assert_eq!(
            Ok((&[][..], "spam".as_bytes())),
            parse_bytes("4:spam".as_bytes()),
        );

        assert_eq!(
            Ok(("m".as_bytes(), "spa".as_bytes())),
            parse_bytes("3:spam".as_bytes()),
        );
    }

    #[test]
    fn parse_bytes_test_error() {
        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                ":spam".as_bytes(),
                error::ErrorKind::Digit,
            ))),
            parse_bytes(":spam".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "i5e".as_bytes(),
                error::ErrorKind::Digit,
            ))),
            parse_bytes("i5e".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "5:spam".as_bytes(),
                error::ErrorKind::Complete,
            ))),
            parse_bytes("5:spam".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "spam".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_bytes("5spam".as_bytes()),
        );
    }

    #[test]
    fn parse_integer_test_success() {
        assert_eq!(Ok((&[][..], 0)), parse_integer("i0e".as_bytes()));

        assert_eq!(Ok((&[][..], 999)), parse_integer("i999e".as_bytes()));

        assert_eq!(Ok((&[][..], -999)), parse_integer("i-999e".as_bytes()));
    }

    #[test]
    fn parse_integer_test_error() {
        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "5:spam".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_integer("5:spam".as_bytes()),
        );
    }

    #[test]
    fn parse_integer_test_failure() {
        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                "-0e".as_bytes(),
                error::ErrorKind::Not,
            ))),
            parse_integer("i-0e".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                "e".as_bytes(),
                error::ErrorKind::Digit,
            ))),
            parse_integer("ie".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &[][..],
                error::ErrorKind::Tag,
            ))),
            parse_integer("i999999".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &[][..],
                error::ErrorKind::Digit,
            ))),
            parse_integer("i".as_bytes()),
        );
    }
}
