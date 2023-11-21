use std::borrow::Cow;
use std::collections::HashMap;
use std::iter;
use std::time::{Duration, SystemTime};

use crate::schema::Error;

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

impl<'a> BencodeValue<'a> {
    pub fn decode(input: &'a [u8]) -> Result<Self, Error> {
        combinator::all_consuming(parse_once)(input)
            .map(|(_, v)| v)
            .map_err(|e| format!("{}", e).into())
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            BencodeValue::Bytes(b) => iter::empty()
                .chain(b.len().to_string().as_bytes())
                .chain(b":")
                .chain(b.into_iter())
                .copied()
                .collect(),
            BencodeValue::Integer(i) => iter::empty()
                .chain(b"i")
                .chain(i.to_string().as_bytes())
                .chain(b"e")
                .copied()
                .collect(),
            BencodeValue::List(l) => iter::empty()
                .chain(b"l".into_iter().copied())
                .chain(l.into_iter().flat_map(|v| v.encode().into_iter()))
                .chain(b"e".into_iter().copied())
                .collect(),
            BencodeValue::Dict(d) => {
                let mut key_values: Vec<(&Cow<'_, [u8]>, &BencodeValue<'_>)> = d.iter().collect();
                key_values.sort_by(|(a, _), (b, _)| a.cmp(b));

                iter::empty()
                    .chain(b"d".into_iter().copied())
                    .chain(key_values.into_iter().flat_map(|(k, v)| {
                        iter::empty()
                            .chain(BencodeValue::Bytes(k.clone()).encode().into_iter())
                            .chain(v.encode().into_iter())
                    }))
                    .chain(b"e".into_iter().copied())
                    .collect()
            }
        }
    }

    pub fn to_string(self) -> Option<String> {
        self.to_bytes()
            .map(|bytes| String::from_utf8_lossy(&bytes).into())
    }

    pub fn to_bytes(self) -> Option<Cow<'a, [u8]>> {
        if let Self::Bytes(bytes) = self {
            Some(bytes)
        } else {
            None
        }
    }

    pub fn to_i128(self) -> Option<i128> {
        if let Self::Integer(integer) = self {
            Some(integer)
        } else {
            None
        }
    }

    pub fn to_u64(self) -> Option<u64> {
        self.to_i128().and_then(|i| i.try_into().ok())
    }

    pub fn to_list(self) -> Option<Vec<BencodeValue<'a>>> {
        if let Self::List(list) = self {
            Some(list)
        } else {
            None
        }
    }

    pub fn to_dict(self) -> Option<HashMap<Cow<'a, [u8]>, BencodeValue<'a>>> {
        if let Self::Dict(dict) = self {
            Some(dict)
        } else {
            None
        }
    }

    pub fn to_time(self) -> Option<SystemTime> {
        self.to_i128().and_then(|i| {
            if i.is_negative() {
                (i * -1)
                    .try_into()
                    .map(|u| SystemTime::UNIX_EPOCH - Duration::from_secs(u))
            } else {
                i.try_into()
                    .map(|u| SystemTime::UNIX_EPOCH + Duration::from_secs(u))
            }
            .ok()
        })
    }
}

impl<'a> From<&'a [u8]> for BencodeValue<'a> {
    fn from(input: &'a [u8]) -> Self {
        BencodeValue::Bytes(input.into())
    }
}

impl<'a> From<Vec<u8>> for BencodeValue<'a> {
    fn from(input: Vec<u8>) -> Self {
        BencodeValue::Bytes(input.into())
    }
}

impl<'a> From<&'a str> for BencodeValue<'a> {
    fn from(input: &'a str) -> Self {
        input.as_bytes().into()
    }
}

impl From<String> for BencodeValue<'_> {
    fn from(input: String) -> Self {
        input.into_bytes().into()
    }
}

impl From<&SystemTime> for BencodeValue<'_> {
    fn from(input: &SystemTime) -> Self {
        BencodeValue::Integer(
            input
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|u| u.as_secs().into())
                .unwrap_or_else(|e| i128::from(e.duration().as_secs()) * -1),
        )
    }
}

impl From<i128> for BencodeValue<'_> {
    fn from(input: i128) -> Self {
        BencodeValue::Integer(input)
    }
}

impl From<u64> for BencodeValue<'_> {
    fn from(input: u64) -> Self {
        i128::from(input).into()
    }
}

impl<'a> From<Vec<BencodeValue<'a>>> for BencodeValue<'a> {
    fn from(input: Vec<BencodeValue<'a>>) -> Self {
        BencodeValue::List(input)
    }
}

