pub mod peer;
pub mod tracker;

mod metainfo;

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use tide::prelude::Deserialize;

pub type Error = Cow<'static, str>;

#[derive(Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(from = "[u8; 20]")]
pub struct InfoHash([u8; 20]);

#[derive(Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(from = "[u8; 20]")]
pub struct PeerId([u8; 20]);

#[derive(Clone, Deserialize, Eq, Hash, PartialEq)]
#[serde(from = "&[u8]")]
pub struct PeerKey(Vec<u8>);

pub struct Bytes(u64);

impl FromStr for InfoHash {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(InfoHash(parse_qs_to_bytes(input)?))
    }
}

impl FromStr for PeerId {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(PeerId(parse_qs_to_bytes(input)?))
    }
}

impl From<[u8; 20]> for InfoHash {
    fn from(input: [u8; 20]) -> Self {
        Self(input)
    }
}

impl From<[u8; 20]> for PeerId {
    fn from(input: [u8; 20]) -> Self {
        Self(input)
    }
}

impl From<&[u8]> for PeerKey {
    fn from(input: &[u8]) -> Self {
        Self(input.to_vec())
    }
}

impl TryFrom<&[u8]> for PeerId {
    type Error = Error;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        Ok(PeerId(input.try_into().map_err(|e| format!("{}", e))?))
    }
}

impl fmt::Debug for InfoHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "InfoHash({})", self)
    }
}

impl fmt::Display for InfoHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.iter().try_for_each(|u| write!(f, "{:x}", u))
    }
}

impl fmt::Debug for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "PeerId({})", self)
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", String::from_utf8_lossy(&self.0[..]))
    }
}

impl fmt::Debug for PeerKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "PeerKey({})", self)
    }
}

impl fmt::Display for PeerKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", String::from_utf8_lossy(&self.0[..]))
    }
}

fn parse_qs_to_bytes<const N: usize, T: From<[u8; N]>>(input: &str) -> Result<T, &'static str> {
    let mut input_iter = input.chars();
    let mut result_arr = [0u8; N];

    for i in 0.. {
        let c = input_iter.next();
        if i >= N {
            if c.is_some() {
                return Err("Too many characters");
            } else {
                break;
            }
        }

        match c {
            Some('%') => {
                let [Some(a), Some(b)] = [
                    input_iter.next().and_then(|c| c.to_digit(16)),
                    input_iter.next().and_then(|c| c.to_digit(16)),
                ] else {
                    return Err("Expected % to be followed by two hex characters");
                };

                result_arr[i] = (a * 16 + b).try_into().unwrap();
            }
            Some(c) if c >= '\0' && c <= '\u{7f}' => result_arr[i] = c.try_into().unwrap(),
            Some(_) => return Err("Unexpected non-ASCII character"),
            None => return Err("Too few characters"),
        }
    }

    Ok(result_arr.into())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn peerid_hash_test() {
        let mut set: HashSet<PeerId> = HashSet::new();

        assert_eq!(true, set.insert([0; 20].into()));
        assert_eq!(false, set.insert([0; 20].into()));
        assert_eq!(1, set.len());
    }
}
