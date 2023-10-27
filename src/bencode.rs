use nom::bytes::complete as bytes;
use nom::IResult;
use nom::{branch, character, combinator, multi, sequence};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BencodeValue<'a> {
    Bytes(&'a [u8]),
    Integer(i128),
    List(Vec<BencodeValue<'a>>),
    Dict(HashMap<&'a [u8], BencodeValue<'a>>),
}

fn parse_once<'a>(b: &'a [u8]) -> IResult<&'a [u8], BencodeValue<'a>> {
    branch::alt((
        combinator::map(parse_bytes, |b| BencodeValue::Bytes(b)),
        combinator::map(parse_integer, |i| BencodeValue::Integer(i)),
        combinator::map(parse_list, |l| BencodeValue::List(l)),
        combinator::map(parse_dict, |d| BencodeValue::Dict(d)),
    ))(b)
}

fn parse_bytes<'a>(b: &'a [u8]) -> IResult<&'a [u8], &'a [u8]> {
    combinator::complete(multi::length_data(sequence::terminated(
        character::complete::u64,
        bytes::tag(":"),
    )))(b)
}

fn parse_integer<'a>(b: &'a [u8]) -> IResult<&'a [u8], i128> {
    sequence::delimited(
        bytes::tag("i"),
        sequence::preceded(
            combinator::cut(combinator::peek(combinator::not(bytes::tag(
                "-0".as_bytes(),
            )))),
            combinator::cut(character::complete::i128),
        ),
        combinator::cut(bytes::tag("e")),
    )(b)
}

fn parse_list<'a>(b: &'a [u8]) -> IResult<&'a [u8], Vec<BencodeValue<'a>>> {
    combinator::map(
        sequence::preceded(
            bytes::tag("l"),
            combinator::cut(multi::many_till(parse_once, bytes::tag("e"))),
        ),
        |(l, _)| l,
    )(b)
}

fn parse_dict<'a>(b: &'a [u8]) -> IResult<&'a [u8], HashMap<&'a [u8], BencodeValue<'a>>> {
    combinator::map(
        sequence::preceded(
            bytes::tag("d"),
            combinator::cut(multi::many_till(
                sequence::pair(parse_bytes, parse_once),
                bytes::tag("e"),
            )),
        ),
        |(v, _)| v.into_iter().collect(),
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
                "le".as_bytes(),
                error::ErrorKind::Digit,
            ))),
            parse_bytes("le".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "de".as_bytes(),
                error::ErrorKind::Digit,
            ))),
            parse_bytes("de".as_bytes()),
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

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "le".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_integer("le".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "de".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_integer("de".as_bytes()),
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

    #[test]
    fn parse_list_test_success() {
        assert_eq!(
            Ok((
                &[][..],
                vec![
                    BencodeValue::Bytes("spam".as_bytes()),
                    BencodeValue::Bytes("eggs".as_bytes()),
                ],
            )),
            parse_list("l4:spam4:eggse".as_bytes()),
        );

        assert_eq!(Ok((&[][..], Vec::new())), parse_list("le".as_bytes()),);

        assert_eq!(
            Ok((
                &[][..],
                vec![
                    BencodeValue::Bytes("str".as_bytes()),
                    BencodeValue::Integer(123),
                    BencodeValue::List(vec![BencodeValue::Bytes("nested".as_bytes())])
                ],
            )),
            parse_list("l3:stri123el6:nestedee".as_bytes()),
        );

        assert_eq!(
            Ok(("e".as_bytes(), Vec::new())),
            parse_list("lee".as_bytes()),
        );
    }

    #[test]
    fn parse_list_test_error() {
        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "5:blah".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_list("5:blah".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "i0e".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_list("i0e".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "de".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_list("de".as_bytes()),
        );
    }

    #[test]
    fn parse_list_test_failure() {
        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &[][..],
                error::ErrorKind::Tag,
            ))),
            parse_list("li5e".as_bytes()),
        );
    }

    #[test]
    fn parse_dict_test_success() {
        {
            let result: HashMap<_, _> = [
                ("cow".as_bytes(), BencodeValue::Bytes("moo".as_bytes())),
                ("spam".as_bytes(), BencodeValue::Bytes("eggs".as_bytes())),
            ]
            .into_iter()
            .collect();

            assert_eq!(
                Ok((&[][..], result)),
                parse_dict("d3:cow3:moo4:spam4:eggse".as_bytes()),
            );
        }

        {
            let result: HashMap<_, _> = [(
                "spam".as_bytes(),
                BencodeValue::List(vec![
                    BencodeValue::Bytes("a".as_bytes()),
                    BencodeValue::Bytes("b".as_bytes()),
                ]),
            )]
            .into_iter()
            .collect();

            assert_eq!(
                Ok((&[][..], result)),
                parse_dict("d4:spaml1:a1:bee".as_bytes()),
            );
        }

        {
            let result: HashMap<_, _> = [
                (
                    "start".as_bytes(),
                    BencodeValue::Dict(
                        [
                            ("a".as_bytes(), BencodeValue::Integer(1)),
                            ("b".as_bytes(), BencodeValue::Integer(2)),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                ),
                (
                    "end".as_bytes(),
                    BencodeValue::Dict(
                        [
                            ("y".as_bytes(), BencodeValue::Integer(25)),
                            ("z".as_bytes(), BencodeValue::Integer(26)),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                ),
            ]
            .into_iter()
            .collect();

            assert_eq!(
                Ok((&[][..], result)),
                parse_dict("d5:startd1:ai1e1:bi2ee3:endd1:yi25e1:zi26eee".as_bytes()),
            );
        }

        assert_eq!(Ok((&[][..], HashMap::new())), parse_dict("de".as_bytes()),);

        assert_eq!(
            Ok(("e".as_bytes(), HashMap::new())),
            parse_dict("dee".as_bytes()),
        );
    }

    #[test]
    fn parse_dict_test_error() {
        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "5:blah".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_dict("5:blah".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "i0e".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_dict("i0e".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "le".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_dict("le".as_bytes()),
        );
    }

    #[test]
    fn parse_dict_test_failure() {
        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &[][..],
                error::ErrorKind::Digit,
            ))),
            parse_dict("d3:key3:val".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                "i1ei2ee".as_bytes(),
                error::ErrorKind::Digit,
            ))),
            parse_dict("di1ei2ee".as_bytes()),
        );
    }
}
