use crate::Error;

use std::fmt;

#[derive(Clone, Eq, PartialEq)]
pub struct Piece([u8; 20]);

impl Piece {
    pub fn iter(&self) -> impl Iterator<Item = &u8> {
        self.0.iter()
    }
}

impl TryFrom<&[u8]> for Piece {
    type Error = Error;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        Ok(Piece(input.try_into().map_err(|e| format!("{}", e))?))
    }
}

impl From<[u8; 20]> for Piece {
    fn from(input: [u8; 20]) -> Self {
        Piece(input)
    }
}

impl fmt::Debug for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Piece(")?;
        self.0.iter().try_for_each(|u| write!(f, "{:x}", u))?;
        write!(f, ")")?;
        Ok(())
    }
}
