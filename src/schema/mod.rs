pub mod peer;
pub mod tracker;

use crate::bencode::BencodeValue;

use sha1::{Digest, Sha1};
use tide::prelude::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "[u8; 20]")]
pub struct InfoHash([u8; 20]);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "[u8; 20]")]
pub struct PeerId([u8; 20]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetainfoFile {
    info: Info,
    announce: String,
    info_hash: InfoHash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Info {
    SingleFile {
        piece_length: u64,
        pieces: Vec<[u8; 20]>,
        name: String,
        length: u64,
    },
    MultiFile {
        piece_length: u64,
        pieces: Vec<[u8; 20]>,
        name: String,
        files: Vec<File>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct File {
    length: u64,
    path: Vec<String>,
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

impl TryFrom<&[u8]> for PeerId {
    type Error = ();

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        Ok(PeerId(input.try_into().map_err(|_| ())?))
    }
}

impl TryFrom<&[u8]> for MetainfoFile {
    type Error = ();

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        BencodeValue::try_from(input)?.try_into()
    }
}

impl TryFrom<BencodeValue<'_>> for MetainfoFile {
    type Error = ();

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input.to_dict().ok_or_else(|| ())?;

        let (Some(info_benc), Some(announce)) = (
            input_dict.remove(&b"info"[..]),
            input_dict
                .remove(&b"announce"[..])
                .and_then(BencodeValue::to_string),
        ) else {
            return Err(());
        };

        let info_hash_array: [u8; 20] = Sha1::new_with_prefix(&Vec::<u8>::from(&info_benc))
            .finalize()
            .into();

        Ok(MetainfoFile {
            info: info_benc.try_into()?,
            announce,
            info_hash: info_hash_array.into(),
        })
    }
}

impl TryFrom<BencodeValue<'_>> for Info {
    type Error = ();

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input.to_dict().ok_or_else(|| ())?;

        let (
            Some(BencodeValue::Integer(piece_length)),
            Some(BencodeValue::Bytes(pieces_bytes)),
            Some(name),
        ) = (
            input_dict.remove("piece length".as_bytes()),
            input_dict.remove("pieces".as_bytes()),
            input_dict
                .remove("name".as_bytes())
                .and_then(BencodeValue::to_string),
        )
        else {
            return Err(());
        };

        let pieces = if !pieces_bytes.is_empty() && pieces_bytes.len() % 20 == 0 {
            pieces_bytes
                .chunks_exact(20)
                .map(|a| a.try_into())
                .collect::<Result<_, _>>()
                .map_err(|_| ())
        } else {
            Err(())
        }?;

        match (
            input_dict.remove("length".as_bytes()),
            input_dict.remove("files".as_bytes()),
        ) {
            (Some(BencodeValue::Integer(length)), None) => Ok(Info::SingleFile {
                piece_length: piece_length.try_into().map_err(|_| ())?,
                pieces,
                name,
                length: length.try_into().map_err(|_| ())?,
            }),
            (None, Some(BencodeValue::List(files))) => Ok(Info::MultiFile {
                piece_length: piece_length.try_into().map_err(|_| ())?,
                pieces,
                name,
                files: files
                    .into_iter()
                    .map(|file| file.try_into())
                    .collect::<Result<_, _>>()?,
            }),
            _ => Err(()),
        }
    }
}

impl TryFrom<BencodeValue<'_>> for File {
    type Error = ();

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input.to_dict().ok_or_else(|| ())?;

        let (Some(BencodeValue::Integer(length)), Some(BencodeValue::List(path_list))) = (
            input_dict.remove("length".as_bytes()),
            input_dict.remove("path".as_bytes()),
        ) else {
            return Err(());
        };

        let path = path_list
            .into_iter()
            .map(|benc| benc.to_string().ok_or(()))
            .collect::<Result<_, _>>()?;

        Ok(File {
            length: length.try_into().map_err(|_| ())?,
            path,
        })
    }
}
