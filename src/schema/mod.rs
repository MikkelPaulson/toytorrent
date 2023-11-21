pub mod peer;
pub mod tracker;

mod metainfo;

use std::borrow::Cow;
use std::fmt;

use tide::prelude::Deserialize;

pub type Error = Cow<'static, str>;

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(from = "[u8; 20]")]
pub struct InfoHash([u8; 20]);

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(from = "[u8; 20]")]
pub struct PeerId([u8; 20]);

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

impl TryFrom<&[u8]> for PeerId {
    type Error = Error;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        Ok(PeerId(input.try_into().map_err(|e| format!("{}", e))?))
    }
}

impl fmt::Debug for InfoHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "InfoHash(")?;
        self.0.iter().try_for_each(|u| write!(f, "{:x}", u))?;
        write!(f, ")")?;
        Ok(())
    }
}

impl fmt::Debug for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "PeerId({})", String::from_utf8_lossy(&self.0[..]))
    }
}
