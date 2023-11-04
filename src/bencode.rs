use std::borrow::Cow;
use std::collections::HashMap;
use std::iter;

use nom::bytes::complete as bytes;
use nom::IResult;
use nom::{branch, character, combinator, multi, sequence};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BencodeValue<'a> {
    Bytes(Cow<'a, [u8]>),
    Integer(i128),
    List(Vec<BencodeValue<'a>>),
    Dict(HashMap<Cow<'a, [u8]>, BencodeValue<'a>>),
}

impl<'a> TryFrom<&'a [u8]> for BencodeValue<'a> {
    type Error = ();

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        combinator::all_consuming(parse_once)(value)
            .map(|(_, v)| v)
            .map_err(|_| ())
    }
}

impl From<BencodeValue<'_>> for Vec<u8> {
    fn from(input: BencodeValue<'_>) -> Self {
        match input {
            BencodeValue::Bytes(b) => iter::empty()
                .chain(b.len().to_string().as_bytes())
                .chain(":".as_bytes())
                .chain(b.into_iter())
                .copied()
                .collect(),
            BencodeValue::Integer(i) => iter::empty()
                .chain("i".as_bytes())
                .chain(i.to_string().as_bytes())
                .chain("e".as_bytes())
                .copied()
                .collect(),
            BencodeValue::List(l) => iter::empty()
                .chain("l".as_bytes().into_iter().copied())
                .chain(l.into_iter().flat_map(|v| Vec::<u8>::from(v).into_iter()))
                .chain("e".as_bytes().into_iter().copied())
                .collect(),
            BencodeValue::Dict(d) => {
                let mut key_values: Vec<(Cow<'_, [u8]>, BencodeValue<'_>)> =
                    d.into_iter().collect();
                key_values.sort_by(|(a, _), (b, _)| a.cmp(b));

                iter::empty()
                    .chain("d".as_bytes().into_iter().copied())
                    .chain(key_values.into_iter().flat_map(|(k, v)| {
                        iter::empty()
                            .chain(Vec::<u8>::from(BencodeValue::Bytes(k.into())).into_iter())
                            .chain(Vec::<u8>::from(v).into_iter())
                    }))
                    .chain("e".as_bytes().into_iter().copied())
                    .collect()
            }
        }
    }
}

fn parse_once<'a>(b: &'a [u8]) -> IResult<&'a [u8], BencodeValue<'a>> {
    branch::alt((
        combinator::map(parse_bytes, |b| BencodeValue::Bytes(b.into())),
        combinator::map(parse_integer, |i| BencodeValue::Integer(i)),
        combinator::map(parse_list, |l| BencodeValue::List(l)),
        combinator::map(parse_dict, |d| BencodeValue::Dict(d)),
    ))(b)
}

fn parse_bytes<'a>(b: &'a [u8]) -> IResult<&'a [u8], &'a [u8]> {
    combinator::complete(multi::length_data(sequence::delimited(
        combinator::peek(combinator::not(sequence::pair(
            bytes::tag("0"),
            character::complete::one_of("0123456789"),
        ))),
        character::complete::u64,
        bytes::tag(":"),
    )))(b)
}

fn parse_integer<'a>(b: &'a [u8]) -> IResult<&'a [u8], i128> {
    branch::alt((
        combinator::map(bytes::tag("i0e"), |_| 0),
        sequence::delimited(
            bytes::tag("i"),
            combinator::cut(branch::alt((
                combinator::map_res(
                    sequence::preceded(
                        bytes::tag("-"),
                        sequence::preceded(
                            combinator::peek(character::complete::one_of("123456789")),
                            character::complete::u128,
                        ),
                    ),
                    |u| i128::try_from(u).map(|i| i * -1),
                ),
                combinator::map_res(
                    sequence::preceded(
                        combinator::peek(character::complete::one_of("123456789")),
                        character::complete::u128,
                    ),
                    |u| i128::try_from(u),
                ),
            ))),
            combinator::cut(bytes::tag("e")),
        ),
    ))(b)
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

fn parse_dict<'a>(b: &'a [u8]) -> IResult<&'a [u8], HashMap<Cow<'a, [u8]>, BencodeValue<'a>>> {
    combinator::map(
        sequence::preceded(
            bytes::tag("d"),
            combinator::cut(multi::many_till(
                sequence::pair(parse_bytes, parse_once),
                bytes::tag("e"),
            )),
        ),
        |(v, _)| v.into_iter().map(|(k, v)| (k.into(), v)).collect(),
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

        assert_eq!(Ok((&[][..], &[][..])), parse_bytes("0:".as_bytes()),);

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

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                "04:spam".as_bytes(),
                error::ErrorKind::Not,
            ))),
            parse_bytes("04:spam".as_bytes()),
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
                error::ErrorKind::OneOf,
            ))),
            parse_integer("i-0e".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                "00e".as_bytes(),
                error::ErrorKind::OneOf,
            ))),
            parse_integer("i00e".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                "01e".as_bytes(),
                error::ErrorKind::OneOf,
            ))),
            parse_integer("i01e".as_bytes()),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                "e".as_bytes(),
                error::ErrorKind::OneOf,
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
                error::ErrorKind::OneOf,
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
                "5:spam".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_list("5:spam".as_bytes()),
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
                "5:spam".as_bytes(),
                error::ErrorKind::Tag,
            ))),
            parse_dict("5:spam".as_bytes()),
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

    #[test]
    fn try_into_bencode_value_test_success() {
        assert_eq!(Ok(BencodeValue::Bytes(&[][..])), "0:".as_bytes().try_into());

        assert_eq!(Ok(BencodeValue::Integer(0)), "i0e".as_bytes().try_into());

        assert_eq!(
            Ok(BencodeValue::List(Vec::new())),
            "le".as_bytes().try_into(),
        );

        assert_eq!(
            Ok(BencodeValue::Dict(HashMap::new())),
            "de".as_bytes().try_into(),
        );
    }

    #[test]
    fn try_into_bencode_value_test_failure() {
        assert_eq!(Err(()), BencodeValue::try_from("0:e".as_bytes()));
    }

    #[test]
    fn from_bencode_value_test() {
        assert_eq!("i3e".as_bytes(), Vec::<u8>::from(BencodeValue::Integer(3)));

        assert_eq!(
            "i-3e".as_bytes(),
            Vec::<u8>::from(BencodeValue::Integer(-3)),
        );

        assert_eq!(
            "4:spam".as_bytes(),
            Vec::<u8>::from(BencodeValue::Bytes("spam".as_bytes())),
        );

        assert_eq!(
            "l4:spam4:eggse".as_bytes(),
            Vec::<u8>::from(BencodeValue::List(vec![
                BencodeValue::Bytes("spam".as_bytes()),
                BencodeValue::Bytes("eggs".as_bytes()),
            ])),
        );

        assert_eq!(
            "d3:cow3:moo4:spam4:eggse".as_bytes(),
            Vec::<u8>::from(BencodeValue::Dict(
                [
                    ("cow".as_bytes(), BencodeValue::Bytes("moo".as_bytes())),
                    ("spam".as_bytes(), BencodeValue::Bytes("eggs".as_bytes())),
                ]
                .into_iter()
                .collect()
            )),
        );

        assert_eq!(
            "d4:spaml1:a1:bee".as_bytes(),
            Vec::<u8>::from(BencodeValue::Dict(
                [(
                    "spam".as_bytes(),
                    BencodeValue::List(vec![
                        BencodeValue::Bytes("a".as_bytes()),
                        BencodeValue::Bytes("b".as_bytes()),
                    ]),
                )]
                .into_iter()
                .collect()
            )),
        );
    }
}
