use crate::bencode::BencodeValue;
use crate::schema::Error;

use std::fmt;

#[derive(Clone, Eq, PartialEq)]
pub struct Md5Value([u8; 16]);

impl TryFrom<BencodeValue<'_>> for Md5Value {
    type Error = Error;

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let input_bytes = input.to_bytes().ok_or("`md5sum` value must be a string")?;

        if input_bytes.len() != 32 {
            return Err("`md5sum` value must be 32 bytes long".into());
        }

        Ok(Self(
            input_bytes
                .chunks_exact(2)
                .map(|slice| {
                    std::str::from_utf8(slice)
                        .ok()
                        .and_then(|s| u8::from_str_radix(s, 16).ok())
                        .ok_or("`md5sum` must be made up of valid hex characters")
                })
                .collect::<Result<Vec<u8>, _>>()?
                .try_into()
                .unwrap(),
        ))
    }
}

impl From<&Md5Value> for BencodeValue<'_> {
    fn from(input: &Md5Value) -> Self {
        input
            .0
            .iter()
            .map(|u| format!("{:x}", u).into_bytes())
            .flatten()
            .collect::<Vec<u8>>()
            .into()
    }
}

impl fmt::Debug for Md5Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Md5Value(")?;
        self.0.iter().try_for_each(|u| write!(f, "{:x}", u))?;
        write!(f, ")")?;
        Ok(())
    }
}