impl<'a> iter::FromIterator<(&'a str, BencodeValue<'a>)> for BencodeValue<'a> {
    fn from_iter<I: IntoIterator<Item = (&'a str, BencodeValue<'a>)>>(iter: I) -> Self {
        BencodeValue::Dict(
            iter.into_iter()
                .map(|(k, v)| (k.as_bytes().into(), v))
                .collect(),
        )
    }
}

impl<'a> iter::FromIterator<BencodeValue<'a>> for BencodeValue<'a> {
    fn from_iter<I: IntoIterator<Item = BencodeValue<'a>>>(iter: I) -> Self {
        BencodeValue::List(iter.into_iter().collect())
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
        assert_eq!(Ok((&[][..], &b"spam"[..])), parse_bytes(&b"4:spam"[..]),);

        assert_eq!(Ok((&[][..], &[][..])), parse_bytes(&b"0:"[..]),);

        assert_eq!(Ok((&b"m"[..], &b"spa"[..])), parse_bytes(&b"3:spam"[..]),);
    }

    #[test]
    fn parse_bytes_test_error() {
        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b":spam"[..],
                error::ErrorKind::Digit,
            ))),
            parse_bytes(&b":spam"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"i5e"[..],
                error::ErrorKind::Digit,
            ))),
            parse_bytes(&b"i5e"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"le"[..],
                error::ErrorKind::Digit,
            ))),
            parse_bytes(&b"le"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"de"[..],
                error::ErrorKind::Digit,
            ))),
            parse_bytes(&b"de"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"5:spam"[..],
                error::ErrorKind::Complete,
            ))),
            parse_bytes(&b"5:spam"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"spam"[..],
                error::ErrorKind::Tag,
            ))),
            parse_bytes(&b"5spam"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"04:spam"[..],
                error::ErrorKind::Not,
            ))),
            parse_bytes(&b"04:spam"[..]),
        );
    }

    #[test]
    fn parse_integer_test_success() {
        assert_eq!(Ok((&[][..], 0)), parse_integer(&b"i0e"[..]));

        assert_eq!(Ok((&[][..], 999)), parse_integer(&b"i999e"[..]));

        assert_eq!(Ok((&[][..], -999)), parse_integer(&b"i-999e"[..]));
    }

    #[test]
    fn parse_integer_test_error() {
        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"5:spam"[..],
                error::ErrorKind::Tag,
            ))),
            parse_integer(&b"5:spam"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"le"[..],
                error::ErrorKind::Tag,
            ))),
            parse_integer(&b"le"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"de"[..],
                error::ErrorKind::Tag,
            ))),
            parse_integer(&b"de"[..]),
        );
    }

    #[test]
    fn parse_integer_test_failure() {
        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &b"-0e"[..],
                error::ErrorKind::OneOf,
            ))),
            parse_integer(&b"i-0e"[..]),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &b"00e"[..],
                error::ErrorKind::OneOf,
            ))),
            parse_integer(&b"i00e"[..]),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &b"01e"[..],
                error::ErrorKind::OneOf,
            ))),
            parse_integer(&b"i01e"[..]),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &b"e"[..],
                error::ErrorKind::OneOf,
            ))),
            parse_integer(&b"ie"[..]),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &[][..],
                error::ErrorKind::Tag,
            ))),
            parse_integer(&b"i999999"[..]),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &[][..],
                error::ErrorKind::OneOf,
            ))),
            parse_integer(b"i"),
        );
    }

    #[test]
    fn parse_list_test_success() {
        assert_eq!(
            Ok((
                &[][..],
                vec![BencodeValue::from("spam"), BencodeValue::from("eggs")],
            )),
            parse_list(&b"l4:spam4:eggse"[..]),
        );

        assert_eq!(Ok((&[][..], Vec::new())), parse_list(&b"le"[..]));

        assert_eq!(
            Ok((
                &[][..],
                vec![
                    BencodeValue::from("str"),
                    BencodeValue::from(123u64),
                    BencodeValue::from(vec![BencodeValue::from("nested")])
                ],
            )),
            parse_list(&b"l3:stri123el6:nestedee"[..]),
        );

        assert_eq!(Ok((&b"e"[..], Vec::new())), parse_list(&b"lee"[..]),);
    }

    #[test]
    fn parse_list_test_error() {
        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"5:spam"[..],
                error::ErrorKind::Tag,
            ))),
            parse_list(&b"5:spam"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"i0e"[..],
                error::ErrorKind::Tag,
            ))),
            parse_list(&b"i0e"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"de"[..],
                error::ErrorKind::Tag,
            ))),
            parse_list(&b"de"[..]),
        );
    }

    #[test]
    fn parse_list_test_failure() {
        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &[][..],
                error::ErrorKind::Tag,
            ))),
            parse_list(&b"li5e"[..]),
        );
    }

    #[test]
    fn parse_dict_test_success() {
        {
            let result: HashMap<_, _> = [
                (b"cow"[..].into(), BencodeValue::from("moo")),
                (b"spam"[..].into(), BencodeValue::from("eggs")),
            ]
            .into_iter()
            .collect();

            assert_eq!(
                Ok((&[][..], result)),
                parse_dict(&b"d3:cow3:moo4:spam4:eggse"[..]),
            );
        }

        {
            let result: HashMap<_, _> = [(
                b"spam"[..].into(),
                BencodeValue::from(vec![BencodeValue::from("a"), BencodeValue::from("b")]),
            )]
            .into_iter()
            .collect();

            assert_eq!(Ok((&[][..], result)), parse_dict(&b"d4:spaml1:a1:bee"[..]),);
        }

        {
            let result: HashMap<_, _> = [
                (
                    b"start"[..].into(),
                    [
                        ("a", BencodeValue::Integer(1)),
                        ("b", BencodeValue::Integer(2)),
                    ]
                    .into_iter()
                    .collect::<BencodeValue<'_>>(),
                ),
                (
                    b"end"[..].into(),
                    [
                        ("y", BencodeValue::Integer(25)),
                        ("z", BencodeValue::Integer(26)),
                    ]
                    .into_iter()
                    .collect::<BencodeValue<'_>>(),
                ),
            ]
            .into_iter()
            .collect();

            assert_eq!(
                Ok((&[][..], result)),
                parse_dict(&b"d5:startd1:ai1e1:bi2ee3:endd1:yi25e1:zi26eee"[..]),
            );
        }

        assert_eq!(Ok((&[][..], HashMap::new())), parse_dict(&b"de"[..]),);

        assert_eq!(Ok((&b"e"[..], HashMap::new())), parse_dict(&b"dee"[..]));
    }

    #[test]
    fn parse_dict_test_error() {
        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"5:spam"[..],
                error::ErrorKind::Tag,
            ))),
            parse_dict(&b"5:spam"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"i0e"[..],
                error::ErrorKind::Tag,
            ))),
            parse_dict(&b"i0e"[..]),
        );

        assert_eq!(
            Err(nom::Err::Error(error::Error::new(
                &b"le"[..],
                error::ErrorKind::Tag,
            ))),
            parse_dict(&b"le"[..]),
        );
    }

    #[test]
    fn parse_dict_test_failure() {
        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &[][..],
                error::ErrorKind::Digit,
            ))),
            parse_dict(&b"d3:key3:val"[..]),
        );

        assert_eq!(
            Err(nom::Err::Failure(error::Error::new(
                &b"i1ei2ee"[..],
                error::ErrorKind::Digit,
            ))),
            parse_dict(&b"di1ei2ee"[..]),
        );
    }

    #[test]
    fn decode_test_success() {
        assert_eq!(
            Ok(BencodeValue::Bytes([][..].into())),
            BencodeValue::decode(&b"0:"[..]),
        );

        assert_eq!(
            Ok(BencodeValue::Integer(0)),
            BencodeValue::decode(&b"i0e"[..]),
        );

        assert_eq!(
            Ok(BencodeValue::List(Vec::new())),
            BencodeValue::decode(&b"le"[..]),
        );

        assert_eq!(
            Ok(BencodeValue::Dict(HashMap::new())),
            BencodeValue::decode(&b"de"[..]),
        );
    }

    #[test]
    fn decode_test_failure() {
        assert_eq!(
            Err("Parsing Error: Error { input: [101], code: Eof }".into()),
            BencodeValue::decode(&b"0:e"[..]),
        );
    }

    #[test]
    fn encode_test() {
        assert_eq!(&b"i3e"[..], BencodeValue::from(3u64).encode());

        assert_eq!(&b"i-3e"[..], BencodeValue::from(-3i128).encode());

        assert_eq!(&b"4:spam"[..], BencodeValue::from("spam").encode());

        assert_eq!(
            &b"l4:spam4:eggse"[..],
            BencodeValue::from(vec![BencodeValue::from("spam"), BencodeValue::from("eggs")])
                .encode(),
        );

        assert_eq!(
            &b"d3:cow3:moo4:spam4:eggse"[..],
            [
                ("cow", BencodeValue::from("moo")),
                ("spam", BencodeValue::from("eggs")),
            ]
            .into_iter()
            .collect::<BencodeValue<'_>>()
            .encode(),
        );

        assert_eq!(
            &b"d4:spaml1:a1:bee"[..],
            [(
                "spam",
                BencodeValue::from(vec![BencodeValue::from("a"), BencodeValue::from("b")]),
            )]
            .into_iter()
            .collect::<BencodeValue<'_>>()
            .encode(),
        );
    }
}
